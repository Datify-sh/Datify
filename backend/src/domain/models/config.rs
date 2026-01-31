use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFormat {
    File,
    Kv,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConfigSource {
    File,
    Runtime,
    Empty,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DatabaseConfigResponse {
    pub database_id: String,
    pub database_type: String,
    pub format: ConfigFormat,
    pub source: ConfigSource,
    pub content: String,
    pub warnings: Vec<String>,
    pub requires_restart: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateDatabaseConfigRequest {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UpdateDatabaseConfigResponse {
    pub database_id: String,
    pub database_type: String,
    pub applied: bool,
    pub warnings: Vec<String>,
    pub requires_restart: bool,
}
