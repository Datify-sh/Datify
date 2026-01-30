use std::time::{Duration, Instant};

use axum::Json;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use utoipa::ToSchema;

const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const DOCKER_HUB_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PostgresVersionInfo {
    pub version: String,
    pub tag: String,
    pub is_latest: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PostgresVersionsResponse {
    pub versions: Vec<PostgresVersionInfo>,
    pub default_version: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ValkeyVersionInfo {
    pub version: String,
    pub tag: String,
    pub is_latest: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ValkeyVersionsResponse {
    pub versions: Vec<ValkeyVersionInfo>,
    pub default_version: String,
}

#[derive(Debug, Deserialize)]
struct DockerHubResponse {
    results: Vec<DockerHubTag>,
    next: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DockerHubTag {
    name: String,
}

struct VersionCache<T> {
    versions: Vec<T>,
    fetched_at: Instant,
}

static POSTGRES_VERSION_CACHE: Lazy<RwLock<Option<VersionCache<PostgresVersionInfo>>>> =
    Lazy::new(|| RwLock::new(None));

static VALKEY_VERSION_CACHE: Lazy<RwLock<Option<VersionCache<ValkeyVersionInfo>>>> =
    Lazy::new(|| RwLock::new(None));

async fn fetch_docker_hub_tags(repo: &str, pages: usize) -> Result<Vec<String>, reqwest::Error> {
    let client = reqwest::Client::builder()
        .timeout(DOCKER_HUB_TIMEOUT)
        .build()?;

    let mut all_tags = Vec::new();
    let mut url = format!(
        "https://hub.docker.com/v2/repositories/{}/tags?page_size=100",
        repo
    );

    for _ in 0..pages {
        let resp: DockerHubResponse = client.get(&url).send().await?.json().await?;
        all_tags.extend(resp.results.into_iter().map(|t| t.name));

        match resp.next {
            Some(next_url) => url = next_url,
            None => break,
        }
    }

    Ok(all_tags)
}

async fn fetch_postgres_versions() -> Result<Vec<PostgresVersionInfo>, reqwest::Error> {
    let all_tags = fetch_docker_hub_tags("library/postgres", 3).await?;

    let mut major_versions: Vec<u32> = all_tags
        .iter()
        .filter_map(|tag| tag.parse::<u32>().ok())
        .filter(|&v| (13..=25).contains(&v))
        .collect();

    major_versions.sort();
    major_versions.dedup();

    let latest_version = major_versions.last().copied();

    let versions = major_versions
        .into_iter()
        .map(|v| PostgresVersionInfo {
            version: v.to_string(),
            tag: format!("postgres:{}-alpine", v),
            is_latest: Some(v) == latest_version,
        })
        .collect();

    Ok(versions)
}

async fn fetch_valkey_versions() -> Result<Vec<ValkeyVersionInfo>, reqwest::Error> {
    let all_tags = fetch_docker_hub_tags("valkey/valkey", 3).await?;

    let mut versions: Vec<(u32, u32)> = all_tags
        .iter()
        .filter(|tag| tag.ends_with("-alpine"))
        .filter_map(|tag| {
            let version_part = tag.trim_end_matches("-alpine");
            let parts: Vec<&str> = version_part.split('.').collect();
            if parts.len() >= 2 {
                let major = parts[0].parse::<u32>().ok()?;
                let minor = parts[1].parse::<u32>().ok()?;
                Some((major, minor))
            } else {
                None
            }
        })
        .collect();

    versions.sort();
    versions.dedup();

    let latest_version = versions.last().copied();

    let versions = versions
        .into_iter()
        .map(|(major, minor)| {
            let version = format!("{}.{}", major, minor);
            ValkeyVersionInfo {
                tag: format!("valkey/valkey:{}-alpine", version),
                is_latest: Some((major, minor)) == latest_version,
                version,
            }
        })
        .collect();

    Ok(versions)
}

#[utoipa::path(
    get,
    path = "/system/postgres-versions",
    responses(
        (status = 200, description = "Available PostgreSQL versions", body = PostgresVersionsResponse)
    ),
    tag = "System"
)]
pub async fn get_postgres_versions() -> Json<PostgresVersionsResponse> {
    {
        let cache = POSTGRES_VERSION_CACHE.read().await;
        if let Some(ref cached) = *cache {
            if cached.fetched_at.elapsed() < CACHE_TTL {
                return Json(PostgresVersionsResponse {
                    versions: cached.versions.clone(),
                    default_version: "16".to_string(),
                });
            }
        }
    }

    let versions = match fetch_postgres_versions().await {
        Ok(v) => {
            let mut cache = POSTGRES_VERSION_CACHE.write().await;
            *cache = Some(VersionCache {
                versions: v.clone(),
                fetched_at: Instant::now(),
            });
            v
        },
        Err(e) => {
            tracing::warn!("Failed to fetch PostgreSQL versions from Docker Hub: {}", e);
            vec![]
        },
    };

    Json(PostgresVersionsResponse {
        versions,
        default_version: "16".to_string(),
    })
}

#[utoipa::path(
    get,
    path = "/system/valkey-versions",
    responses(
        (status = 200, description = "Available Valkey versions", body = ValkeyVersionsResponse)
    ),
    tag = "System"
)]
pub async fn get_valkey_versions() -> Json<ValkeyVersionsResponse> {
    {
        let cache = VALKEY_VERSION_CACHE.read().await;
        if let Some(ref cached) = *cache {
            if cached.fetched_at.elapsed() < CACHE_TTL {
                return Json(ValkeyVersionsResponse {
                    versions: cached.versions.clone(),
                    default_version: "8.0".to_string(),
                });
            }
        }
    }

    let versions = match fetch_valkey_versions().await {
        Ok(v) => {
            let mut cache = VALKEY_VERSION_CACHE.write().await;
            *cache = Some(VersionCache {
                versions: v.clone(),
                fetched_at: Instant::now(),
            });
            v
        },
        Err(e) => {
            tracing::warn!("Failed to fetch Valkey versions from Docker Hub: {}", e);
            vec![]
        },
    };

    Json(ValkeyVersionsResponse {
        versions,
        default_version: "8.0".to_string(),
    })
}
