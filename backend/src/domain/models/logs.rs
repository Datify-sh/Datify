use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct LogQueryParams {
    #[serde(default = "default_tail")]
    pub tail: i64,
    pub since: Option<i64>,
    #[serde(default)]
    pub timestamps: bool,
}

fn default_tail() -> i64 {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LogType {
    Setup,
    Runtime,
    System,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LogEntryResponse {
    pub timestamp: Option<String>,
    pub log_type: LogType,
    pub stream: String,
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LogsResponse {
    pub database_id: String,
    pub container_id: Option<String>,
    pub entries: Vec<LogEntryResponse>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PullProgressResponse {
    pub status: String,
    pub progress: Option<String>,
    pub layer_id: Option<String>,
    pub current_bytes: Option<i64>,
    pub total_bytes: Option<i64>,
    pub percent_complete: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SetupStatusResponse {
    pub database_id: String,
    pub phase: SetupPhase,
    pub message: String,
    pub progress: Option<PullProgressResponse>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SetupPhase {
    PullingImage,
    CreatingContainer,
    StartingContainer,
    WaitingForReady,
    Complete,
    Failed,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TerminalInputMessage {
    Input { data: String },
    Resize { cols: u16, rows: u16 },
    Ping,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TerminalOutputMessage {
    Output { data: String },
    Error { message: String },
    Connected { exec_id: String },
    Pong,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LogStreamMessage {
    Log(LogEntryResponse),
    Error { message: String },
    Connected,
    Ping,
}
