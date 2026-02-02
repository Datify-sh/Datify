use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum UserRole {
    Admin,
    #[default]
    User,
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::User => write!(f, "user"),
        }
    }
}

impl TryFrom<&str> for UserRole {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "admin" => Ok(Self::Admin),
            "user" => Ok(Self::User),
            _ => Err(format!("Invalid role: {}", value)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct User {
    pub id: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    pub email_verified: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl User {
    pub fn role_enum(&self) -> UserRole {
        UserRole::try_from(self.role.as_str()).unwrap_or_default()
    }

    pub fn is_admin(&self) -> bool {
        self.role_enum() == UserRole::Admin
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub role: String,
    pub email_verified: bool,
    pub created_at: String,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            role: user.role,
            email_verified: user.email_verified,
            created_at: user.created_at,
        }
    }
}
