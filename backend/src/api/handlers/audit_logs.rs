use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};

use crate::api::extractors::{AuthUser, PaginatedResponse, Pagination};
use crate::domain::models::{AuditLogFilter, AuditLogResponse};
use crate::domain::services::AuditLogService;
use crate::error::AppResult;

pub type AuditLogServiceState = Arc<AuditLogService>;

#[utoipa::path(
    get,
    path = "/api/v1/audit-logs",
    params(
        ("page" = Option<i64>, Query, description = "Page number (default: 1)"),
        ("page_size" = Option<i64>, Query, description = "Items per page (default: 20)"),
        ("action" = Option<String>, Query, description = "Filter by action"),
        ("entity_type" = Option<String>, Query, description = "Filter by entity type"),
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("start_date" = Option<String>, Query, description = "Filter by start date"),
        ("end_date" = Option<String>, Query, description = "Filter by end date")
    ),
    responses(
        (status = 200, description = "List of audit logs with pagination"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Audit Logs",
    security(("bearer" = []))
)]
pub async fn list_audit_logs(
    State(audit_service): State<AuditLogServiceState>,
    auth_user: AuthUser,
    pagination: Pagination,
    Query(filter): Query<AuditLogFilter>,
) -> AppResult<Json<PaginatedResponse<AuditLogResponse>>> {
    let logs = audit_service
        .list(
            auth_user.id(),
            auth_user.is_admin(),
            &filter,
            pagination.limit,
            pagination.offset,
        )
        .await?;

    let total = audit_service
        .count(auth_user.id(), auth_user.is_admin(), &filter)
        .await?;

    Ok(Json(PaginatedResponse::new(logs, &pagination, total)))
}
