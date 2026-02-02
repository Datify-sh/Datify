use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Login,
    Logout,
    Register,
    CreateProject,
    UpdateProject,
    DeleteProject,
    CreateDatabase,
    UpdateDatabase,
    DeleteDatabase,
    StartDatabase,
    StopDatabase,
    ChangePassword,
    CreateBranch,
    SyncFromParent,
    ExecuteQuery,
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Login => write!(f, "login"),
            Self::Logout => write!(f, "logout"),
            Self::Register => write!(f, "register"),
            Self::CreateProject => write!(f, "create_project"),
            Self::UpdateProject => write!(f, "update_project"),
            Self::DeleteProject => write!(f, "delete_project"),
            Self::CreateDatabase => write!(f, "create_database"),
            Self::UpdateDatabase => write!(f, "update_database"),
            Self::DeleteDatabase => write!(f, "delete_database"),
            Self::StartDatabase => write!(f, "start_database"),
            Self::StopDatabase => write!(f, "stop_database"),
            Self::ChangePassword => write!(f, "change_password"),
            Self::CreateBranch => write!(f, "create_branch"),
            Self::SyncFromParent => write!(f, "sync_from_parent"),
            Self::ExecuteQuery => write!(f, "execute_query"),
        }
    }
}

impl TryFrom<&str> for AuditAction {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "login" => Ok(Self::Login),
            "logout" => Ok(Self::Logout),
            "register" => Ok(Self::Register),
            "create_project" => Ok(Self::CreateProject),
            "update_project" => Ok(Self::UpdateProject),
            "delete_project" => Ok(Self::DeleteProject),
            "create_database" => Ok(Self::CreateDatabase),
            "update_database" => Ok(Self::UpdateDatabase),
            "delete_database" => Ok(Self::DeleteDatabase),
            "start_database" => Ok(Self::StartDatabase),
            "stop_database" => Ok(Self::StopDatabase),
            "change_password" => Ok(Self::ChangePassword),
            "create_branch" => Ok(Self::CreateBranch),
            "sync_from_parent" => Ok(Self::SyncFromParent),
            "execute_query" => Ok(Self::ExecuteQuery),
            _ => Err(format!("Invalid audit action: {}", value)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuditEntityType {
    User,
    Project,
    Database,
    Branch,
    Query,
}

impl std::fmt::Display for AuditEntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Project => write!(f, "project"),
            Self::Database => write!(f, "database"),
            Self::Branch => write!(f, "branch"),
            Self::Query => write!(f, "query"),
        }
    }
}

impl TryFrom<&str> for AuditEntityType {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "user" => Ok(Self::User),
            "project" => Ok(Self::Project),
            "database" => Ok(Self::Database),
            "branch" => Ok(Self::Branch),
            "query" => Ok(Self::Query),
            _ => Err(format!("Invalid entity type: {}", value)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuditStatus {
    Success,
    Failure,
}

impl std::fmt::Display for AuditStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::Failure => write!(f, "failure"),
        }
    }
}

impl TryFrom<&str> for AuditStatus {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "success" => Ok(Self::Success),
            "failure" => Ok(Self::Failure),
            _ => Err(format!("Invalid audit status: {}", value)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    pub id: String,
    pub user_id: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub changes: Option<String>,
    pub status: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLogWithUser {
    pub id: String,
    pub user_id: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub changes: Option<String>,
    pub status: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: String,
    pub user_email: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AuditLogFilter {
    pub action: Option<String>,
    pub entity_type: Option<String>,
    pub status: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AuditLogResponse {
    pub id: String,
    pub user_id: String,
    pub user_email: Option<String>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub changes: Option<serde_json::Value>,
    pub status: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: String,
}

impl AuditLog {
    pub fn to_response(self, user_email: Option<String>) -> AuditLogResponse {
        let changes = self
            .changes
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok());

        AuditLogResponse {
            id: self.id,
            user_id: self.user_id,
            user_email,
            action: self.action,
            entity_type: self.entity_type,
            entity_id: self.entity_id,
            changes,
            status: self.status,
            ip_address: self.ip_address,
            user_agent: self.user_agent,
            created_at: self.created_at,
        }
    }
}

impl AuditLogWithUser {
    pub fn to_response(self) -> AuditLogResponse {
        let changes = self
            .changes
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok());

        AuditLogResponse {
            id: self.id,
            user_id: self.user_id,
            user_email: self.user_email,
            action: self.action,
            entity_type: self.entity_type,
            entity_id: self.entity_id,
            changes,
            status: self.status,
            ip_address: self.ip_address,
            user_agent: self.user_agent,
            created_at: self.created_at,
        }
    }
}

pub struct CreateAuditLog {
    pub user_id: String,
    pub action: AuditAction,
    pub entity_type: AuditEntityType,
    pub entity_id: Option<String>,
    pub changes: Option<serde_json::Value>,
    pub status: AuditStatus,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
