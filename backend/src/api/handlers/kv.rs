use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap},
    Extension, Json,
};

use crate::api::extractors::AuthUser;
use crate::api::handlers::DatabaseServiceState;
use crate::domain::models::{
    AuditAction, AuditEntityType, AuditStatus, ExecuteKvCommandRequest, KvCommandResult,
};
use crate::domain::services::AuditLogService;
use crate::error::AppResult;

/// Extracts the client IP address from the `X-Forwarded-For` header.
///
/// Reads the `x-forwarded-for` header and returns the first comma-separated
/// entry (trimmed) if present and valid.
///
/// # Examples
///
/// ```
/// use http::HeaderMap;
/// use http::HeaderValue;
///
/// let mut headers = HeaderMap::new();
/// headers.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.5, 198.51.100.1"));
/// assert_eq!(get_client_ip(&headers), Some("203.0.113.5".to_string()));
/// ```
fn get_client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
}

/// Extracts the `User-Agent` header value from the provided HTTP headers.
///
/// Returns `Some(String)` with the header's string value, `None` if the header is
/// missing or cannot be converted to valid UTF-8.
///
/// # Examples
///
/// ```
/// use http::{HeaderMap, HeaderValue, header};
/// let mut headers = HeaderMap::new();
/// headers.insert(header::USER_AGENT, HeaderValue::from_static("req/1.0"));
/// assert_eq!(crate::get_user_agent(&headers), Some("req/1.0".to_string()));
/// ```
fn get_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Execute a key-value command against the specified database and record a success audit entry.
///
/// On successful execution, returns the database command result and logs an audit entry containing
/// the actor, action, target database id, command details, client IP, and user agent.
///
/// # Examples
///
/// ```
/// // Pseudocode example — replace with actual service state, audit service, and auth user in tests.
/// # async fn example() {
/// use axum::Json;
/// let db_id = "my-db".to_string();
/// let payload = ExecuteKvCommandRequest { command: "GET key".to_string(), timeout_ms: None };
/// let response: AppResult<Json<KvCommandResult>> = execute_kv_command(
///     State(database_service),
///     Extension(audit_service),
///     HeaderMap::new(),
///     auth_user,
///     Path(db_id),
///     Json(payload),
/// ).await;
/// # }
/// ```
#[utoipa::path(
post,
path = "/api/v1/databases/{id}/kv",
params(
("id" = String, Path, description = "Database ID")
),
request_body = ExecuteKvCommandRequest,
responses(
(status = 200, description = "Command executed successfully", body = KvCommandResult),
(status = 400, description = "Command validation failed or database not running"),
(status = 401, description = "Unauthorized"),
(status = 403, description = "Forbidden - no access to database"),
(status = 404, description = "Database not found"),
(status = 409, description = "Database type mismatch or container not running")
),
tag = "Key-Value",
security(("bearer" = []))
)]
pub async fn execute_kv_command(
    State(database_service): State<DatabaseServiceState>,
    Extension(audit_service): Extension<Arc<AuditLogService>>,
    headers: HeaderMap,
    auth_user: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<ExecuteKvCommandRequest>,
) -> AppResult<Json<KvCommandResult>> {
    let result = database_service
        .execute_kv_command(&id, auth_user.id(), &payload.command, payload.timeout_ms)
        .await?;

    audit_service.log(
        auth_user.id().to_string(),
        AuditAction::ExecuteQuery,
        AuditEntityType::Query,
        Some(id),
        Some(serde_json::json!({
            "command": payload.command,
            "timeout_ms": payload.timeout_ms,
        })),
        AuditStatus::Success,
        get_client_ip(&headers),
        get_user_agent(&headers),
    );

    Ok(Json(result))
}