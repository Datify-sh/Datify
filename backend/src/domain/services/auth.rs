use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::ValidateEmail;

use crate::config::Settings;
use crate::domain::models::{User, UserResponse};
use crate::error::{AppError, AppResult};
use crate::repositories::{TokenRepository, UserRepository};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
    pub token_type: String,
    pub jti: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub user: UserResponse,
    pub tokens: AuthTokens,
}

#[derive(Clone)]
pub struct AuthService {
    user_repo: UserRepository,
    token_repo: TokenRepository,
    settings: Arc<Settings>,
}

impl AuthService {
    pub fn new(
        user_repo: UserRepository,
        token_repo: TokenRepository,
        settings: Arc<Settings>,
    ) -> Self {
        Self {
            user_repo,
            token_repo,
            settings,
        }
    }

    pub fn secure_cookies(&self) -> bool {
        self.settings.security.secure_cookies
    }

    pub async fn register(&self, email: &str, password: &str) -> AppResult<LoginResponse> {
        if !email.validate_email() {
            return Err(AppError::Validation("Invalid email format".to_string()));
        }

        self.validate_password_strength(password)?;

        if self.user_repo.find_by_email(email).await?.is_some() {
            return Err(AppError::AlreadyExists(format!(
                "User with email '{}' already exists",
                email
            )));
        }

        let password_hash = self.hash_password(password).await?;

        let count = self.user_repo.count().await?;
        let role = if count == 0 { "admin" } else { "user" };

        let user = self.user_repo.create(email, &password_hash, role).await?;

        let tokens = self.generate_tokens(&user)?;

        Ok(LoginResponse {
            user: user.into(),
            tokens,
        })
    }

    fn validate_password_strength(&self, password: &str) -> AppResult<()> {
        if password.len() < self.settings.auth.password_min_length {
            return Err(AppError::Validation(format!(
                "Password must be at least {} characters",
                self.settings.auth.password_min_length
            )));
        }

        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        let has_special = password.chars().any(|c| !c.is_alphanumeric());

        if !has_uppercase || !has_lowercase || !has_digit || !has_special {
            return Err(AppError::Validation(
                "Password must contain at least one uppercase letter, one lowercase letter, one \
                 digit, and one special character"
                    .to_string(),
            ));
        }

        Ok(())
    }

    pub async fn login(&self, email: &str, password: &str) -> AppResult<LoginResponse> {
        let user = self
            .user_repo
            .find_by_email(email)
            .await?
            .ok_or(AppError::InvalidCredentials)?;

        if !self.verify_password(password, &user.password_hash)? {
            return Err(AppError::InvalidCredentials);
        }

        let tokens = self.generate_tokens(&user)?;

        Ok(LoginResponse {
            user: user.into(),
            tokens,
        })
    }

    pub async fn refresh_token(&self, refresh_token: &str) -> AppResult<AuthTokens> {
        let claims = self.decode_token(refresh_token)?;

        if claims.token_type != "refresh" {
            return Err(AppError::InvalidToken);
        }

        if self.token_repo.is_revoked(&claims.jti).await? {
            return Err(AppError::InvalidToken);
        }

        let user = self
            .user_repo
            .find_by_id(&claims.sub)
            .await?
            .ok_or(AppError::InvalidToken)?;

        let expires_at = DateTime::from_timestamp(claims.exp, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| Utc::now().to_rfc3339());
        self.token_repo
            .revoke_token(&claims.jti, &claims.sub, &expires_at)
            .await?;

        self.generate_tokens(&user)
    }

    pub async fn validate_token(&self, token: &str) -> AppResult<Claims> {
        let claims = self.decode_token(token)?;

        if claims.token_type != "access" {
            return Err(AppError::InvalidToken);
        }

        if self.token_repo.is_revoked(&claims.jti).await? {
            return Err(AppError::InvalidToken);
        }

        if let Some(revoked_at) = self
            .token_repo
            .get_user_revocation_timestamp(&claims.sub)
            .await?
        {
            if let Ok(revoked_time) = DateTime::parse_from_rfc3339(&revoked_at) {
                let token_issued = DateTime::from_timestamp(claims.iat, 0);
                if let Some(issued) = token_issued {
                    if issued < revoked_time {
                        return Err(AppError::InvalidToken);
                    }
                }
            }
        }

        Ok(claims)
    }

    pub async fn logout(&self, token: &str) -> AppResult<()> {
        let claims = self.decode_token(token)?;
        let expires_at = DateTime::from_timestamp(claims.exp, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| Utc::now().to_rfc3339());
        self.token_repo
            .revoke_token(&claims.jti, &claims.sub, &expires_at)
            .await?;
        Ok(())
    }

    pub async fn logout_all(&self, user_id: &str) -> AppResult<()> {
        self.token_repo.revoke_all_user_tokens(user_id).await?;
        Ok(())
    }

    pub async fn get_user_by_id(&self, user_id: &str) -> AppResult<Option<User>> {
        self.user_repo.find_by_id(user_id).await
    }

    fn generate_tokens(&self, user: &User) -> AppResult<AuthTokens> {
        let now = Utc::now();
        let access_exp = now + Duration::hours(self.settings.auth.jwt_expiration_hours);
        let refresh_exp = now + Duration::days(self.settings.auth.refresh_token_expiration_days);

        let access_claims = Claims {
            sub: user.id.clone(),
            email: user.email.clone(),
            role: user.role.clone(),
            exp: access_exp.timestamp(),
            iat: now.timestamp(),
            token_type: "access".to_string(),
            jti: Uuid::new_v4().to_string(),
        };

        let refresh_claims = Claims {
            sub: user.id.clone(),
            email: user.email.clone(),
            role: user.role.clone(),
            exp: refresh_exp.timestamp(),
            iat: now.timestamp(),
            token_type: "refresh".to_string(),
            jti: Uuid::new_v4().to_string(),
        };

        let header = Header::new(Algorithm::HS256);

        let access_token = encode(
            &header,
            &access_claims,
            &EncodingKey::from_secret(self.settings.auth.jwt_secret.as_bytes()),
        )?;

        let refresh_token = encode(
            &header,
            &refresh_claims,
            &EncodingKey::from_secret(self.settings.auth.jwt_secret.as_bytes()),
        )?;

        Ok(AuthTokens {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.settings.auth.jwt_expiration_hours * 3600,
        })
    }

    fn decode_token(&self, token: &str) -> AppResult<Claims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_required_spec_claims(&["exp", "iat", "sub"]);

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.settings.auth.jwt_secret.as_bytes()),
            &validation,
        )?;

        Ok(token_data.claims)
    }

    async fn hash_password(&self, password: &str) -> AppResult<String> {
        crate::utils::hash::hash_password(password).await
    }

    fn verify_password(&self, password: &str, hash: &str) -> AppResult<bool> {
        crate::utils::hash::verify_password(password, hash)
    }
}
