use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use sqlx::sqlite::SqlitePool;
use sysinfo::System;
use utoipa::ToSchema;

use crate::infrastructure::docker::DockerManager;

#[derive(Clone)]
pub struct HealthState {
    pub db_pool: SqlitePool,
    pub docker: Arc<DockerManager>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReadyResponse {
    pub status: String,
    pub checks: HealthChecks,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthChecks {
    pub database: CheckStatus,
    pub docker: CheckStatus,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CheckStatus {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl CheckStatus {
    fn ok() -> Self {
        Self {
            status: "ok".to_string(),
            message: None,
        }
    }

    fn error(message: &str) -> Self {
        Self {
            status: "error".to_string(),
            message: Some(message.to_string()),
        }
    }
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    ),
    tag = "Health"
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[utoipa::path(
    get,
    path = "/ready",
    responses(
        (status = 200, description = "All services are ready", body = ReadyResponse),
        (status = 503, description = "One or more services are not ready", body = ReadyResponse)
    ),
    tag = "Health"
)]
pub async fn ready(State(state): State<HealthState>) -> (StatusCode, Json<ReadyResponse>) {
    let mut all_ok = true;

    let database = match sqlx::query("SELECT 1").fetch_one(&state.db_pool).await {
        Ok(_) => CheckStatus::ok(),
        Err(e) => {
            all_ok = false;
            CheckStatus::error(&format!("Database check failed: {}", e))
        },
    };

    let docker = match state.docker.list_containers(None).await {
        Ok(_) => CheckStatus::ok(),
        Err(e) => {
            all_ok = false;
            CheckStatus::error(&format!("Docker check failed: {}", e))
        },
    };

    let status = if all_ok { "ready" } else { "degraded" };
    let status_code = if all_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        Json(ReadyResponse {
            status: status.to_string(),
            checks: HealthChecks { database, docker },
        }),
    )
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SystemInfoResponse {
    pub cpu_cores: usize,
    pub total_memory_mb: u64,
}

#[utoipa::path(
    get,
    path = "/system",
    responses(
        (status = 200, description = "System information", body = SystemInfoResponse)
    ),
    tag = "Health"
)]
pub async fn system_info() -> Json<SystemInfoResponse> {
    let sys = System::new_all();
    let cpu_cores = sys.cpus().len();
    let total_memory_mb = sys.total_memory() / (1024 * 1024);

    Json(SystemInfoResponse {
        cpu_cores,
        total_memory_mb,
    })
}
