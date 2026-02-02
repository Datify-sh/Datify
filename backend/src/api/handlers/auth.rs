use std::sync::Arc;

use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{AppendHeaders, IntoResponse},
    Extension, Json,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::api::extractors::AuthUser;
use crate::domain::models::{AuditAction, AuditEntityType, AuditStatus, UserResponse};
use crate::domain::services::{AuditLogService, AuthService, AuthTokens, LoginResponse};
use crate::error::{AppError, AppResult};

pub fn get_client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
}

pub fn get_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

pub type AuthServiceState = Arc<AuthService>;

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshRequest {
    #[serde(default)]
    pub refresh_token: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully", body = LoginResponse),
        (status = 400, description = "Invalid request or validation error"),
        (status = 409, description = "User already exists")
    ),
    tag = "Authentication"
)]
pub async fn register(
    State(auth_service): State<AuthServiceState>,
    Extension(audit_service): Extension<Arc<AuditLogService>>,
    headers: HeaderMap,
    Json(payload): Json<RegisterRequest>,
) -> AppResult<impl IntoResponse> {
    let response = auth_service
        .register(&payload.email, &payload.password)
        .await?;

    audit_service.log(
        response.user.id.clone(),
        AuditAction::Register,
        AuditEntityType::User,
        Some(response.user.id.clone()),
        None,
        AuditStatus::Success,
        get_client_ip(&headers),
        get_user_agent(&headers),
    );

    let secure = auth_service.secure_cookies();
    let access_cookie = Cookie::build(("access_token", response.tokens.access_token.clone()))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::hours(24));

    let refresh_cookie = Cookie::build(("refresh_token", response.tokens.refresh_token.clone()))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::days(7));

    Ok((
        StatusCode::CREATED,
        AppendHeaders([
            (header::SET_COOKIE, access_cookie.to_string()),
            (header::SET_COOKIE, refresh_cookie.to_string()),
        ]),
        Json(response),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials")
    ),
    tag = "Authentication"
)]
pub async fn login(
    State(auth_service): State<AuthServiceState>,
    Extension(audit_service): Extension<Arc<AuditLogService>>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> AppResult<impl IntoResponse> {
    let response = auth_service
        .login(&payload.email, &payload.password)
        .await?;

    audit_service.log(
        response.user.id.clone(),
        AuditAction::Login,
        AuditEntityType::User,
        Some(response.user.id.clone()),
        None,
        AuditStatus::Success,
        get_client_ip(&headers),
        get_user_agent(&headers),
    );

    let secure = auth_service.secure_cookies();
    let access_cookie = Cookie::build(("access_token", response.tokens.access_token.clone()))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::hours(24));

    let refresh_cookie = Cookie::build(("refresh_token", response.tokens.refresh_token.clone()))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::days(7));

    Ok((
        AppendHeaders([
            (header::SET_COOKIE, access_cookie.to_string()),
            (header::SET_COOKIE, refresh_cookie.to_string()),
        ]),
        Json(response),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Token refreshed successfully", body = AuthTokens),
        (status = 401, description = "Invalid or expired refresh token")
    ),
    tag = "Authentication"
)]
pub async fn refresh(
    State(auth_service): State<AuthServiceState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<RefreshRequest>,
) -> AppResult<impl IntoResponse> {
    let refresh_token = if !payload.refresh_token.is_empty() {
        payload.refresh_token.clone()
    } else if let Some(cookie_header) = headers.get(header::COOKIE) {
        cookie_header
            .to_str()
            .ok()
            .and_then(|cookies| {
                cookies.split(';').find_map(|c| {
                    let c = c.trim();
                    c.strip_prefix("refresh_token=").map(|t| t.to_string())
                })
            })
            .unwrap_or_default()
    } else {
        String::new()
    };

    if refresh_token.is_empty() {
        return Err(AppError::InvalidToken);
    }

    let tokens = auth_service.refresh_token(&refresh_token).await?;

    let secure = auth_service.secure_cookies();
    let access_cookie = Cookie::build(("access_token", tokens.access_token.clone()))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::hours(24));

    let refresh_cookie = Cookie::build(("refresh_token", tokens.refresh_token.clone()))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::days(7));

    Ok((
        AppendHeaders([
            (header::SET_COOKIE, access_cookie.to_string()),
            (header::SET_COOKIE, refresh_cookie.to_string()),
        ]),
        Json(tokens),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/me",
    responses(
        (status = 200, description = "Current user information", body = UserResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "User not found")
    ),
    tag = "Authentication",
    security(("bearer" = []))
)]
pub async fn me(
    State(auth_service): State<AuthServiceState>,
    auth_user: AuthUser,
) -> AppResult<Json<UserResponse>> {
    let user = auth_service
        .get_user_by_id(auth_user.id())
        .await?
        .ok_or(AppError::NotFound("User not found".to_string()))?;

    Ok(Json(user.into()))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LogoutRequest {
    pub access_token: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    request_body = LogoutRequest,
    responses(
        (status = 204, description = "Logged out successfully"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Authentication",
    security(("bearer" = []))
)]
pub async fn logout(
    State(auth_service): State<AuthServiceState>,
    Extension(audit_service): Extension<Arc<AuditLogService>>,
    headers: HeaderMap,
    auth_user: AuthUser,
    Json(payload): Json<LogoutRequest>,
) -> AppResult<impl IntoResponse> {
    if let Some(token) = payload.access_token {
        auth_service.logout(&token).await?;
    }

    audit_service.log(
        auth_user.id().to_string(),
        AuditAction::Logout,
        AuditEntityType::User,
        Some(auth_user.id().to_string()),
        None,
        AuditStatus::Success,
        get_client_ip(&headers),
        get_user_agent(&headers),
    );

    let secure = auth_service.secure_cookies();
    let clear_access = Cookie::build(("access_token", ""))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::ZERO);

    let clear_refresh = Cookie::build(("refresh_token", ""))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::ZERO);

    Ok((
        StatusCode::NO_CONTENT,
        AppendHeaders([
            (header::SET_COOKIE, clear_access.to_string()),
            (header::SET_COOKIE, clear_refresh.to_string()),
        ]),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout-all",
    responses(
        (status = 204, description = "All sessions logged out"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Authentication",
    security(("bearer" = []))
)]
pub async fn logout_all(
    State(auth_service): State<AuthServiceState>,
    auth_user: AuthUser,
) -> AppResult<impl IntoResponse> {
    auth_service.logout_all(auth_user.id()).await?;

    let secure = auth_service.secure_cookies();
    let clear_access = Cookie::build(("access_token", ""))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::ZERO);

    let clear_refresh = Cookie::build(("refresh_token", ""))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::ZERO);

    Ok((
        StatusCode::NO_CONTENT,
        AppendHeaders([
            (header::SET_COOKIE, clear_access.to_string()),
            (header::SET_COOKIE, clear_refresh.to_string()),
        ]),
    ))
}
