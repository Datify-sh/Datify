use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiScope {
    Read,
    Write,
    Admin,
}

impl std::fmt::Display for ApiScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read => write!(f, "read"),
            Self::Write => write!(f, "write"),
            Self::Admin => write!(f, "admin"),
        }
    }
}

impl TryFrom<&str> for ApiScope {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            "admin" => Ok(Self::Admin),
            _ => Err(format!("Invalid scope: {}", value)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiKey {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub key_prefix: String,
    #[serde(skip_serializing)]
    pub key_hash: String,
    pub scopes: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

impl ApiKey {
    pub fn scopes_vec(&self) -> Vec<ApiScope> {
        serde_json::from_str::<Vec<String>>(&self.scopes)
            .unwrap_or_default()
            .iter()
            .filter_map(|s| ApiScope::try_from(s.as_str()).ok())
            .collect()
    }

    pub fn has_scope(&self, scope: &ApiScope) -> bool {
        self.scopes_vec().contains(scope)
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = &self.expires_at {
            if let Ok(expiry) = chrono::DateTime::parse_from_rfc3339(expires_at) {
                return expiry < chrono::Utc::now();
            }
        }
        false
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
    pub expires_in_days: Option<i64>,
}

fn default_scopes() -> Vec<String> {
    vec!["read".to_string()]
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub scopes: Vec<String>,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

impl From<ApiKey> for ApiKeyResponse {
    fn from(key: ApiKey) -> Self {
        Self {
            id: key.id,
            name: key.name,
            key_prefix: key.key_prefix,
            scopes: serde_json::from_str(&key.scopes).unwrap_or_default(),
            last_used_at: key.last_used_at,
            expires_at: key.expires_at,
            created_at: key.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiKeyCreatedResponse {
    #[serde(flatten)]
    pub key_info: ApiKeyResponse,
    pub key: String,
}
