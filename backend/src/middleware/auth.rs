use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};

use crate::domain::models::User;
use crate::domain::services::AuthService;
use crate::error::AppError;

#[derive(Clone)]
pub struct AuthState {
    pub auth_service: Arc<AuthService>,
}

#[derive(Clone, Debug)]
pub struct CurrentUser {
    pub id: String,
    pub email: String,
    pub role: String,
}

impl From<User> for CurrentUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            role: user.role,
        }
    }
}

pub async fn auth_middleware(
    State(state): State<AuthState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_token(&request)?;

    let claims = state.auth_service.validate_token(&token).await?;

    let current_user = CurrentUser {
        id: claims.sub,
        email: claims.email,
        role: claims.role,
    };

    request.extensions_mut().insert(current_user);

    Ok(next.run(request).await)
}

pub async fn optional_auth_middleware(
    State(state): State<AuthState>,
    mut request: Request,
    next: Next,
) -> Response {
    if let Ok(token) = extract_token(&request) {
        if let Ok(claims) = state.auth_service.validate_token(&token).await {
            let current_user = CurrentUser {
                id: claims.sub,
                email: claims.email,
                role: claims.role,
            };
            request.extensions_mut().insert(current_user);
        }
    }

    next.run(request).await
}

pub async fn admin_middleware(
    State(state): State<AuthState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_token(&request)?;

    let claims = state.auth_service.validate_token(&token).await?;

    if claims.role != "admin" {
        return Err(AppError::Forbidden);
    }

    let current_user = CurrentUser {
        id: claims.sub,
        email: claims.email,
        role: claims.role,
    };

    request.extensions_mut().insert(current_user);

    Ok(next.run(request).await)
}

fn extract_token(request: &Request) -> Result<String, AppError> {
    if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        let auth_str = auth_header.to_str().map_err(|_| AppError::InvalidToken)?;

        if let Some(token) = auth_str.strip_prefix("Bearer ") {
            tracing::debug!("Token extracted from Authorization header");
            return Ok(token.to_string());
        }
    }

    if let Some(cookie_header) = request.headers().get(header::COOKIE) {
        tracing::debug!("Cookie header present: {:?}", cookie_header);
        if let Ok(cookies) = cookie_header.to_str() {
            for cookie in cookies.split(';') {
                let cookie = cookie.trim();
                if let Some(token) = cookie.strip_prefix("access_token=") {
                    tracing::debug!("Token extracted from cookie");
                    return Ok(token.to_string());
                }
            }
        }
        tracing::debug!("No access_token cookie found in cookies");
    } else {
        tracing::debug!("No Cookie header present in request");
    }

    Err(AppError::Unauthorized)
}
