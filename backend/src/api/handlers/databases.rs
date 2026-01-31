use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    Extension, Json,
};

use crate::api::extractors::{AuthUser, PaginatedResponse, Pagination};
use crate::domain::models::{
    AuditAction, AuditEntityType, AuditStatus, BranchResponse, ChangePasswordRequest,
    CreateBranchRequest, CreateDatabaseRequest, DatabaseResponse, UpdateDatabaseRequest,
};
use crate::domain::services::{AuditLogService, DatabaseService};
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

pub type DatabaseServiceState = Arc<DatabaseService>;

#[utoipa::path(
    post,
    path = "/api/v1/projects/{project_id}/databases",
    params(
        ("project_id" = String, Path, description = "Project ID")
    ),
    request_body = CreateDatabaseRequest,
    responses(
        (status = 201, description = "Database created successfully", body = DatabaseResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not project owner")
    ),
    tag = "Databases",
    security(("bearer" = []))
)]
pub async fn create_database(
    State(database_service): State<DatabaseServiceState>,
    Extension(audit_service): Extension<Arc<AuditLogService>>,
    headers: HeaderMap,
    auth_user: AuthUser,
    Path(project_id): Path<String>,
    Json(payload): Json<CreateDatabaseRequest>,
) -> AppResult<(StatusCode, Json<DatabaseResponse>)> {
    let database = database_service
        .create(
            &project_id,
            auth_user.id(),
            &payload.name,
            &payload.database_type,
            &payload.postgres_version,
            payload.valkey_version.as_deref(),
            payload.redis_version.as_deref(),
            payload.password.as_deref(),
            payload.cpu_limit,
            payload.memory_limit_mb,
            payload.storage_limit_mb,
        )
        .await?;

    audit_service.log(
        auth_user.id().to_string(),
        AuditAction::CreateDatabase,
        AuditEntityType::Database,
        Some(database.id.clone()),
        Some(serde_json::json!({ "name": database.name, "type": database.database_type })),
        AuditStatus::Success,
        get_client_ip(&headers),
        get_user_agent(&headers),
    );

    Ok((StatusCode::CREATED, Json(database)))
}

#[utoipa::path(
    get,
    path = "/api/v1/projects/{project_id}/databases",
    params(
        ("project_id" = String, Path, description = "Project ID"),
        ("limit" = Option<i64>, Query, description = "Number of items per page (default: 50)"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip (default: 0)")
    ),
    responses(
        (status = 200, description = "List of databases with pagination"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not project owner")
    ),
    tag = "Databases",
    security(("bearer" = []))
)]
pub async fn list_databases(
    State(database_service): State<DatabaseServiceState>,
    auth_user: AuthUser,
    Path(project_id): Path<String>,
    pagination: Pagination,
) -> AppResult<Json<PaginatedResponse<DatabaseResponse>>> {
    let databases = database_service
        .list_by_project(
            &project_id,
            auth_user.id(),
            pagination.limit,
            pagination.offset,
        )
        .await?;

    let total = database_service.count_by_project(&project_id).await?;

    Ok(Json(PaginatedResponse::new(databases, &pagination, total)))
}

#[utoipa::path(
    get,
    path = "/api/v1/databases/{id}",
    params(
        ("id" = String, Path, description = "Database ID")
    ),
    responses(
        (status = 200, description = "Database details", body = DatabaseResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database not found")
    ),
    tag = "Databases",
    security(("bearer" = []))
)]
pub async fn get_database(
    State(database_service): State<DatabaseServiceState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<DatabaseResponse>> {
    if !database_service.check_access(&id, auth_user.id()).await? {
        return Err(AppError::Forbidden);
    }

    let database = database_service
        .get_by_id_response(&id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", id)))?;

    Ok(Json(database))
}

#[utoipa::path(
    put,
    path = "/api/v1/databases/{id}",
    params(
        ("id" = String, Path, description = "Database ID")
    ),
    request_body = UpdateDatabaseRequest,
    responses(
        (status = 200, description = "Database updated successfully", body = DatabaseResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database not found")
    ),
    tag = "Databases",
    security(("bearer" = []))
)]
pub async fn update_database(
    State(database_service): State<DatabaseServiceState>,
    Extension(audit_service): Extension<Arc<AuditLogService>>,
    headers: HeaderMap,
    auth_user: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<UpdateDatabaseRequest>,
) -> AppResult<Json<DatabaseResponse>> {
    let database = database_service
        .update(
            &id,
            auth_user.id(),
            payload.name.as_deref(),
            payload.cpu_limit,
            payload.memory_limit_mb,
            payload.storage_limit_mb,
            payload.public_exposed,
        )
        .await?;

    audit_service.log(
        auth_user.id().to_string(),
        AuditAction::UpdateDatabase,
        AuditEntityType::Database,
        Some(id),
        None,
        AuditStatus::Success,
        get_client_ip(&headers),
        get_user_agent(&headers),
    );

    Ok(Json(database))
}

#[utoipa::path(
    post,
    path = "/api/v1/databases/{id}/change-password",
    params(
        ("id" = String, Path, description = "Database ID")
    ),
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed successfully", body = DatabaseResponse),
        (status = 400, description = "Invalid request or incorrect current password"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database not found"),
        (status = 409, description = "Database must be stopped to change password")
    ),
    tag = "Databases",
    security(("bearer" = []))
)]
pub async fn change_database_password(
    State(database_service): State<DatabaseServiceState>,
    Extension(audit_service): Extension<Arc<AuditLogService>>,
    headers: HeaderMap,
    auth_user: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<ChangePasswordRequest>,
) -> AppResult<Json<DatabaseResponse>> {
    let database = database_service
        .change_password(
            &id,
            auth_user.id(),
            &payload.current_password,
            &payload.new_password,
        )
        .await?;

    audit_service.log(
        auth_user.id().to_string(),
        AuditAction::ChangePassword,
        AuditEntityType::Database,
        Some(id),
        None,
        AuditStatus::Success,
        get_client_ip(&headers),
        get_user_agent(&headers),
    );

    Ok(Json(database))
}

#[utoipa::path(
    delete,
    path = "/api/v1/databases/{id}",
    params(
        ("id" = String, Path, description = "Database ID")
    ),
    responses(
        (status = 204, description = "Database deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database not found")
    ),
    tag = "Databases",
    security(("bearer" = []))
)]
pub async fn delete_database(
    State(database_service): State<DatabaseServiceState>,
    Extension(audit_service): Extension<Arc<AuditLogService>>,
    headers: HeaderMap,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    database_service.delete(&id, auth_user.id()).await?;

    audit_service.log(
        auth_user.id().to_string(),
        AuditAction::DeleteDatabase,
        AuditEntityType::Database,
        Some(id),
        None,
        AuditStatus::Success,
        get_client_ip(&headers),
        get_user_agent(&headers),
    );

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/v1/databases/{id}/start",
    params(
        ("id" = String, Path, description = "Database ID")
    ),
    responses(
        (status = 200, description = "Database started successfully", body = DatabaseResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database not found"),
        (status = 409, description = "Database already running")
    ),
    tag = "Databases",
    security(("bearer" = []))
)]
pub async fn start_database(
    State(database_service): State<DatabaseServiceState>,
    Extension(audit_service): Extension<Arc<AuditLogService>>,
    headers: HeaderMap,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<DatabaseResponse>> {
    let database = database_service.start(&id, auth_user.id()).await?;

    audit_service.log(
        auth_user.id().to_string(),
        AuditAction::StartDatabase,
        AuditEntityType::Database,
        Some(id),
        None,
        AuditStatus::Success,
        get_client_ip(&headers),
        get_user_agent(&headers),
    );

    Ok(Json(database))
}

#[utoipa::path(
    post,
    path = "/api/v1/databases/{id}/stop",
    params(
        ("id" = String, Path, description = "Database ID")
    ),
    responses(
        (status = 200, description = "Database stopped successfully", body = DatabaseResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database not found"),
        (status = 409, description = "Database already stopped")
    ),
    tag = "Databases",
    security(("bearer" = []))
)]
pub async fn stop_database(
    State(database_service): State<DatabaseServiceState>,
    Extension(audit_service): Extension<Arc<AuditLogService>>,
    headers: HeaderMap,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<DatabaseResponse>> {
    let database = database_service.stop(&id, auth_user.id()).await?;

    audit_service.log(
        auth_user.id().to_string(),
        AuditAction::StopDatabase,
        AuditEntityType::Database,
        Some(id),
        None,
        AuditStatus::Success,
        get_client_ip(&headers),
        get_user_agent(&headers),
    );

    Ok(Json(database))
}

#[utoipa::path(
    get,
    path = "/api/v1/databases/{id}/branches",
    params(
        ("id" = String, Path, description = "Database ID")
    ),
    responses(
        (status = 200, description = "List of branches", body = Vec<BranchResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database not found")
    ),
    tag = "Branches",
    security(("bearer" = []))
)]
pub async fn list_branches(
    State(database_service): State<DatabaseServiceState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<Vec<BranchResponse>>> {
    let branches = database_service.list_branches(&id, auth_user.id()).await?;
    Ok(Json(branches))
}

#[utoipa::path(
    post,
    path = "/api/v1/databases/{id}/branches",
    params(
        ("id" = String, Path, description = "Database ID")
    ),
    request_body = CreateBranchRequest,
    responses(
        (status = 201, description = "Branch created successfully", body = DatabaseResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database not found"),
        (status = 409, description = "Branch already exists")
    ),
    tag = "Branches",
    security(("bearer" = []))
)]
pub async fn create_branch(
    State(database_service): State<DatabaseServiceState>,
    Extension(audit_service): Extension<Arc<AuditLogService>>,
    headers: HeaderMap,
    auth_user: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<CreateBranchRequest>,
) -> AppResult<(StatusCode, Json<DatabaseResponse>)> {
    let database = database_service
        .create_branch(&id, auth_user.id(), &payload.name, payload.include_data)
        .await?;

    audit_service.log(
        auth_user.id().to_string(),
        AuditAction::CreateBranch,
        AuditEntityType::Branch,
        Some(database.id.clone()),
        Some(serde_json::json!({ "name": payload.name, "parent_id": id })),
        AuditStatus::Success,
        get_client_ip(&headers),
        get_user_agent(&headers),
    );

    Ok((StatusCode::CREATED, Json(database)))
}

#[utoipa::path(
    post,
    path = "/api/v1/databases/{id}/sync-from-parent",
    params(
        ("id" = String, Path, description = "Database ID")
    ),
    responses(
        (status = 200, description = "Sync completed successfully", body = DatabaseResponse),
        (status = 400, description = "Invalid request - database is root or not running"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database not found")
    ),
    tag = "Branches",
    security(("bearer" = []))
)]
pub async fn sync_from_parent(
    State(database_service): State<DatabaseServiceState>,
    Extension(audit_service): Extension<Arc<AuditLogService>>,
    headers: HeaderMap,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<DatabaseResponse>> {
    let database = database_service
        .sync_from_parent(&id, auth_user.id())
        .await?;

    audit_service.log(
        auth_user.id().to_string(),
        AuditAction::SyncFromParent,
        AuditEntityType::Branch,
        Some(id),
        None,
        AuditStatus::Success,
        get_client_ip(&headers),
        get_user_agent(&headers),
    );

    Ok(Json(database))
}
