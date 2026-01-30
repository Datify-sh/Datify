use chrono::Utc;
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use crate::domain::models::Database;
use crate::error::{AppError, AppResult};

const DATABASE_COLUMNS: &str = r#"
    id, project_id, name, database_type, postgres_version, valkey_version, redis_version,
    container_id, container_status, host, port, username, password_encrypted,
    cpu_limit, memory_limit_mb, storage_limit_mb, public_exposed,
    created_at, updated_at, parent_branch_id, branch_name, is_default_branch, forked_at
"#;

#[derive(Clone)]
pub struct DatabaseRepository {
    pool: SqlitePool,
}

impl DatabaseRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        project_id: &str,
        name: &str,
        database_type: &str,
        postgres_version: &str,
        valkey_version: Option<&str>,
        redis_version: Option<&str>,
        cpu_limit: f64,
        memory_limit_mb: i32,
        storage_limit_mb: i32,
        branch_name: &str,
        is_default_branch: bool,
        parent_branch_id: Option<&str>,
    ) -> AppResult<Database> {
        let id = Uuid::new_v4().to_string();
        let forked_at = if parent_branch_id.is_some() {
            Some(Utc::now().to_rfc3339())
        } else {
            None
        };

        sqlx::query(
            r#"
            INSERT INTO databases (id, project_id, name, database_type, postgres_version, valkey_version, redis_version, cpu_limit, memory_limit_mb, storage_limit_mb, branch_name, is_default_branch, parent_branch_id, forked_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(name)
        .bind(database_type)
        .bind(postgres_version)
        .bind(valkey_version)
        .bind(redis_version)
        .bind(cpu_limit)
        .bind(memory_limit_mb)
        .bind(storage_limit_mb)
        .bind(branch_name)
        .bind(is_default_branch)
        .bind(parent_branch_id)
        .bind(&forked_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                AppError::AlreadyExists(format!(
                    "Database with name '{}' already exists in this project",
                    name
                ))
            } else {
                AppError::Database(e)
            }
        })?;

        self.find_by_id(&id)
            .await?
            .ok_or_else(|| AppError::Internal("Failed to retrieve created database".to_string()))
    }

    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<Database>> {
        let query = format!("SELECT {} FROM databases WHERE id = ?", DATABASE_COLUMNS);
        let database = sqlx::query_as::<_, Database>(&query)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(database)
    }

    pub async fn find_by_project_id(
        &self,
        project_id: &str,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<Database>> {
        let query = format!(
            "SELECT {} FROM databases WHERE project_id = ? ORDER BY created_at DESC LIMIT ? \
             OFFSET ?",
            DATABASE_COLUMNS
        );
        let databases = sqlx::query_as::<_, Database>(&query)
            .bind(project_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        Ok(databases)
    }

    pub async fn find_by_name_and_project(
        &self,
        project_id: &str,
        name: &str,
    ) -> AppResult<Option<Database>> {
        let query = format!(
            "SELECT {} FROM databases WHERE project_id = ? AND name = ?",
            DATABASE_COLUMNS
        );
        let database = sqlx::query_as::<_, Database>(&query)
            .bind(project_id)
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        Ok(database)
    }

    pub async fn update(
        &self,
        id: &str,
        name: Option<&str>,
        cpu_limit: Option<f64>,
        memory_limit_mb: Option<i32>,
        storage_limit_mb: Option<i32>,
        public_exposed: Option<bool>,
    ) -> AppResult<Database> {
        let current = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database with id '{}' not found", id)))?;

        let new_name = name.unwrap_or(&current.name);
        let new_cpu = cpu_limit.unwrap_or(current.cpu_limit);
        let new_memory = memory_limit_mb.unwrap_or(current.memory_limit_mb);
        let new_storage = storage_limit_mb.unwrap_or(current.storage_limit_mb);
        let new_public = public_exposed.unwrap_or(current.public_exposed);

        sqlx::query(
            r#"
            UPDATE databases
            SET name = ?, cpu_limit = ?, memory_limit_mb = ?, storage_limit_mb = ?, public_exposed = ?
            WHERE id = ?
            "#,
        )
        .bind(new_name)
        .bind(new_cpu)
        .bind(new_memory)
        .bind(new_storage)
        .bind(new_public)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database with id '{}' not found", id)))
    }

    pub async fn update_container(
        &self,
        id: &str,
        container_id: &str,
        status: &str,
        host: &str,
        port: i32,
        password_encrypted: &str,
    ) -> AppResult<Database> {
        let result = sqlx::query(
            r#"
            UPDATE databases
            SET container_id = ?, container_status = ?, host = ?, port = ?, password_encrypted = ?
            WHERE id = ?
            "#,
        )
        .bind(container_id)
        .bind(status)
        .bind(host)
        .bind(port)
        .bind(password_encrypted)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Database with id '{}' not found",
                id
            )));
        }

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database with id '{}' not found", id)))
    }

    pub async fn update_status(&self, id: &str, status: &str) -> AppResult<()> {
        let result = sqlx::query(r#"UPDATE databases SET container_status = ? WHERE id = ?"#)
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Database with id '{}' not found",
                id
            )));
        }

        Ok(())
    }

    pub async fn update_password(&self, id: &str, password_encrypted: &str) -> AppResult<()> {
        let result = sqlx::query(
            r#"UPDATE databases SET password_encrypted = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"#,
        )
        .bind(password_encrypted)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Database with id '{}' not found",
                id
            )));
        }

        Ok(())
    }

    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let result = sqlx::query(r#"DELETE FROM databases WHERE id = ?"#)
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Database with id '{}' not found",
                id
            )));
        }

        Ok(())
    }

    pub async fn count_by_project(&self, project_id: &str) -> AppResult<i64> {
        let count: (i64,) =
            sqlx::query_as(r#"SELECT COUNT(*) FROM databases WHERE project_id = ?"#)
                .bind(project_id)
                .fetch_one(&self.pool)
                .await?;

        Ok(count.0)
    }

    pub async fn get_project_id(&self, database_id: &str) -> AppResult<Option<String>> {
        let result: Option<(String,)> =
            sqlx::query_as(r#"SELECT project_id FROM databases WHERE id = ?"#)
                .bind(database_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(result.map(|(id,)| id))
    }

    pub async fn get_next_available_port(&self) -> AppResult<i32> {
        let result: Option<(Option<i32>,)> =
            sqlx::query_as(r#"SELECT MAX(port) FROM databases WHERE port IS NOT NULL"#)
                .fetch_optional(&self.pool)
                .await?;

        let max_port = result.and_then(|(p,)| p).unwrap_or(5432);
        Ok(max_port + 1)
    }

    pub async fn find_all_running(&self) -> AppResult<Vec<Database>> {
        let query = format!(
            "SELECT {} FROM databases WHERE container_status = 'running' ORDER BY created_at DESC",
            DATABASE_COLUMNS
        );
        let databases = sqlx::query_as::<_, Database>(&query)
            .fetch_all(&self.pool)
            .await?;

        Ok(databases)
    }

    pub async fn find_branches(&self, database_id: &str) -> AppResult<Vec<Database>> {
        let root = self.find_root_database(database_id).await?;
        let root_id = root
            .map(|d| d.id)
            .unwrap_or_else(|| database_id.to_string());

        let query = format!(
            "SELECT {} FROM databases WHERE id = ? OR parent_branch_id = ? ORDER BY \
             is_default_branch DESC, created_at ASC",
            DATABASE_COLUMNS
        );
        let databases = sqlx::query_as::<_, Database>(&query)
            .bind(&root_id)
            .bind(&root_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(databases)
    }

    pub async fn find_root_database(&self, database_id: &str) -> AppResult<Option<Database>> {
        let mut current_id = database_id.to_string();

        loop {
            let database = self.find_by_id(&current_id).await?;

            match database {
                Some(db) => match db.parent_branch_id {
                    None => return Ok(Some(db)),
                    Some(parent_id) => current_id = parent_id,
                },
                None => return Ok(None),
            }
        }
    }

    pub async fn find_children(&self, database_id: &str) -> AppResult<Vec<Database>> {
        let query = format!(
            "SELECT {} FROM databases WHERE parent_branch_id = ? ORDER BY created_at ASC",
            DATABASE_COLUMNS
        );
        let databases = sqlx::query_as::<_, Database>(&query)
            .bind(database_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(databases)
    }

    pub async fn update_forked_at(&self, id: &str) -> AppResult<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(r#"UPDATE databases SET forked_at = ? WHERE id = ?"#)
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
