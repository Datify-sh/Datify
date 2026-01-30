use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    #[default]
    Postgres,
    Valkey,
    Redis,
}

impl DatabaseType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Postgres => "postgres",
            Self::Valkey => "valkey",
            Self::Redis => "redis",
        }
    }
}

impl std::str::FromStr for DatabaseType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "postgres" => Ok(Self::Postgres),
            "valkey" => Ok(Self::Valkey),
            "redis" => Ok(Self::Redis),
            _ => Err(format!("Unknown database type: {}", s)),
        }
    }
}

impl std::fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub struct ValkeyVersion;

impl ValkeyVersion {
    pub fn is_valid(version: &str) -> bool {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() < 2 {
            return false;
        }
        parts[0].parse::<u32>().is_ok() && parts[1].parse::<u32>().is_ok()
    }
}

pub struct RedisVersion;

impl RedisVersion {
    pub fn is_valid(version: &str) -> bool {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() < 2 {
            return false;
        }
        parts[0].parse::<u32>().is_ok() && parts[1].parse::<u32>().is_ok()
    }
}

pub struct PostgresVersion;

impl PostgresVersion {
    pub fn is_valid(version: &str) -> bool {
        version.parse::<u32>().is_ok()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Database {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub database_type: String,
    pub postgres_version: String,
    pub valkey_version: Option<String>,
    pub redis_version: Option<String>,
    pub container_id: Option<String>,
    pub container_status: String,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub username: String,
    pub password_encrypted: Option<String>,
    pub cpu_limit: f64,
    pub memory_limit_mb: i32,
    pub storage_limit_mb: i32,
    pub public_exposed: bool,
    pub created_at: String,
    pub updated_at: String,
    pub parent_branch_id: Option<String>,
    pub branch_name: String,
    pub is_default_branch: bool,
    pub forked_at: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDatabaseRequest {
    pub name: String,
    #[serde(default = "default_database_type")]
    #[schema(example = "postgres")]
    pub database_type: String,
    #[serde(default = "default_postgres_version")]
    #[schema(example = "16")]
    pub postgres_version: String,
    #[schema(example = "8.0")]
    pub valkey_version: Option<String>,
    #[schema(example = "7.4")]
    pub redis_version: Option<String>,
    pub password: Option<String>,
    #[serde(default = "default_cpu_limit")]
    pub cpu_limit: f64,
    #[serde(default = "default_memory_limit")]
    pub memory_limit_mb: i32,
    #[serde(default = "default_storage_limit")]
    pub storage_limit_mb: i32,
}

fn default_database_type() -> String {
    "postgres".to_string()
}

fn default_postgres_version() -> String {
    "16".to_string()
}

fn default_cpu_limit() -> f64 {
    1.0
}

fn default_memory_limit() -> i32 {
    512
}

fn default_storage_limit() -> i32 {
    1024
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateDatabaseRequest {
    pub name: Option<String>,
    pub cpu_limit: Option<f64>,
    pub memory_limit_mb: Option<i32>,
    pub storage_limit_mb: Option<i32>,
    pub public_exposed: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DatabaseResponse {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub database_type: String,
    pub postgres_version: String,
    pub valkey_version: Option<String>,
    pub redis_version: Option<String>,
    pub status: String,
    pub connection: Option<ConnectionInfo>,
    pub resources: ResourceLimits,
    pub storage_used_mb: Option<i32>,
    pub public_exposed: bool,
    pub created_at: String,
    pub updated_at: String,
    pub branch: BranchInfo,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BranchInfo {
    pub name: String,
    pub is_default: bool,
    pub parent_id: Option<String>,
    pub forked_at: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateBranchRequest {
    pub name: String,
    #[serde(default = "default_include_data")]
    pub include_data: bool,
}

fn default_include_data() -> bool {
    true
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BranchResponse {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub status: String,
    pub parent_id: Option<String>,
    pub forked_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConnectionInfo {
    pub host: String,
    pub port: i32,
    pub username: String,
    pub password: String,
    pub database: String,
    pub connection_string: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ResourceLimits {
    pub cpu_limit: f64,
    pub memory_limit_mb: i32,
    pub storage_limit_mb: i32,
}

impl Database {
    pub fn container_name(&self) -> String {
        let prefix = match self.database_type.as_str() {
            "valkey" => "datify-valkey",
            "redis" => "datify-redis",
            _ => "datify-pg",
        };
        let sanitized = self
            .name
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric(), "-");
        format!("{}-{}", prefix, sanitized)
    }

    pub fn to_response(&self, password: Option<&str>) -> DatabaseResponse {
        self.to_response_with_host(password, None)
    }

    pub fn to_response_with_host(
        &self,
        password: Option<&str>,
        public_host: Option<&str>,
    ) -> DatabaseResponse {
        let is_key_value = self.database_type == "valkey" || self.database_type == "redis";
        let connection = if self.container_status == "running" {
            self.port.map(|port| {
                let pwd = password.unwrap_or("********");
                let internal_port = if is_key_value { 6379 } else { 5432 };
                let container_name = self.container_name();
                let host = if self.public_exposed {
                    public_host
                        .unwrap_or_else(|| self.host.as_deref().unwrap_or("localhost"))
                        .to_string()
                } else {
                    container_name.clone()
                };
                let display_port = if self.public_exposed {
                    port
                } else {
                    internal_port
                };

                let (database, connection_string) = if is_key_value {
                    (
                        "0".to_string(),
                        format!("redis://:{}@{}:{}/0", pwd, host, display_port),
                    )
                } else {
                    (
                        "postgres".to_string(),
                        format!(
                            "postgresql://{}:{}@{}:{}/postgres",
                            self.username, pwd, host, display_port
                        ),
                    )
                };

                ConnectionInfo {
                    host: host.to_string(),
                    port: display_port,
                    username: self.username.clone(),
                    password: pwd.to_string(),
                    database,
                    connection_string,
                }
            })
        } else {
            None
        };

        DatabaseResponse {
            id: self.id.clone(),
            project_id: self.project_id.clone(),
            name: self.name.clone(),
            database_type: self.database_type.clone(),
            postgres_version: self.postgres_version.clone(),
            valkey_version: self.valkey_version.clone(),
            redis_version: self.redis_version.clone(),
            status: self.container_status.clone(),
            connection,
            resources: ResourceLimits {
                cpu_limit: self.cpu_limit,
                memory_limit_mb: self.memory_limit_mb,
                storage_limit_mb: self.storage_limit_mb,
            },
            storage_used_mb: None,
            public_exposed: self.public_exposed,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            branch: BranchInfo {
                name: self.branch_name.clone(),
                is_default: self.is_default_branch,
                parent_id: self.parent_branch_id.clone(),
                forked_at: self.forked_at.clone(),
            },
        }
    }

    pub fn to_branch_response(&self) -> BranchResponse {
        BranchResponse {
            id: self.id.clone(),
            name: self.branch_name.clone(),
            is_default: self.is_default_branch,
            status: self.container_status.clone(),
            parent_id: self.parent_branch_id.clone(),
            forked_at: self.forked_at.clone(),
            created_at: self.created_at.clone(),
        }
    }
}

impl From<Database> for DatabaseResponse {
    fn from(db: Database) -> Self {
        db.to_response(None)
    }
}
