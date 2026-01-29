use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use crate::domain::models::ApiKey;
use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct ApiKeyRepository {
    pool: SqlitePool,
}

impl ApiKeyRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        user_id: &str,
        name: &str,
        key_prefix: &str,
        key_hash: &str,
        scopes: &str,
        expires_at: Option<&str>,
    ) -> AppResult<ApiKey> {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO api_keys (id, user_id, name, key_prefix, key_hash, scopes, expires_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(name)
        .bind(key_prefix)
        .bind(key_hash)
        .bind(scopes)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        self.find_by_id(&id)
            .await?
            .ok_or_else(|| AppError::Internal("Failed to retrieve created API key".to_string()))
    }

    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<ApiKey>> {
        let key = sqlx::query_as::<_, ApiKey>(r#"SELECT * FROM api_keys WHERE id = ?"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(key)
    }

    pub async fn find_by_hash(&self, key_hash: &str) -> AppResult<Option<ApiKey>> {
        let key = sqlx::query_as::<_, ApiKey>(r#"SELECT * FROM api_keys WHERE key_hash = ?"#)
            .bind(key_hash)
            .fetch_optional(&self.pool)
            .await?;

        Ok(key)
    }

    pub async fn find_by_prefix(&self, key_prefix: &str) -> AppResult<Vec<ApiKey>> {
        let keys = sqlx::query_as::<_, ApiKey>(r#"SELECT * FROM api_keys WHERE key_prefix = ?"#)
            .bind(key_prefix)
            .fetch_all(&self.pool)
            .await?;

        Ok(keys)
    }

    pub async fn find_by_user_id(&self, user_id: &str) -> AppResult<Vec<ApiKey>> {
        let keys = sqlx::query_as::<_, ApiKey>(
            r#"
            SELECT * FROM api_keys
            WHERE user_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(keys)
    }

    pub async fn update_last_used(&self, id: &str) -> AppResult<()> {
        sqlx::query(r#"UPDATE api_keys SET last_used_at = datetime('now') WHERE id = ?"#)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let result = sqlx::query(r#"DELETE FROM api_keys WHERE id = ?"#)
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "API key with id '{}' not found",
                id
            )));
        }

        Ok(())
    }

    pub async fn delete_by_user_id(&self, user_id: &str) -> AppResult<u64> {
        let result = sqlx::query(r#"DELETE FROM api_keys WHERE user_id = ?"#)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    pub async fn delete_expired(&self) -> AppResult<u64> {
        let result = sqlx::query(
            r#"DELETE FROM api_keys WHERE expires_at IS NOT NULL AND expires_at < datetime('now')"#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
