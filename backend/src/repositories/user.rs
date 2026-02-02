use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use crate::domain::models::User;
use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct UserRepository {
    pool: SqlitePool,
}

impl UserRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, email: &str, password_hash: &str, role: &str) -> AppResult<User> {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO users (id, email, password_hash, role)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(email)
        .bind(password_hash)
        .bind(role)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                AppError::AlreadyExists(format!("User with email '{}' already exists", email))
            } else {
                AppError::Database(e)
            }
        })?;

        self.find_by_id(&id)
            .await?
            .ok_or_else(|| AppError::Internal("Failed to retrieve created user".to_string()))
    }

    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<User>> {
        let user = sqlx::query_as::<_, User>(r#"SELECT * FROM users WHERE id = ?"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(user)
    }

    pub async fn find_by_email(&self, email: &str) -> AppResult<Option<User>> {
        let user = sqlx::query_as::<_, User>(r#"SELECT * FROM users WHERE email = ?"#)
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;

        Ok(user)
    }

    pub async fn update(
        &self,
        id: &str,
        email: Option<&str>,
        password_hash: Option<&str>,
        role: Option<&str>,
    ) -> AppResult<User> {
        if email.is_none() && password_hash.is_none() && role.is_none() {
            return self
                .find_by_id(id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("User with id '{}' not found", id)));
        }

        let mut query = "UPDATE users SET ".to_string();
        let mut updates = Vec::new();

        if email.is_some() {
            updates.push("email = ?");
        }
        if password_hash.is_some() {
            updates.push("password_hash = ?");
        }
        if role.is_some() {
            updates.push("role = ?");
        }

        query.push_str(&updates.join(", "));
        query.push_str(" WHERE id = ?");

        let mut query_builder = sqlx::query(&query);

        if let Some(e) = email {
            query_builder = query_builder.bind(e);
        }
        if let Some(p) = password_hash {
            query_builder = query_builder.bind(p);
        }
        if let Some(r) = role {
            query_builder = query_builder.bind(r);
        }

        query_builder = query_builder.bind(id);

        let result = query_builder.execute(&self.pool).await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "User with id '{}' not found",
                id
            )));
        }

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal("Failed to retrieve updated user".to_string()))
    }

    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let result = sqlx::query(r#"DELETE FROM users WHERE id = ?"#)
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "User with id '{}' not found",
                id
            )));
        }

        Ok(())
    }

    pub async fn list(&self, limit: i64, offset: i64) -> AppResult<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            r#"SELECT * FROM users ORDER BY created_at DESC LIMIT ? OFFSET ?"#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }

    pub async fn count(&self) -> AppResult<i64> {
        let count: (i64,) = sqlx::query_as(r#"SELECT COUNT(*) FROM users"#)
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0)
    }

    pub async fn verify_email(&self, id: &str) -> AppResult<()> {
        let result = sqlx::query(r#"UPDATE users SET email_verified = 1 WHERE id = ?"#)
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "User with id '{}' not found",
                id
            )));
        }

        Ok(())
    }
}
