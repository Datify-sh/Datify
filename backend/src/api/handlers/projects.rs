use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    Extension, Json,
};

use crate::api::extractors::{AuthUser, PaginatedResponse, Pagination};
use crate::domain::models::{
    AuditAction, AuditEntityType, AuditStatus, CreateProjectRequest, ProjectResponse,
    ProjectWithStats, UpdateProjectRequest,
};
use crate::domain::services::{AuditLogService, ProjectService};
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

pub type ProjectServiceState = Arc<ProjectService>;

#[utoipa::path(
    post,
    path = "/api/v1/projects",
    request_body = CreateProjectRequest,
    responses(
        (status = 201, description = "Project created successfully", body = ProjectResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Projects",
    security(("bearer" = []))
)]
pub async fn create_project(
    State(project_service): State<ProjectServiceState>,
    Extension(audit_service): Extension<Arc<AuditLogService>>,
    headers: HeaderMap,
    auth_user: AuthUser,
    Json(payload): Json<CreateProjectRequest>,
) -> AppResult<(StatusCode, Json<ProjectResponse>)> {
    let settings = payload
        .settings
        .map(|s| serde_json::to_string(&s).unwrap_or_default());

    let project = project_service
        .create(
            auth_user.id(),
            &payload.name,
            payload.description.as_deref(),
            settings.as_deref(),
        )
        .await?;

    audit_service.log(
        auth_user.id().to_string(),
        AuditAction::CreateProject,
        AuditEntityType::Project,
        Some(project.id.clone()),
        Some(serde_json::json!({ "name": project.name })),
        AuditStatus::Success,
        get_client_ip(&headers),
        get_user_agent(&headers),
    );

    Ok((StatusCode::CREATED, Json(project)))
}

#[utoipa::path(
    get,
    path = "/api/v1/projects",
    params(
        ("limit" = Option<i64>, Query, description = "Number of items per page (default: 50)"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip (default: 0)")
    ),
    responses(
        (status = 200, description = "List of projects with pagination"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Projects",
    security(("bearer" = []))
)]
pub async fn list_projects(
    State(project_service): State<ProjectServiceState>,
    auth_user: AuthUser,
    pagination: Pagination,
) -> AppResult<Json<PaginatedResponse<ProjectWithStats>>> {
    let projects = project_service
        .list_by_user_with_stats(auth_user.id(), pagination.limit, pagination.offset)
        .await?;

    let total = project_service.count_by_user(auth_user.id()).await?;

    Ok(Json(PaginatedResponse::new(projects, &pagination, total)))
}

#[utoipa::path(
    get,
    path = "/api/v1/projects/{id}",
    params(
        ("id" = String, Path, description = "Project ID")
    ),
    responses(
        (status = 200, description = "Project details", body = ProjectWithStats),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not project owner"),
        (status = 404, description = "Project not found")
    ),
    tag = "Projects",
    security(("bearer" = []))
)]
pub async fn get_project(
    State(project_service): State<ProjectServiceState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<ProjectWithStats>> {
    let project = project_service
        .get_by_id_with_stats(&id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Project '{}' not found", id)))?;

    if !project_service.is_owner(&id, auth_user.id()).await? {
        return Err(AppError::Forbidden);
    }

    Ok(Json(project))
}

#[utoipa::path(
    put,
    path = "/api/v1/projects/{id}",
    params(
        ("id" = String, Path, description = "Project ID")
    ),
    request_body = UpdateProjectRequest,
    responses(
        (status = 200, description = "Project updated successfully", body = ProjectResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not project owner"),
        (status = 404, description = "Project not found")
    ),
    tag = "Projects",
    security(("bearer" = []))
)]
pub async fn update_project(
    State(project_service): State<ProjectServiceState>,
    Extension(audit_service): Extension<Arc<AuditLogService>>,
    headers: HeaderMap,
    auth_user: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<UpdateProjectRequest>,
) -> AppResult<Json<ProjectResponse>> {
    let settings = payload
        .settings
        .map(|s| serde_json::to_string(&s).unwrap_or_default());

    let project = project_service
        .update(
            &id,
            auth_user.id(),
            payload.name.as_deref(),
            payload.description.as_deref(),
            settings.as_deref(),
        )
        .await?;

    audit_service.log(
        auth_user.id().to_string(),
        AuditAction::UpdateProject,
        AuditEntityType::Project,
        Some(id),
        None,
        AuditStatus::Success,
        get_client_ip(&headers),
        get_user_agent(&headers),
    );

    Ok(Json(project))
}

#[utoipa::path(
    delete,
    path = "/api/v1/projects/{id}",
    params(
        ("id" = String, Path, description = "Project ID")
    ),
    responses(
        (status = 204, description = "Project deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not project owner"),
        (status = 404, description = "Project not found")
    ),
    tag = "Projects",
    security(("bearer" = []))
)]
pub async fn delete_project(
    State(project_service): State<ProjectServiceState>,
    Extension(audit_service): Extension<Arc<AuditLogService>>,
    headers: HeaderMap,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    project_service.delete(&id, auth_user.id()).await?;

    audit_service.log(
        auth_user.id().to_string(),
        AuditAction::DeleteProject,
        AuditEntityType::Project,
        Some(id),
        None,
        AuditStatus::Success,
        get_client_ip(&headers),
        get_user_agent(&headers),
    );

    Ok(StatusCode::NO_CONTENT)
}
