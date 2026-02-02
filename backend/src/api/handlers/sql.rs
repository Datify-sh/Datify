use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap},
    Extension, Json,
};

use crate::api::extractors::AuthUser;
use crate::domain::models::{AuditAction, AuditEntityType, AuditStatus};
use crate::domain::models::{
    ExecuteQueryRequest, QueryResult, SchemaInfo, TablePreview, TablePreviewQuery,
};
use crate::domain::services::{AuditLogService, SqlService};
use crate::error::{AppError, AppResult};

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

fn summarize_sql_for_audit(sql: &str, limit: i32, timeout_ms: i32) -> serde_json::Value {
    let statement = sql.split_whitespace().next().unwrap_or("").to_uppercase();

    serde_json::json!({
        "statement": statement,
        "length": sql.len(),
        "limit": limit,
        "timeout_ms": timeout_ms,
    })
}

#[derive(Clone)]
pub struct SqlState {
    pub sql_service: Arc<SqlService>,
}

#[utoipa::path(
    get,
    path = "/api/v1/databases/{id}/schema",
    params(
        ("id" = String, Path, description = "Database ID")
    ),
    responses(
        (status = 200, description = "Schema information retrieved", body = SchemaInfo),
        (status = 400, description = "Database not running"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database not found")
    ),
    tag = "SQL",
    security(("bearer" = []))
)]
pub async fn get_database_schema(
    State(state): State<SqlState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<SchemaInfo>> {
    if !state
        .sql_service
        .check_access(&id, auth_user.id(), auth_user.is_admin())
        .await?
    {
        return Err(AppError::Forbidden);
    }

    let schema = state.sql_service.get_schema(&id).await?;
    Ok(Json(schema))
}

#[utoipa::path(
    post,
    path = "/api/v1/databases/{id}/query",
    params(
        ("id" = String, Path, description = "Database ID")
    ),
    request_body = ExecuteQueryRequest,
    responses(
        (status = 200, description = "Query executed successfully", body = QueryResult),
        (status = 400, description = "Database not running or query validation failed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database not found")
    ),
    tag = "SQL",
    security(("bearer" = []))
)]
pub async fn execute_query(
    State(state): State<SqlState>,
    Extension(audit_service): Extension<Arc<AuditLogService>>,
    headers: HeaderMap,
    auth_user: AuthUser,
    Path(id): Path<String>,
    Json(request): Json<ExecuteQueryRequest>,
) -> AppResult<Json<QueryResult>> {
    if !state
        .sql_service
        .check_access(&id, auth_user.id(), auth_user.is_admin())
        .await?
    {
        return Err(AppError::Forbidden);
    }

    let sql = request.sql.trim();
    if sql.is_empty() {
        return Err(AppError::Validation(
            "SQL query cannot be empty".to_string(),
        ));
    }

    // Limit between 1 and 10000, default 1000
    let limit = request.limit.unwrap_or(1000).clamp(1, 10000);
    // Timeout between 1000 and 60000, default 30000
    let timeout_ms = request.timeout_ms.unwrap_or(30000).clamp(1000, 60000);

    let result = state
        .sql_service
        .execute_query(&id, sql, limit, timeout_ms)
        .await?;

    audit_service.log(
        auth_user.id().to_string(),
        AuditAction::ExecuteQuery,
        AuditEntityType::Query,
        Some(id),
        Some(summarize_sql_for_audit(sql, limit, timeout_ms)),
        AuditStatus::Success,
        get_client_ip(&headers),
        get_user_agent(&headers),
    );

    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/api/v1/databases/{id}/tables/{schema}/{table}/preview",
    params(
        ("id" = String, Path, description = "Database ID"),
        ("schema" = String, Path, description = "Schema name"),
        ("table" = String, Path, description = "Table name"),
        ("limit" = Option<i32>, Query, description = "Maximum rows to return (default: 100, max: 1000)"),
        ("offset" = Option<i32>, Query, description = "Number of rows to skip (default: 0)")
    ),
    responses(
        (status = 200, description = "Table preview retrieved", body = TablePreview),
        (status = 400, description = "Database not running or invalid table name"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database or table not found")
    ),
    tag = "SQL",
    security(("bearer" = []))
)]
pub async fn preview_table(
    State(state): State<SqlState>,
    auth_user: AuthUser,
    Path((id, schema, table)): Path<(String, String, String)>,
    Query(query): Query<TablePreviewQuery>,
) -> AppResult<Json<TablePreview>> {
    if !state
        .sql_service
        .check_access(&id, auth_user.id(), auth_user.is_admin())
        .await?
    {
        return Err(AppError::Forbidden);
    }

    let limit = query.limit.unwrap_or(100).clamp(1, 1000);
    let offset = query.offset.unwrap_or(0).max(0);

    let preview = state
        .sql_service
        .preview_table(&id, &schema, &table, limit, offset)
        .await?;

    Ok(Json(preview))
}
