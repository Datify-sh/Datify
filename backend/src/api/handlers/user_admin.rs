use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    Extension, Json,
};
use serde::Deserialize;

use crate::api::extractors::AuthUser;
use crate::domain::models::{AuditAction, AuditEntityType, AuditStatus, User};
use crate::domain::services::{AuditLogService, AuthService, ProjectService};
use crate::error::{AppError, AppResult};
use crate::middleware::AuthState;
use crate::repositories::UserRepository;

#[derive(Clone)]
pub struct UserAdminState {
    pub user_repo: UserRepository,
    pub project_service: Arc<ProjectService>,
    pub auth_service: Arc<AuthService>,
}

impl axum::extract::FromRef<UserAdminState> for AuthState {
    fn from_ref(state: &UserAdminState) -> Self {
        Self {
            auth_service: state.auth_service.clone(),
        }
    }
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct UserListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub password: Option<String>,
    pub role: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/users",
    params(
        UserListQuery
    ),
    responses(
        (status = 200, description = "List users", body = Vec<User>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin only")
    ),
    tag = "User Management",
    security(("bearer" = []))
)]
pub async fn list_users(
    State(state): State<UserAdminState>,
    _auth_user: AuthUser,
    Query(query): Query<UserListQuery>,
) -> AppResult<Json<Vec<User>>> {
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);

    let users = state.user_repo.list(limit, offset).await?;
    Ok(Json(users))
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/users/{id}",
    params(
        ("id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Get user details", body = User),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin only"),
        (status = 404, description = "User not found")
    ),
    tag = "User Management",
    security(("bearer" = []))
)]
pub async fn get_user(
    State(state): State<UserAdminState>,
    _auth_user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<User>> {
    let user = state
        .user_repo
        .find_by_id(&id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User '{}' not found", id)))?;

    Ok(Json(user))
}

#[utoipa::path(
    put,
    path = "/api/v1/admin/users/{id}",
    params(
        ("id" = String, Path, description = "User ID")
    ),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated", body = User),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin only"),
        (status = 404, description = "User not found")
    ),
    tag = "User Management",
    security(("bearer" = []))
)]
pub async fn update_user(
    State(state): State<UserAdminState>,
    Extension(audit_service): Extension<Arc<AuditLogService>>,
    headers: HeaderMap,
    auth_user: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<UpdateUserRequest>,
) -> AppResult<Json<User>> {
    let password_hash = if let Some(p) = payload.password {
        Some(crate::utils::hash::hash_password(&p).await?)
    } else {
        None
    };

    let user = state
        .user_repo
        .update(
            &id,
            payload.email.as_deref(),
            password_hash.as_deref(),
            payload.role.as_deref(),
        )
        .await?;

    audit_service.log(
        auth_user.id().to_string(),
        AuditAction::UpdateUser,
        AuditEntityType::User,
        Some(id.clone()),
        Some(serde_json::json!({
            "target_user_id": id,
            "updated_fields": {
                "email": payload.email.is_some(),
                "password": password_hash.is_some(),
                "role": payload.role.is_some()
            }
        })),
        AuditStatus::Success,
        get_client_ip(&headers),
        get_user_agent(&headers),
    );

    Ok(Json(user))
}

#[utoipa::path(
    delete,
    path = "/api/v1/admin/users/{id}",
    params(
        ("id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin only"),
        (status = 404, description = "User not found")
    ),
    tag = "User Management",
    security(("bearer" = []))
)]
pub async fn delete_user(
    State(state): State<UserAdminState>,
    Extension(audit_service): Extension<Arc<AuditLogService>>,
    headers: HeaderMap,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    // 1. List user projects
    let projects = state
        .project_service
        .list_by_user(&id, 1000, 0) // Limit to 1000 projects for now
        .await?;

    // 2. Delete all projects (and their databases)
    for project in projects {
        state
            .project_service
            .delete(&project.id, auth_user.id(), true) // Admin delete
            .await?;
    }

    // 3. Delete user
    state.user_repo.delete(&id).await?;

    audit_service.log(
        auth_user.id().to_string(),
        AuditAction::DeleteUser,
        AuditEntityType::User,
        Some(id.clone()),
        Some(serde_json::json!({ "target_user_id": id })),
        AuditStatus::Success,
        get_client_ip(&headers),
        get_user_agent(&headers),
    );

    Ok(StatusCode::NO_CONTENT)
}

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
