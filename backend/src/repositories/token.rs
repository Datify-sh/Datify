use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use crate::error::AppResult;

#[derive(Clone)]
pub struct TokenRepository {
    pool: SqlitePool,
}

impl TokenRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn revoke_token(&self, jti: &str, user_id: &str, expires_at: &str) -> AppResult<()> {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT OR IGNORE INTO revoked_tokens (id, token_jti, user_id, expires_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(jti)
        .bind(user_id)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn is_revoked(&self, jti: &str) -> AppResult<bool> {
        let result: Option<(i32,)> =
            sqlx::query_as(r#"SELECT 1 FROM revoked_tokens WHERE token_jti = ?"#)
                .bind(jti)
                .fetch_optional(&self.pool)
                .await?;

        Ok(result.is_some())
    }

    pub async fn revoke_all_user_tokens(&self, user_id: &str) -> AppResult<u64> {
        let result = sqlx::query(
            r#"
            INSERT INTO revoked_tokens (id, token_jti, user_id, expires_at)
            SELECT
                lower(hex(randomblob(16))),
                'all_' || ? || '_' || strftime('%s', 'now'),
                ?,
                datetime('now', '+7 days')
            WHERE NOT EXISTS (
                SELECT 1 FROM revoked_tokens
                WHERE user_id = ? AND token_jti LIKE 'all_%'
            )
            "#,
        )
        .bind(user_id)
        .bind(user_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn get_user_revocation_timestamp(&self, user_id: &str) -> AppResult<Option<String>> {
        let result: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT revoked_at FROM revoked_tokens
            WHERE user_id = ? AND token_jti LIKE 'all_%'
            ORDER BY revoked_at DESC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|r| r.0))
    }

    pub async fn cleanup_expired(&self) -> AppResult<u64> {
        let result =
            sqlx::query(r#"DELETE FROM revoked_tokens WHERE expires_at < datetime('now')"#)
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected())
    }
}
