use std::sync::Arc;

use hex;
use once_cell::sync::Lazy;
use serde::Deserialize;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct ConfigError(String);

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ConfigError {}

pub static SETTINGS: Lazy<RwLock<Arc<Settings>>> = Lazy::new(|| {
    RwLock::new(Arc::new(
        Settings::new().expect("Failed to load configuration"),
    ))
});

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub server: ServerSettings,
    #[serde(default)]
    pub database: DatabaseSettings,
    #[serde(default)]
    pub auth: AuthSettings,
    #[serde(default)]
    pub docker: DockerSettings,
    #[serde(default)]
    pub rate_limit: RateLimitSettings,
    #[serde(default)]
    pub logging: LoggingSettings,
    #[serde(default)]
    pub cors: CorsSettings,
    #[serde(default)]
    pub security: SecuritySettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerSettings {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_workers")]
    pub workers: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseSettings {
    #[serde(default = "default_db_url")]
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthSettings {
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,
    #[serde(default = "default_jwt_expiration")]
    pub jwt_expiration_hours: i64,
    #[serde(default = "default_refresh_expiration")]
    pub refresh_token_expiration_days: i64,
    #[serde(default = "default_password_min_length")]
    pub password_min_length: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DockerSettings {
    #[serde(default = "default_socket_path")]
    pub socket_path: String,
    #[serde(default = "default_network_name")]
    pub network_name: String,
    #[serde(default = "default_postgres_image")]
    pub postgres_image: String,
    #[serde(default = "default_pgbouncer_image")]
    pub pgbouncer_image: String,
    #[serde(default = "default_valkey_image")]
    pub valkey_image: String,
    #[serde(default = "default_redis_image")]
    pub redis_image: String,
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
    #[serde(default = "default_public_host")]
    pub public_host: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitSettings {
    #[serde(default = "default_requests_per_minute")]
    pub requests_per_minute: u32,
    #[serde(default = "default_burst_size")]
    pub burst_size: u32,
    #[serde(default = "default_rate_limit_max_entries")]
    pub max_entries: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingSettings {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CorsSettings {
    #[serde(default = "default_allowed_origins")]
    pub allowed_origins: StringOrVec,
}

#[derive(Debug, Clone)]
pub struct StringOrVec(pub Vec<String>);

impl<'de> serde::Deserialize<'de> for StringOrVec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};

        struct StringOrVecVisitor;

        impl<'de> Visitor<'de> for StringOrVecVisitor {
            type Value = StringOrVec;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or array of strings")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(StringOrVec(
                    v.split(',').map(|s| s.trim().to_string()).collect(),
                ))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(s) = seq.next_element::<String>()? {
                    vec.push(s);
                }
                Ok(StringOrVec(vec))
            }
        }

        deserializer.deserialize_any(StringOrVecVisitor)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecuritySettings {
    #[serde(default = "default_encryption_key")]
    pub encryption_key: String,
    #[serde(default = "default_secure_cookies")]
    pub secure_cookies: bool,
}

fn default_secure_cookies() -> bool {
    std::env::var("ENVIRONMENT")
        .map(|e| e == "production" || e == "prod")
        .unwrap_or(false)
}

// Default value functions
fn default_host() -> String {
    "127.0.0.1".to_string()
}
fn default_port() -> u16 {
    8080
}
fn default_workers() -> usize {
    4
}
fn default_db_url() -> String {
    "sqlite:datify.db?mode=rwc".to_string()
}
fn default_max_connections() -> u32 {
    10
}
fn default_min_connections() -> u32 {
    1
}
fn default_jwt_secret() -> String {
    String::new()
}
fn default_jwt_expiration() -> i64 {
    24
}
fn default_refresh_expiration() -> i64 {
    7
}
fn default_password_min_length() -> usize {
    12
}
fn default_socket_path() -> String {
    "/var/run/docker.sock".to_string()
}
fn default_network_name() -> String {
    "datify_network".to_string()
}
fn default_postgres_image() -> String {
    "postgres:16-alpine".to_string()
}
fn default_pgbouncer_image() -> String {
    "edoburu/pgbouncer:1.23.1".to_string()
}
fn default_valkey_image() -> String {
    "valkey/valkey:8.0-alpine".to_string()
}
fn default_redis_image() -> String {
    "redis:8.0-alpine".to_string()
}
fn default_data_dir() -> String {
    "/var/lib/datify/data".to_string()
}
fn default_public_host() -> String {
    "localhost".to_string()
}
fn default_requests_per_minute() -> u32 {
    60
}
fn default_burst_size() -> u32 {
    10
}
fn default_rate_limit_max_entries() -> u32 {
    10_000
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_format() -> String {
    "pretty".to_string()
}
fn default_encryption_key() -> String {
    String::new()
}
fn default_allowed_origins() -> StringOrVec {
    StringOrVec(vec![
        "http://localhost:5173".to_string(),
        "http://localhost:8080".to_string(),
    ])
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            workers: default_workers(),
        }
    }
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            url: default_db_url(),
            max_connections: default_max_connections(),
            min_connections: default_min_connections(),
        }
    }
}

impl Default for DockerSettings {
    fn default() -> Self {
        Self {
            socket_path: default_socket_path(),
            network_name: default_network_name(),
            postgres_image: default_postgres_image(),
            pgbouncer_image: default_pgbouncer_image(),
            valkey_image: default_valkey_image(),
            redis_image: default_redis_image(),
            data_dir: default_data_dir(),
            public_host: default_public_host(),
        }
    }
}

impl Default for RateLimitSettings {
    fn default() -> Self {
        Self {
            requests_per_minute: default_requests_per_minute(),
            burst_size: default_burst_size(),
            max_entries: default_rate_limit_max_entries(),
        }
    }
}

impl Default for LoggingSettings {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
        }
    }
}

impl Default for CorsSettings {
    fn default() -> Self {
        Self {
            allowed_origins: default_allowed_origins(),
        }
    }
}

impl Default for AuthSettings {
    fn default() -> Self {
        Self {
            jwt_secret: default_jwt_secret(),
            jwt_expiration_hours: default_jwt_expiration(),
            refresh_token_expiration_days: default_refresh_expiration(),
            password_min_length: default_password_min_length(),
        }
    }
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            encryption_key: default_encryption_key(),
            secure_cookies: default_secure_cookies(),
        }
    }
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let settings = Settings {
            server: ServerSettings {
                host: std::env::var("SERVER_HOST").unwrap_or_else(|_| default_host()),
                port: std::env::var("SERVER_PORT")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(default_port),
                workers: std::env::var("SERVER_WORKERS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(default_workers),
            },
            database: DatabaseSettings {
                url: std::env::var("DATABASE_URL").unwrap_or_else(|_| default_db_url()),
                max_connections: std::env::var("DATABASE_MAX_CONNECTIONS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(default_max_connections),
                min_connections: std::env::var("DATABASE_MIN_CONNECTIONS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(default_min_connections),
            },
            auth: AuthSettings {
                jwt_secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| default_jwt_secret()),
                jwt_expiration_hours: std::env::var("JWT_EXPIRATION_HOURS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(default_jwt_expiration),
                refresh_token_expiration_days: std::env::var("REFRESH_TOKEN_EXPIRATION_DAYS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(default_refresh_expiration),
                password_min_length: std::env::var("PASSWORD_MIN_LENGTH")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(default_password_min_length),
            },
            docker: DockerSettings {
                socket_path: std::env::var("DOCKER_SOCKET_PATH")
                    .unwrap_or_else(|_| default_socket_path()),
                network_name: std::env::var("DOCKER_NETWORK_NAME")
                    .unwrap_or_else(|_| default_network_name()),
                postgres_image: std::env::var("POSTGRES_IMAGE")
                    .unwrap_or_else(|_| default_postgres_image()),
                pgbouncer_image: std::env::var("PGBOUNCER_IMAGE")
                    .unwrap_or_else(|_| default_pgbouncer_image()),
                valkey_image: std::env::var("VALKEY_IMAGE")
                    .unwrap_or_else(|_| default_valkey_image()),
                redis_image: std::env::var("REDIS_IMAGE").unwrap_or_else(|_| default_redis_image()),
                data_dir: std::env::var("DOCKER_DATA_DIR").unwrap_or_else(|_| default_data_dir()),
                public_host: std::env::var("DOCKER_PUBLIC_HOST")
                    .unwrap_or_else(|_| default_public_host()),
            },
            rate_limit: RateLimitSettings {
                requests_per_minute: std::env::var("RATE_LIMIT_REQUESTS_PER_MINUTE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(default_requests_per_minute),
                burst_size: std::env::var("RATE_LIMIT_BURST_SIZE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(default_burst_size),
                max_entries: std::env::var("RATE_LIMIT_MAX_ENTRIES")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(default_rate_limit_max_entries),
            },
            logging: LoggingSettings {
                level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| default_log_level()),
                format: std::env::var("LOG_FORMAT").unwrap_or_else(|_| default_log_format()),
            },
            cors: CorsSettings {
                allowed_origins: std::env::var("CORS_ALLOWED_ORIGINS")
                    .map(|s| StringOrVec(s.split(',').map(|s| s.trim().to_string()).collect()))
                    .unwrap_or_else(|_| default_allowed_origins()),
            },
            security: SecuritySettings {
                encryption_key: std::env::var("ENCRYPTION_KEY")
                    .unwrap_or_else(|_| default_encryption_key()),
                secure_cookies: std::env::var("SECURE_COOKIES")
                    .map(|s| s == "true" || s == "1")
                    .unwrap_or(false),
            },
        };

        settings.validate()?;
        Ok(settings)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.auth.jwt_secret.len() < 32 {
            return Err(ConfigError(
                "JWT secret must be at least 32 characters long".to_string(),
            ));
        }

        if self.security.encryption_key.len() != 64 {
            return Err(ConfigError(
                "Encryption key must be 64 hex characters (32 bytes)".to_string(),
            ));
        }
        if hex::decode(&self.security.encryption_key).is_err() {
            return Err(ConfigError(
                "Encryption key must be valid hex".to_string(),
            ));
        }

        Ok(())
    }

    pub async fn get() -> Arc<Settings> {
        SETTINGS.read().await.clone()
    }
}

impl ServerSettings {
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
