use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use dashmap::DashMap;
use serde::Serialize;

use crate::config::Settings;

#[derive(Clone)]
pub struct RateLimitState {
    buckets: Arc<DashMap<String, TokenBucket>>,
    requests_per_minute: u32,
    burst_size: u32,
    max_entries: usize,
}

struct TokenBucket {
    tokens: f64,
    last_update: Instant,
    last_access: Instant,
    max_tokens: f64,
    refill_rate: f64,
}

impl TokenBucket {
    fn new(max_tokens: u32, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens as f64,
            last_update: Instant::now(),
            last_access: Instant::now(),
            max_tokens: max_tokens as f64,
            refill_rate,
        }
    }

    fn take(&mut self) -> bool {
        self.touch();
        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_update = now;
    }

    fn touch(&mut self) {
        self.last_access = Instant::now();
    }

    fn tokens_remaining(&self) -> u32 {
        self.tokens as u32
    }

    fn reset_after(&self) -> Duration {
        let tokens_needed = 1.0 - self.tokens;
        if tokens_needed <= 0.0 {
            Duration::ZERO
        } else {
            Duration::from_secs_f64(tokens_needed / self.refill_rate)
        }
    }
}

impl RateLimitState {
    pub fn new(settings: &Settings) -> Self {
        let state = Self {
            buckets: Arc::new(DashMap::new()),
            requests_per_minute: settings.rate_limit.requests_per_minute,
            burst_size: settings.rate_limit.burst_size,
            max_entries: settings.rate_limit.max_entries as usize,
        };

        let buckets = state.buckets.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300));
            loop {
                interval.tick().await;
                let threshold = Instant::now() - Duration::from_secs(3600);
                buckets.retain(|_, bucket| bucket.last_access > threshold);
            }
        });

        state
    }

    fn get_bucket(&self, key: &str) -> bool {
        let refill_rate = self.requests_per_minute as f64 / 60.0;

        let allowed = {
            let mut entry = self
                .buckets
                .entry(key.to_string())
                .or_insert_with(|| TokenBucket::new(self.burst_size, refill_rate));

            entry.take()
        };
        self.evict_lru_if_needed();
        allowed
    }

    fn get_bucket_info(&self, key: &str) -> (u32, Duration) {
        if let Some(mut entry) = self.buckets.get_mut(key) {
            entry.touch();
            (entry.tokens_remaining(), entry.reset_after())
        } else {
            (self.burst_size, Duration::ZERO)
        }
    }

    fn get_bucket_with_limits(&self, key: &str, requests_per_minute: u32, burst: u32) -> bool {
        let refill_rate = requests_per_minute as f64 / 60.0;

        let allowed = {
            let mut entry = self
                .buckets
                .entry(key.to_string())
                .or_insert_with(|| TokenBucket::new(burst, refill_rate));

            entry.take()
        };
        self.evict_lru_if_needed();
        allowed
    }

    fn get_bucket_info_with_limits(&self, key: &str, burst: u32) -> (u32, Duration) {
        if let Some(mut entry) = self.buckets.get_mut(key) {
            entry.touch();
            (entry.tokens_remaining(), entry.reset_after())
        } else {
            (burst, Duration::ZERO)
        }
    }

    fn evict_lru_if_needed(&self) {
        if self.max_entries == 0 {
            return;
        }

        while self.buckets.len() > self.max_entries {
            let mut oldest_key: Option<String> = None;
            let mut oldest_access: Option<Instant> = None;

            for entry in self.buckets.iter() {
                let access = entry.value().last_access;
                let replace = match oldest_access {
                    Some(current) => access < current,
                    None => true,
                };

                if replace {
                    oldest_access = Some(access);
                    oldest_key = Some(entry.key().clone());
                }
            }

            if let Some(key) = oldest_key {
                self.buckets.remove(&key);
            } else {
                break;
            }
        }
    }
}

#[derive(Serialize)]
struct RateLimitError {
    error: RateLimitErrorDetail,
}

#[derive(Serialize)]
struct RateLimitErrorDetail {
    code: String,
    message: String,
    retry_after_seconds: u64,
}

pub async fn rate_limit_middleware(
    State(state): State<RateLimitState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    let key = addr.ip().to_string();

    if !state.get_bucket(&key) {
        let (_, reset_after) = state.get_bucket_info(&key);

        let error = RateLimitError {
            error: RateLimitErrorDetail {
                code: "RATE_LIMIT_EXCEEDED".to_string(),
                message: "Too many requests. Please try again later.".to_string(),
                retry_after_seconds: reset_after.as_secs(),
            },
        };

        return (
            StatusCode::TOO_MANY_REQUESTS,
            [
                ("Retry-After", reset_after.as_secs().to_string()),
                ("X-RateLimit-Remaining", "0".to_string()),
            ],
            Json(error),
        )
            .into_response();
    }

    let (remaining, _) = state.get_bucket_info(&key);

    let mut response = next.run(request).await;

    response.headers_mut().insert(
        "X-RateLimit-Limit",
        state.requests_per_minute.to_string().parse().unwrap(),
    );
    response.headers_mut().insert(
        "X-RateLimit-Remaining",
        remaining.to_string().parse().unwrap(),
    );

    response
}

pub async fn auth_rate_limit_middleware(
    State(state): State<RateLimitState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    let ip = addr.ip().to_string();
    let ip_key = format!("auth:ip:{}", ip);

    let requests_per_minute = 10;
    let burst = 5;

    if !state.get_bucket_with_limits(&ip_key, requests_per_minute, burst) {
        let (_, reset_after) = state.get_bucket_info_with_limits(&ip_key, burst);

        tracing::warn!(
            ip = %ip,
            "Auth rate limit exceeded for IP"
        );

        let error = RateLimitError {
            error: RateLimitErrorDetail {
                code: "RATE_LIMIT_EXCEEDED".to_string(),
                message: "Too many authentication attempts. Please try again later.".to_string(),
                retry_after_seconds: reset_after.as_secs().max(60),
            },
        };

        return (
            StatusCode::TOO_MANY_REQUESTS,
            [("Retry-After", reset_after.as_secs().max(60).to_string())],
            Json(error),
        )
            .into_response();
    }

    next.run(request).await
}
