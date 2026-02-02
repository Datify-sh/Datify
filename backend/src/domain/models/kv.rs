use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request body for executing KV commands (Redis/Valkey)
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ExecuteKvCommandRequest {
    /// The command to execute (e.g. "GET my:key")
    pub command: String,
    /// Command timeout in milliseconds (default: 5000, max: 60000)
    #[serde(default)]
    pub timeout_ms: Option<i32>,
}

/// Result of executing a KV command
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct KvCommandResult {
    pub result: String,
}
