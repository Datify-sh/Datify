use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::error::AppError;
use crate::middleware::CurrentUser;

#[derive(Clone, Debug)]
pub struct AuthUser(pub CurrentUser);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<CurrentUser>()
            .cloned()
            .map(AuthUser)
            .ok_or(AppError::Unauthorized)
    }
}

impl AuthUser {
    pub fn id(&self) -> &str {
        &self.0.id
    }

    pub fn email(&self) -> &str {
        &self.0.email
    }

    pub fn role(&self) -> &str {
        &self.0.role
    }

    pub fn is_admin(&self) -> bool {
        self.0.role == "admin"
    }
}

#[derive(Clone, Debug)]
pub struct OptionalAuthUser(pub Option<CurrentUser>);

impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(OptionalAuthUser(
            parts.extensions.get::<CurrentUser>().cloned(),
        ))
    }
}
