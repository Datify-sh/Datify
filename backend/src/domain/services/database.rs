use std::sync::Arc;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngCore;

use crate::domain::models::{BranchResponse, Database, DatabaseResponse, PostgresVersion, ValkeyVersion};
use crate::error::{AppError, AppResult};
use crate::infrastructure::docker::{ContainerConfig, DockerManager};
use crate::repositories::{DatabaseRepository, ProjectRepository};

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

    pub async fn create(
        &self,
        project_id: &str,
        user_id: &str,
        name: &str,
        database_type: &str,
        postgres_version: &str,
        valkey_version: Option<&str>,
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

        if is_valkey {
            let version = valkey_version.unwrap_or("8.0");
            if !ValkeyVersion::is_valid(version) {
                return Err(AppError::Validation(
                    "Invalid Valkey version".to_string(),
                ));
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
                cpu_limit,
                memory_limit_mb,
                storage_limit_mb,
                "main",
                true,
                None,
            )
            .await?;

        let password = password.map(|p| p.to_string()).unwrap_or_else(generate_password);

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
            self.docker.create_valkey_container(config, &password).await?
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
            self.docker.create_postgres_container(config, &password).await?
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
            return Err(AppError::Validation("Current password is incorrect".to_string()));
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

        if source.database_type == "valkey" {
            return Err(AppError::Validation(
                "Branching is not supported for Valkey databases".to_string(),
            ));
        }

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

        let container_id = self
            .docker
            .create_postgres_container(config, &password)
            .await?;
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

            // Wait for Docker DNS to propagate and container to be ready for connections
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            if let Err(e) = self
                .docker
                .fork_database(
                    &source_container,
                    &container_name,
                    &source.username,
                    &source_password,
                    &branch.username,
                    &password,
                )
                .await
            {
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

        self.docker
            .fork_database(
                &parent_container,
                &branch_container,
                &parent.username,
                &parent_password,
                &branch.username,
                &branch_password,
            )
            .await?;

        self.database_repo.update_forked_at(database_id).await?;

        self.get_by_id_response(database_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", database_id)))
    }
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
