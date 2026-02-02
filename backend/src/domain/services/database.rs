use std::sync::Arc;
use std::time::Duration;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;
use shell_words::split as split_shell_words;

use crate::domain::models::{
    BranchResponse, ConfigFormat, ConfigSource, Database, DatabaseConfigResponse, DatabaseResponse,
    KvCommandResult, PostgresVersion, RedisVersion, UpdateDatabaseConfigResponse, ValkeyVersion,
};
use crate::error::{AppError, AppResult};
use crate::infrastructure::docker::{ContainerConfig, DockerManager};
use crate::repositories::{DatabaseRepository, ProjectRepository};

const KV_SENSITIVE_KEYS: &[&str] = &["requirepass", "masterauth"];

#[derive(Clone)]
pub struct DatabaseService {
    database_repo: DatabaseRepository,
    project_repo: ProjectRepository,
    docker: Arc<DockerManager>,
    data_dir: String,
    host: String,
    encryption_key: [u8; 32],
}

impl DatabaseService {
    pub fn new(
        database_repo: DatabaseRepository,
        project_repo: ProjectRepository,
        docker: Arc<DockerManager>,
        data_dir: String,
        host: String,
        encryption_key_hex: &str,
    ) -> Self {
        let encryption_key = hex::decode(encryption_key_hex)
            .expect("Invalid encryption key hex")
            .try_into()
            .expect("Encryption key must be 32 bytes");

        Self {
            database_repo,
            project_repo,
            docker,
            data_dir,
            host,
            encryption_key,
        }
    }

    fn encrypt_password(&self, password: &str) -> AppResult<String> {
        let cipher = Aes256Gcm::new_from_slice(&self.encryption_key)
            .map_err(|e| AppError::Internal(format!("Encryption init failed: {}", e)))?;

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, password.as_bytes())
            .map_err(|e| AppError::Internal(format!("Encryption failed: {}", e)))?;

        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);
        Ok(hex::encode(result))
    }

    fn decrypt_password(&self, encrypted: &str) -> AppResult<String> {
        let data = hex::decode(encrypted)
            .map_err(|e| AppError::Internal(format!("Invalid encrypted data: {}", e)))?;

        if data.len() < 12 {
            return Err(AppError::Internal("Encrypted data too short".to_string()));
        }

        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let cipher = Aes256Gcm::new_from_slice(&self.encryption_key)
            .map_err(|e| AppError::Internal(format!("Decryption init failed: {}", e)))?;

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AppError::Internal(format!("Decryption failed: {}", e)))?;

        String::from_utf8(plaintext)
            .map_err(|e| AppError::Internal(format!("Invalid UTF-8 in password: {}", e)))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        project_id: &str,
        user_id: &str,
        name: &str,
        database_type: &str,
        postgres_version: &str,
        valkey_version: Option<&str>,
        redis_version: Option<&str>,
        password: Option<&str>,
        cpu_limit: f64,
        memory_limit_mb: i32,
        storage_limit_mb: i32,
    ) -> AppResult<DatabaseResponse> {
        if !self.project_repo.is_owner(project_id, user_id).await? {
            return Err(AppError::Forbidden);
        }

        if name.trim().is_empty() {
            return Err(AppError::Validation(
                "Database name cannot be empty".to_string(),
            ));
        }

        if name.len() > 63 {
            return Err(AppError::Validation(
                "Database name must be 63 characters or less".to_string(),
            ));
        }

        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(AppError::Validation(
                "Database name can only contain letters, numbers, underscores, and hyphens"
                    .to_string(),
            ));
        }

        let is_valkey = database_type == "valkey";
        let is_redis = database_type == "redis";

        if is_valkey {
            let version = valkey_version.unwrap_or("8.0");
            if !ValkeyVersion::is_valid(version) {
                return Err(AppError::Validation("Invalid Valkey version".to_string()));
            }
        } else if is_redis {
            let version = redis_version.unwrap_or("7.4");
            if !RedisVersion::is_valid(version) {
                return Err(AppError::Validation("Invalid Redis version".to_string()));
            }
        } else if !PostgresVersion::is_valid(postgres_version) {
            return Err(AppError::Validation(
                "Invalid PostgreSQL version".to_string(),
            ));
        }

        if self
            .database_repo
            .find_by_name_and_project(project_id, name)
            .await?
            .is_some()
        {
            return Err(AppError::AlreadyExists(format!(
                "Database '{}' already exists in this project",
                name
            )));
        }

        let database = self
            .database_repo
            .create(
                project_id,
                name,
                database_type,
                postgres_version,
                valkey_version,
                redis_version,
                cpu_limit,
                memory_limit_mb,
                storage_limit_mb,
                "main",
                true,
                None,
            )
            .await?;

        let password = password
            .map(|p| p.to_string())
            .unwrap_or_else(generate_password);

        let port = self.database_repo.get_next_available_port().await?;

        let data_path = format!("{}/{}", self.data_dir, database.id);

        std::fs::create_dir_all(&data_path)
            .map_err(|e| AppError::Internal(format!("Failed to create data directory: {}", e)))?;

        let container_id = if is_valkey {
            let version = valkey_version.unwrap_or("8.0");
            let config = ContainerConfig {
                name: database.container_name(),
                image: format!("valkey/valkey:{}-alpine", version),
                env: vec![],
                data_path,
                cpu_limit,
                memory_limit_mb: memory_limit_mb as i64,
                internal_port: 6379,
                exposed_port: Some(port as u16),
                cmd: None,
            };
            self.docker
                .create_valkey_container(config, &password)
                .await?
        } else if is_redis {
            let version = redis_version.unwrap_or("7.4");
            let config = ContainerConfig {
                name: database.container_name(),
                image: format!("redis:{}-alpine", version),
                env: vec![],
                data_path,
                cpu_limit,
                memory_limit_mb: memory_limit_mb as i64,
                internal_port: 6379,
                exposed_port: Some(port as u16),
                cmd: None,
            };
            self.docker
                .create_redis_container(config, &password)
                .await?
        } else {
            let config = ContainerConfig {
                name: database.container_name(),
                image: format!("postgres:{}", postgres_version),
                env: vec![
                    format!("POSTGRES_USER={}", database.username),
                    "POSTGRES_DB=postgres".to_string(),
                ],
                data_path,
                cpu_limit,
                memory_limit_mb: memory_limit_mb as i64,
                internal_port: 5432,
                exposed_port: Some(port as u16),
                cmd: Some(vec![
                    "postgres".to_string(),
                    "-c".to_string(),
                    "shared_preload_libraries=pg_stat_statements".to_string(),
                    "-c".to_string(),
                    "pg_stat_statements.track=all".to_string(),
                ]),
            };
            self.docker
                .create_postgres_container(config, &password)
                .await?
        };

        self.docker.start_container(&container_id).await?;

        let healthy = self.docker.wait_for_healthy(&container_id, 60).await?;
        let status = if healthy { "running" } else { "unhealthy" };

        let password_encrypted = self.encrypt_password(&password)?;

        let database = self
            .database_repo
            .update_container(
                &database.id,
                &container_id,
                status,
                &self.host,
                port,
                &password_encrypted,
            )
            .await?;

        Ok(database.to_response_with_host(Some(&password), Some(&self.host)))
    }

    pub async fn get_by_id(&self, id: &str) -> AppResult<Option<Database>> {
        self.database_repo.find_by_id(id).await
    }

    pub async fn get_by_id_response(&self, id: &str) -> AppResult<Option<DatabaseResponse>> {
        let database = self.database_repo.find_by_id(id).await?;
        match database {
            Some(d) => {
                let password = d
                    .password_encrypted
                    .as_ref()
                    .and_then(|p| self.decrypt_password(p).ok());
                Ok(Some(d.to_response_with_host(
                    password.as_deref(),
                    Some(&self.host),
                )))
            },
            None => Ok(None),
        }
    }

    pub async fn list_by_project(
        &self,
        project_id: &str,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<DatabaseResponse>> {
        if !self.project_repo.is_owner(project_id, user_id).await? {
            return Err(AppError::Forbidden);
        }

        let databases = self
            .database_repo
            .find_by_project_id(project_id, limit, offset)
            .await?;

        Ok(databases
            .into_iter()
            .map(|d| {
                let password = d
                    .password_encrypted
                    .as_ref()
                    .and_then(|p| self.decrypt_password(p).ok());
                d.to_response_with_host(password.as_deref(), Some(&self.host))
            })
            .collect())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        &self,
        id: &str,
        user_id: &str,
        name: Option<&str>,
        cpu_limit: Option<f64>,
        memory_limit_mb: Option<i32>,
        storage_limit_mb: Option<i32>,
        public_exposed: Option<bool>,
    ) -> AppResult<DatabaseResponse> {
        let database = self
            .database_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", id)))?;

        if !self
            .project_repo
            .is_owner(&database.project_id, user_id)
            .await?
        {
            return Err(AppError::Forbidden);
        }

        if let Some(n) = name {
            if n.trim().is_empty() {
                return Err(AppError::Validation(
                    "Database name cannot be empty".to_string(),
                ));
            }
            if n.len() > 63 {
                return Err(AppError::Validation(
                    "Database name must be 63 characters or less".to_string(),
                ));
            }
            if !n
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                return Err(AppError::Validation(
                    "Database name can only contain letters, numbers, underscores, and hyphens"
                        .to_string(),
                ));
            }
        }

        let resource_change =
            cpu_limit.is_some() || memory_limit_mb.is_some() || storage_limit_mb.is_some();
        if resource_change && database.container_status == "running" {
            return Err(AppError::Validation(
                "Cannot change resource limits while database is running. Stop the database first."
                    .to_string(),
            ));
        }

        if name.is_some() && database.container_status == "running" {
            return Err(AppError::Validation(
                "Cannot change database name while running. Stop the database first.".to_string(),
            ));
        }
        if public_exposed.is_some() && database.container_status == "running" {
            return Err(AppError::Validation(
                "Cannot change public access while running. Stop the database first.".to_string(),
            ));
        }

        if let Some(cpu) = cpu_limit {
            if cpu < 0.5 {
                return Err(AppError::Validation(
                    "CPU limit must be at least 0.5 cores".to_string(),
                ));
            }
        }
        if let Some(mem) = memory_limit_mb {
            if mem < 256 {
                return Err(AppError::Validation(
                    "Memory limit must be at least 256 MB".to_string(),
                ));
            }
        }
        if let Some(storage) = storage_limit_mb {
            if storage < 512 {
                return Err(AppError::Validation(
                    "Storage limit must be at least 512 MB".to_string(),
                ));
            }
        }

        self.database_repo
            .update(
                id,
                name,
                cpu_limit,
                memory_limit_mb,
                storage_limit_mb,
                public_exposed,
            )
            .await?;
        self.get_by_id_response(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", id)))
    }

    pub async fn change_password(
        &self,
        id: &str,
        user_id: &str,
        current_password: &str,
        new_password: &str,
    ) -> AppResult<DatabaseResponse> {
        let database = self
            .database_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", id)))?;

        if !self
            .project_repo
            .is_owner(&database.project_id, user_id)
            .await?
        {
            return Err(AppError::Forbidden);
        }

        if database.container_status == "running" {
            return Err(AppError::Validation(
                "Cannot change password while database is running. Stop the database first."
                    .to_string(),
            ));
        }

        let stored_password = database
            .password_encrypted
            .as_ref()
            .and_then(|p| self.decrypt_password(p).ok())
            .ok_or_else(|| AppError::Internal("No password stored for database".to_string()))?;

        if stored_password != current_password {
            return Err(AppError::Validation(
                "Current password is incorrect".to_string(),
            ));
        }

        if new_password.len() < 8 {
            return Err(AppError::Validation(
                "New password must be at least 8 characters".to_string(),
            ));
        }

        let new_password_encrypted = self.encrypt_password(new_password)?;
        self.database_repo
            .update_password(id, &new_password_encrypted)
            .await?;

        self.get_by_id_response(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", id)))
    }

    pub async fn delete(&self, id: &str, user_id: &str) -> AppResult<()> {
        let database = self
            .database_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", id)))?;

        if !self
            .project_repo
            .is_owner(&database.project_id, user_id)
            .await?
        {
            return Err(AppError::Forbidden);
        }

        if let Some(container_id) = &database.container_id {
            if self.docker.container_exists(container_id).await? {
                let _ = self.docker.stop_container(container_id).await;
                let _ = self.docker.remove_container(container_id, true).await;
            }
        }

        let data_path = format!("{}/{}", self.data_dir, id);
        let _ = std::fs::remove_dir_all(&data_path);

        self.database_repo.delete(id).await
    }

    pub async fn start(&self, id: &str, user_id: &str) -> AppResult<DatabaseResponse> {
        let database = self
            .database_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", id)))?;

        if !self
            .project_repo
            .is_owner(&database.project_id, user_id)
            .await?
        {
            return Err(AppError::Forbidden);
        }

        let container_id = database
            .container_id
            .as_ref()
            .ok_or_else(|| AppError::Internal("Database has no container".to_string()))?;

        self.docker.start_container(container_id).await?;
        self.database_repo.update_status(id, "running").await?;

        self.get_by_id_response(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", id)))
    }

    pub async fn stop(&self, id: &str, user_id: &str) -> AppResult<DatabaseResponse> {
        let database = self
            .database_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", id)))?;

        if !self
            .project_repo
            .is_owner(&database.project_id, user_id)
            .await?
        {
            return Err(AppError::Forbidden);
        }

        let container_id = database
            .container_id
            .as_ref()
            .ok_or_else(|| AppError::Internal("Database has no container".to_string()))?;

        self.docker.stop_container(container_id).await?;
        self.database_repo.update_status(id, "stopped").await?;

        self.get_by_id_response(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", id)))
    }

    pub async fn get_project_id(&self, database_id: &str) -> AppResult<Option<String>> {
        self.database_repo.get_project_id(database_id).await
    }

    pub async fn count_by_project(&self, project_id: &str) -> AppResult<i64> {
        self.database_repo.count_by_project(project_id).await
    }

    pub async fn check_access(&self, database_id: &str, user_id: &str) -> AppResult<bool> {
        let project_id = self
            .database_repo
            .get_project_id(database_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", database_id)))?;

        self.project_repo.is_owner(&project_id, user_id).await
    }

    pub async fn execute_kv_command(
        &self,
        database_id: &str,
        user_id: &str,
        command: &str,
        timeout_ms: Option<i32>,
    ) -> AppResult<KvCommandResult> {
        if !self.check_access(database_id, user_id).await? {
            return Err(AppError::Forbidden);
        }

        let database = self
            .database_repo
            .find_by_id(database_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", database_id)))?;

        if database.database_type != "redis" && database.database_type != "valkey" {
            return Err(AppError::Validation(
                "KV commands are only supported for Redis or Valkey databases".to_string(),
            ));
        }

        let container_id = database
            .container_id
            .clone()
            .ok_or_else(|| AppError::NotFound("Database has no container".to_string()))?;

        let status = self.docker.get_container_status(&container_id).await?;
        if status != "running" {
            return Err(AppError::Conflict(format!(
                "Container is not running (status: {})",
                status
            )));
        }

        let password = database
            .password_encrypted
            .as_ref()
            .and_then(|p| self.decrypt_password(p).ok())
            .ok_or_else(|| AppError::Internal("No password stored for database".to_string()))?;

        let trimmed = command.trim();
        if trimmed.is_empty() {
            return Err(AppError::Validation("Command cannot be empty".to_string()));
        }

        let args = split_shell_words(trimmed)
            .map_err(|e| AppError::Validation(format!("Invalid command syntax: {}", e)))?;
        if args.is_empty() {
            return Err(AppError::Validation("Command cannot be empty".to_string()));
        }

        let cmd_name = args[0].to_uppercase();
        if matches!(
            cmd_name.as_str(),
            "MONITOR" | "SUBSCRIBE" | "PSUBSCRIBE" | "SSUBSCRIBE"
        ) {
            return Err(AppError::Validation(
                "Streaming commands are not supported in the editor. Use the terminal instead."
                    .to_string(),
            ));
        }

        let cli = if database.database_type == "valkey" {
            "valkey-cli"
        } else {
            "redis-cli"
        };

        let mut cmd = vec![
            cli.to_string(),
            "--no-auth-warning".to_string(),
            "-a".to_string(),
            password,
            "--raw".to_string(),
        ];
        cmd.extend(args);

        let timeout = timeout_ms.unwrap_or(5000).clamp(1000, 60000);
        let output = tokio::time::timeout(
            Duration::from_millis(timeout as u64),
            self.docker.run_exec(&container_id, cmd, None),
        )
        .await
        .map_err(|_| AppError::Validation("Command timed out".to_string()))??;

        if let Some(code) = output.exit_code {
            if code != 0 {
                let message = if output.stderr.trim().is_empty() {
                    output.stdout.trim()
                } else {
                    output.stderr.trim()
                };
                return Err(AppError::Docker(format!("Command failed: {}", message)));
            }
        }

        let result = if output.stdout.trim().is_empty() {
            output.stderr.trim()
        } else {
            output.stdout.trim()
        };

        Ok(KvCommandResult {
            result: result.to_string(),
        })
    }

    pub async fn list_branches(
        &self,
        database_id: &str,
        user_id: &str,
    ) -> AppResult<Vec<BranchResponse>> {
        if !self.check_access(database_id, user_id).await? {
            return Err(AppError::Forbidden);
        }

        let branches = self.database_repo.find_branches(database_id).await?;
        Ok(branches
            .into_iter()
            .map(|d| d.to_branch_response())
            .collect())
    }

    pub async fn create_branch(
        &self,
        database_id: &str,
        user_id: &str,
        branch_name: &str,
        include_data: bool,
    ) -> AppResult<DatabaseResponse> {
        let source = self
            .database_repo
            .find_by_id(database_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", database_id)))?;

        if !self
            .project_repo
            .is_owner(&source.project_id, user_id)
            .await?
        {
            return Err(AppError::Forbidden);
        }

        if branch_name.trim().is_empty() {
            return Err(AppError::Validation(
                "Branch name cannot be empty".to_string(),
            ));
        }

        if branch_name.len() > 63 {
            return Err(AppError::Validation(
                "Branch name must be 63 characters or less".to_string(),
            ));
        }

        if !branch_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(AppError::Validation(
                "Branch name can only contain letters, numbers, underscores, and hyphens"
                    .to_string(),
            ));
        }

        let branches = self.database_repo.find_branches(database_id).await?;
        if branches.iter().any(|b| b.branch_name == branch_name) {
            return Err(AppError::AlreadyExists(format!(
                "Branch '{}' already exists",
                branch_name
            )));
        }

        let db_name = format!("{}-{}", source.name, branch_name);

        if self
            .database_repo
            .find_by_name_and_project(&source.project_id, &db_name)
            .await?
            .is_some()
        {
            return Err(AppError::AlreadyExists(format!(
                "A database named '{}' already exists. Try a different branch name.",
                db_name
            )));
        }

        let branch = self
            .database_repo
            .create(
                &source.project_id,
                &db_name,
                &source.database_type,
                &source.postgres_version,
                source.valkey_version.as_deref(),
                source.redis_version.as_deref(),
                source.cpu_limit,
                source.memory_limit_mb,
                source.storage_limit_mb,
                branch_name,
                false,
                Some(database_id),
            )
            .await?;

        let password = generate_password();
        let port = self.database_repo.get_next_available_port().await?;
        let container_name = branch.container_name();
        let data_path = format!("{}/{}", self.data_dir, branch.id);

        std::fs::create_dir_all(&data_path)
            .map_err(|e| AppError::Internal(format!("Failed to create data directory: {}", e)))?;

        let container_id = match source.database_type.as_str() {
            "redis" => {
                let version = source.redis_version.as_deref().unwrap_or("7.4");
                let config = ContainerConfig {
                    name: container_name.clone(),
                    image: format!("redis:{}-alpine", version),
                    env: vec![],
                    data_path,
                    cpu_limit: source.cpu_limit,
                    memory_limit_mb: source.memory_limit_mb as i64,
                    internal_port: 6379,
                    exposed_port: Some(port as u16),
                    cmd: None,
                };
                self.docker
                    .create_redis_container(config, &password)
                    .await?
            },
            "valkey" => {
                let version = source.valkey_version.as_deref().unwrap_or("8.0");
                let config = ContainerConfig {
                    name: container_name.clone(),
                    image: format!("valkey/valkey:{}-alpine", version),
                    env: vec![],
                    data_path,
                    cpu_limit: source.cpu_limit,
                    memory_limit_mb: source.memory_limit_mb as i64,
                    internal_port: 6379,
                    exposed_port: Some(port as u16),
                    cmd: None,
                };
                self.docker
                    .create_valkey_container(config, &password)
                    .await?
            },
            _ => {
                let config = ContainerConfig {
                    name: container_name.clone(),
                    image: format!("postgres:{}", source.postgres_version),
                    env: vec![
                        format!("POSTGRES_USER={}", branch.username),
                        "POSTGRES_DB=postgres".to_string(),
                    ],
                    data_path,
                    cpu_limit: source.cpu_limit,
                    memory_limit_mb: source.memory_limit_mb as i64,
                    internal_port: 5432,
                    exposed_port: Some(port as u16),
                    cmd: Some(vec![
                        "postgres".to_string(),
                        "-c".to_string(),
                        "shared_preload_libraries=pg_stat_statements".to_string(),
                        "-c".to_string(),
                        "pg_stat_statements.track=all".to_string(),
                    ]),
                };
                self.docker
                    .create_postgres_container(config, &password)
                    .await?
            },
        };
        self.docker.start_container(&container_id).await?;

        let healthy = self.docker.wait_for_healthy(&container_id, 60).await?;
        if !healthy {
            return Err(AppError::Internal(
                "Branch container failed to start".to_string(),
            ));
        }

        let password_encrypted = self.encrypt_password(&password)?;

        let branch = self
            .database_repo
            .update_container(
                &branch.id,
                &container_id,
                "running",
                &self.host,
                port,
                &password_encrypted,
            )
            .await?;

        if include_data && source.container_status == "running" {
            let source_password = source
                .password_encrypted
                .as_ref()
                .map(|p| self.decrypt_password(p))
                .transpose()?
                .ok_or_else(|| AppError::Internal("Source database has no password".to_string()))?;

            let source_container = source.container_name();

            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            let fork_result = match source.database_type.as_str() {
                "redis" => {
                    self.docker
                        .fork_redis_database(
                            &source_container,
                            &container_name,
                            &source_password,
                            &password,
                        )
                        .await
                },
                "valkey" => {
                    self.docker
                        .fork_valkey_database(
                            &source_container,
                            &container_name,
                            &source_password,
                            &password,
                        )
                        .await
                },
                _ => {
                    self.docker
                        .fork_database(
                            &source_container,
                            &container_name,
                            &source.username,
                            &source_password,
                            &branch.username,
                            &password,
                        )
                        .await
                },
            };

            if let Err(e) = fork_result {
                tracing::warn!("Fork data failed (branch still created): {}", e);
            }
        }

        Ok(branch.to_response_with_host(Some(&password), Some(&self.host)))
    }

    pub async fn sync_from_parent(
        &self,
        database_id: &str,
        user_id: &str,
    ) -> AppResult<DatabaseResponse> {
        let branch = self
            .database_repo
            .find_by_id(database_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", database_id)))?;

        if !self
            .project_repo
            .is_owner(&branch.project_id, user_id)
            .await?
        {
            return Err(AppError::Forbidden);
        }

        let parent_id = branch.parent_branch_id.as_ref().ok_or_else(|| {
            AppError::Validation("Cannot sync root database, it has no parent".to_string())
        })?;

        let parent = self
            .database_repo
            .find_by_id(parent_id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("Parent database '{}' not found", parent_id))
            })?;

        if branch.container_status != "running" {
            return Err(AppError::Validation(
                "Branch must be running to sync".to_string(),
            ));
        }

        if parent.container_status != "running" {
            return Err(AppError::Validation(
                "Parent branch must be running to sync".to_string(),
            ));
        }

        let parent_password = parent
            .password_encrypted
            .as_ref()
            .map(|p| self.decrypt_password(p))
            .transpose()?
            .ok_or_else(|| AppError::Internal("Parent database has no password".to_string()))?;

        let branch_password = branch
            .password_encrypted
            .as_ref()
            .map(|p| self.decrypt_password(p))
            .transpose()?
            .ok_or_else(|| AppError::Internal("Branch database has no password".to_string()))?;

        let parent_container = parent.container_name();
        let branch_container = branch.container_name();

        match branch.database_type.as_str() {
            "redis" => {
                self.docker
                    .fork_redis_database(
                        &parent_container,
                        &branch_container,
                        &parent_password,
                        &branch_password,
                    )
                    .await?
            },
            "valkey" => {
                self.docker
                    .fork_valkey_database(
                        &parent_container,
                        &branch_container,
                        &parent_password,
                        &branch_password,
                    )
                    .await?
            },
            _ => {
                self.docker
                    .fork_database(
                        &parent_container,
                        &branch_container,
                        &parent.username,
                        &parent_password,
                        &branch.username,
                        &branch_password,
                    )
                    .await?
            },
        }

        self.database_repo.update_forked_at(database_id).await?;

        self.get_by_id_response(database_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", database_id)))
    }

    pub async fn get_config(
        &self,
        database_id: &str,
        user_id: &str,
    ) -> AppResult<DatabaseConfigResponse> {
        if !self.check_access(database_id, user_id).await? {
            return Err(AppError::Forbidden);
        }

        let database = self
            .database_repo
            .find_by_id(database_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", database_id)))?;

        let mut warnings = Vec::new();
        let mut requires_restart = false;

        let container_id = database
            .container_id
            .clone()
            .ok_or_else(|| AppError::NotFound("Database has no container".to_string()))?;
        let status = self.docker.get_container_status(&container_id).await?;
        if status != "running" {
            return Err(AppError::Conflict(format!(
                "Container is not running (status: {})",
                status
            )));
        }

        let (format, source, content) = match database.database_type.as_str() {
            "postgres" => {
                requires_restart = true;
                let password = database
                    .password_encrypted
                    .as_ref()
                    .and_then(|p| self.decrypt_password(p).ok())
                    .ok_or_else(|| {
                        AppError::Internal("No password stored for database".to_string())
                    })?;
                let config_path = self
                    .get_postgres_config_path(&container_id, &database.username, &password)
                    .await?;
                let content = self
                    .read_file_from_container(&container_id, &config_path)
                    .await?;
                (ConfigFormat::File, ConfigSource::File, content)
            },
            "redis" | "valkey" => {
                let password = database
                    .password_encrypted
                    .as_ref()
                    .and_then(|p| self.decrypt_password(p).ok())
                    .ok_or_else(|| {
                        AppError::Internal("No password stored for database".to_string())
                    })?;
                let content = self
                    .fetch_kv_config_from_container(
                        &container_id,
                        database.database_type.as_str(),
                        &password,
                        &mut warnings,
                    )
                    .await?;
                (ConfigFormat::Kv, ConfigSource::Runtime, content)
            },
            _ => {
                return Err(AppError::Validation(
                    "Unsupported database type for config".to_string(),
                ));
            },
        };

        Ok(DatabaseConfigResponse {
            database_id: database.id,
            database_type: database.database_type,
            format,
            source,
            content,
            warnings,
            requires_restart,
        })
    }

    pub async fn update_config(
        &self,
        database_id: &str,
        user_id: &str,
        content: &str,
    ) -> AppResult<UpdateDatabaseConfigResponse> {
        if !self.check_access(database_id, user_id).await? {
            return Err(AppError::Forbidden);
        }

        let database = self
            .database_repo
            .find_by_id(database_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", database_id)))?;

        let mut warnings = Vec::new();
        let mut requires_restart = false;

        let container_id = database
            .container_id
            .as_ref()
            .ok_or_else(|| AppError::NotFound("Database has no container".to_string()))?;
        let status = self.docker.get_container_status(container_id).await?;
        if status != "running" {
            return Err(AppError::Conflict(format!(
                "Container is not running (status: {})",
                status
            )));
        }

        let applied = match database.database_type.as_str() {
            "postgres" => {
                requires_restart = true;
                let password = database
                    .password_encrypted
                    .as_ref()
                    .and_then(|p| self.decrypt_password(p).ok())
                    .ok_or_else(|| {
                        AppError::Internal("No password stored for database".to_string())
                    })?;
                let config_path = self
                    .get_postgres_config_path(container_id, &database.username, &password)
                    .await?;
                self.write_file_in_container(container_id, &config_path, content)
                    .await?;
                let output = self
                    .reload_postgres_config(container_id, &database.username, &password)
                    .await?;
                if !output {
                    warnings.push("Config saved, but reload did not report success.".to_string());
                }
                output
            },
            "redis" | "valkey" => {
                let password = database
                    .password_encrypted
                    .as_ref()
                    .and_then(|p| self.decrypt_password(p).ok())
                    .ok_or_else(|| {
                        AppError::Internal("No password stored for database".to_string())
                    })?;
                self.apply_kv_config(
                    container_id,
                    database.database_type.as_str(),
                    &password,
                    content,
                    &mut warnings,
                )
                .await?;
                let rewritten = self
                    .rewrite_kv_config(container_id, database.database_type.as_str(), &password)
                    .await?;
                if !rewritten {
                    warnings.push(
                        "CONFIG REWRITE did not succeed. Changes may not persist after restart."
                            .to_string(),
                    );
                }
                true
            },
            _ => {
                return Err(AppError::Validation(
                    "Unsupported database type for config".to_string(),
                ));
            },
        };

        Ok(UpdateDatabaseConfigResponse {
            database_id: database.id,
            database_type: database.database_type,
            applied,
            warnings,
            requires_restart,
        })
    }

    async fn reload_postgres_config(
        &self,
        container_id: &str,
        username: &str,
        password: &str,
    ) -> AppResult<bool> {
        let output = self
            .docker
            .run_exec(
                container_id,
                vec![
                    "psql".to_string(),
                    "-U".to_string(),
                    username.to_string(),
                    "-d".to_string(),
                    "postgres".to_string(),
                    "-t".to_string(),
                    "-A".to_string(),
                    "-c".to_string(),
                    "SELECT pg_reload_conf();".to_string(),
                ],
                Some(vec![format!("PGPASSWORD={}", password)]),
            )
            .await?;

        if let Some(code) = output.exit_code {
            if code != 0 {
                return Err(AppError::Docker(format!(
                    "Failed to reload PostgreSQL config: {}",
                    output.stderr.trim()
                )));
            }
        }

        Ok(output.stdout.trim() == "t")
    }

    async fn read_file_from_container(&self, container_id: &str, path: &str) -> AppResult<String> {
        let output = self
            .docker
            .run_exec(
                container_id,
                vec!["cat".to_string(), path.to_string()],
                None,
            )
            .await?;

        if let Some(code) = output.exit_code {
            if code != 0 {
                return Err(AppError::Docker(format!(
                    "Failed to read config file: {}",
                    output.stderr.trim()
                )));
            }
        }

        Ok(output.stdout)
    }

    async fn write_file_in_container(
        &self,
        container_id: &str,
        path: &str,
        content: &str,
    ) -> AppResult<()> {
        let encoded = general_purpose::STANDARD.encode(content);
        let encoded_escaped = Self::shell_escape_single(&encoded);
        let path_escaped = Self::shell_escape_single(path);
        let command = format!(
            "printf '%s' '{}' | base64 -d > '{}'",
            encoded_escaped, path_escaped
        );

        let output = self
            .docker
            .run_exec(
                container_id,
                vec!["sh".to_string(), "-c".to_string(), command],
                None,
            )
            .await?;

        if let Some(code) = output.exit_code {
            if code != 0 {
                return Err(AppError::Docker(format!(
                    "Failed to write config file: {}",
                    output.stderr.trim()
                )));
            }
        }

        Ok(())
    }

    async fn get_postgres_config_path(
        &self,
        container_id: &str,
        username: &str,
        password: &str,
    ) -> AppResult<String> {
        let output = self
            .docker
            .run_exec(
                container_id,
                vec![
                    "psql".to_string(),
                    "-U".to_string(),
                    username.to_string(),
                    "-d".to_string(),
                    "postgres".to_string(),
                    "-t".to_string(),
                    "-A".to_string(),
                    "-c".to_string(),
                    "SHOW config_file;".to_string(),
                ],
                Some(vec![format!("PGPASSWORD={}", password)]),
            )
            .await?;

        if let Some(code) = output.exit_code {
            if code != 0 {
                return Err(AppError::Docker(format!(
                    "Failed to read PostgreSQL config path: {}",
                    output.stderr.trim()
                )));
            }
        }

        let path = output.stdout.trim().to_string();
        if path.is_empty() {
            return Err(AppError::Docker(
                "PostgreSQL returned empty config path".to_string(),
            ));
        }

        Ok(path)
    }

    async fn rewrite_kv_config(
        &self,
        container_id: &str,
        database_type: &str,
        password: &str,
    ) -> AppResult<bool> {
        let cli = if database_type == "valkey" {
            "valkey-cli"
        } else {
            "redis-cli"
        };

        let output = self
            .docker
            .run_exec(
                container_id,
                vec![
                    cli.to_string(),
                    "-a".to_string(),
                    password.to_string(),
                    "CONFIG".to_string(),
                    "REWRITE".to_string(),
                ],
                None,
            )
            .await?;

        if let Some(code) = output.exit_code {
            if code != 0 {
                return Ok(false);
            }
        }

        Ok(output.stdout.trim().eq_ignore_ascii_case("ok"))
    }

    async fn fetch_kv_config_from_container(
        &self,
        container_id: &str,
        database_type: &str,
        password: &str,
        warnings: &mut Vec<String>,
    ) -> AppResult<String> {
        let cli = if database_type == "valkey" {
            "valkey-cli"
        } else {
            "redis-cli"
        };

        let output = self
            .docker
            .run_exec(
                container_id,
                vec![
                    cli.to_string(),
                    "-a".to_string(),
                    password.to_string(),
                    "--raw".to_string(),
                    "CONFIG".to_string(),
                    "GET".to_string(),
                    "*".to_string(),
                ],
                None,
            )
            .await?;

        if let Some(code) = output.exit_code {
            if code != 0 {
                return Err(AppError::Docker(format!(
                    "Failed to fetch config: {}",
                    output.stderr.trim()
                )));
            }
        }

        let lines: Vec<&str> = output.stdout.lines().collect();
        let mut content = String::new();
        let mut skipped = Vec::new();

        for pair in lines.chunks(2) {
            if pair.len() < 2 {
                continue;
            }
            let key = pair[0].trim();
            let value = pair[1].trim();
            if key.is_empty() {
                continue;
            }
            if KV_SENSITIVE_KEYS.contains(&key) {
                skipped.push(key.to_string());
                continue;
            }
            content.push_str(key);
            if !value.is_empty() {
                content.push(' ');
                content.push_str(value);
            }
            content.push('\n');
        }

        if !skipped.is_empty() {
            warnings.push(format!("Sensitive keys hidden: {}", skipped.join(", ")));
        }

        Ok(content)
    }

    async fn apply_kv_config(
        &self,
        container_id: &str,
        database_type: &str,
        password: &str,
        content: &str,
        warnings: &mut Vec<String>,
    ) -> AppResult<()> {
        let entries = Self::parse_kv_config(content)?;
        let cli = if database_type == "valkey" {
            "valkey-cli"
        } else {
            "redis-cli"
        };

        let mut skipped = Vec::new();

        for entry in entries {
            if KV_SENSITIVE_KEYS.contains(&entry.key.as_str()) {
                skipped.push(entry.key);
                continue;
            }

            if entry.values.is_empty() {
                return Err(AppError::Validation(format!(
                    "Config key '{}' must have a value",
                    entry.key
                )));
            }

            let mut cmd = vec![
                cli.to_string(),
                "-a".to_string(),
                password.to_string(),
                "CONFIG".to_string(),
                "SET".to_string(),
                entry.key.clone(),
            ];
            cmd.extend(entry.values.clone());

            let output = self.docker.run_exec(container_id, cmd, None).await?;
            if let Some(code) = output.exit_code {
                if code != 0 {
                    return Err(AppError::Docker(format!(
                        "Failed to apply config '{}': {}",
                        entry.key,
                        output.stderr.trim()
                    )));
                }
            }
            let response = output.stdout.trim();
            if !response.eq_ignore_ascii_case("ok") {
                return Err(AppError::Docker(format!(
                    "Failed to apply config '{}': {}",
                    entry.key, response
                )));
            }
        }

        if !skipped.is_empty() {
            warnings.push(format!("Ignored sensitive keys: {}", skipped.join(", ")));
        }

        Ok(())
    }

    fn parse_kv_config(content: &str) -> AppResult<Vec<KvConfigEntry>> {
        let mut entries = Vec::new();

        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
                continue;
            }
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            if parts.len() < 2 {
                return Err(AppError::Validation(format!(
                    "Invalid config at line {}: missing value",
                    idx + 1
                )));
            }
            let key = parts[0].to_string();
            let values = parts[1..].iter().map(|v| v.to_string()).collect();
            entries.push(KvConfigEntry { key, values });
        }

        Ok(entries)
    }

    fn shell_escape_single(value: &str) -> String {
        value.replace('\'', "'\"'\"'")
    }
}

#[derive(Debug, Clone)]
struct KvConfigEntry {
    key: String,
    values: Vec<String>,
}

fn generate_password() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..24)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
