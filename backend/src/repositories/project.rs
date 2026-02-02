use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use crate::domain::models::Project;
use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct ProjectRepository {
    pool: SqlitePool,
}

impl ProjectRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        user_id: &str,
        name: &str,
        slug: &str,
        description: Option<&str>,
        settings: Option<&str>,
    ) -> AppResult<Project> {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO projects (id, user_id, name, slug, description, settings)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(settings.unwrap_or("{}"))
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                AppError::AlreadyExists(format!("Project with slug '{}' already exists", slug))
            } else {
                AppError::Database(e)
            }
        })?;

        self.find_by_id(&id)
            .await?
            .ok_or_else(|| AppError::Internal("Failed to retrieve created project".to_string()))
    }

    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<Project>> {
        let project = sqlx::query_as::<_, Project>(r#"SELECT * FROM projects WHERE id = ?"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(project)
    }

    pub async fn find_by_slug(&self, slug: &str) -> AppResult<Option<Project>> {
        let project = sqlx::query_as::<_, Project>(r#"SELECT * FROM projects WHERE slug = ?"#)
            .bind(slug)
            .fetch_optional(&self.pool)
            .await?;

        Ok(project)
    }

    pub async fn find_by_user_id(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<Project>> {
        let projects = sqlx::query_as::<_, Project>(
            r#"
            SELECT * FROM projects
            WHERE user_id = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(projects)
    }

    pub async fn find_all(&self, limit: i64, offset: i64) -> AppResult<Vec<Project>> {
        let projects = sqlx::query_as::<_, Project>(
            r#"
            SELECT * FROM projects
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(projects)
    }

    pub async fn update(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        settings: Option<&str>,
    ) -> AppResult<Project> {
        if name.is_none() && description.is_none() && settings.is_none() {
            return self
                .find_by_id(id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("Project with id '{}' not found", id)));
        }

        let result = match (name, description, settings) {
            (Some(n), None, None) => {
                sqlx::query("UPDATE projects SET name = ? WHERE id = ?")
                    .bind(n)
                    .bind(id)
                    .execute(&self.pool)
                    .await?
            },
            (None, Some(d), None) => {
                sqlx::query("UPDATE projects SET description = ? WHERE id = ?")
                    .bind(d)
                    .bind(id)
                    .execute(&self.pool)
                    .await?
            },
            (None, None, Some(s)) => {
                sqlx::query("UPDATE projects SET settings = ? WHERE id = ?")
                    .bind(s)
                    .bind(id)
                    .execute(&self.pool)
                    .await?
            },
            (Some(n), Some(d), None) => {
                sqlx::query("UPDATE projects SET name = ?, description = ? WHERE id = ?")
                    .bind(n)
                    .bind(d)
                    .bind(id)
                    .execute(&self.pool)
                    .await?
            },
            (Some(n), None, Some(s)) => {
                sqlx::query("UPDATE projects SET name = ?, settings = ? WHERE id = ?")
                    .bind(n)
                    .bind(s)
                    .bind(id)
                    .execute(&self.pool)
                    .await?
            },
            (None, Some(d), Some(s)) => {
                sqlx::query("UPDATE projects SET description = ?, settings = ? WHERE id = ?")
                    .bind(d)
                    .bind(s)
                    .bind(id)
                    .execute(&self.pool)
                    .await?
            },
            (Some(n), Some(d), Some(s)) => {
                sqlx::query(
                    "UPDATE projects SET name = ?, description = ?, settings = ? WHERE id = ?",
                )
                .bind(n)
                .bind(d)
                .bind(s)
                .bind(id)
                .execute(&self.pool)
                .await?
            },
            (None, None, None) => unreachable!(),
        };

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Project with id '{}' not found",
                id
            )));
        }

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal("Failed to retrieve updated project".to_string()))
    }

    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let result = sqlx::query(r#"DELETE FROM projects WHERE id = ?"#)
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Project with id '{}' not found",
                id
            )));
        }

        Ok(())
    }

    pub async fn count_by_user(&self, user_id: &str) -> AppResult<i64> {
        let count: (i64,) = sqlx::query_as(r#"SELECT COUNT(*) FROM projects WHERE user_id = ?"#)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0)
    }

    pub async fn count_all(&self) -> AppResult<i64> {
        let count: (i64,) = sqlx::query_as(r#"SELECT COUNT(*) FROM projects"#)
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0)
    }

    pub async fn is_owner(&self, project_id: &str, user_id: &str) -> AppResult<bool> {
        let project = self.find_by_id(project_id).await?;
        Ok(project.map(|p| p.user_id == user_id).unwrap_or(false))
    }
}
