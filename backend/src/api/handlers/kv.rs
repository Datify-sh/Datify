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

fn get_client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
}

fn get_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn summarize_kv_for_audit(command: &str, timeout_ms: Option<i32>) -> serde_json::Value {
    let cmd = command
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_uppercase();

    serde_json::json!({
        "command": cmd,
        "length": command.len(),
        "timeout_ms": timeout_ms,
    })
}

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
        .execute_kv_command(
            &id,
            auth_user.id(),
            auth_user.is_admin(),
            &payload.command,
            payload.timeout_ms,
        )
        .await?;

    audit_service.log(
        auth_user.id().to_string(),
        AuditAction::ExecuteQuery,
        AuditEntityType::Query,
        Some(id),
        Some(summarize_kv_for_audit(&payload.command, payload.timeout_ms)),
        AuditStatus::Success,
        get_client_ip(&headers),
        get_user_agent(&headers),
    );

    Ok(Json(result))
}
