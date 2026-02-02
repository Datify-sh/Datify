use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use crate::domain::models::{AuditLog, AuditLogFilter, AuditLogWithUser, CreateAuditLog};
use crate::error::AppResult;

#[derive(Clone)]
pub struct AuditLogRepository {
    pool: SqlitePool,
}

impl AuditLogRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, input: CreateAuditLog) -> AppResult<AuditLog> {
        let id = Uuid::new_v4().to_string();
        let changes = input.changes.map(|v| v.to_string());

        sqlx::query(
            r#"
            INSERT INTO audit_logs (id, user_id, action, entity_type, entity_id, changes, status, ip_address, user_agent)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&input.user_id)
        .bind(input.action.to_string())
        .bind(input.entity_type.to_string())
        .bind(&input.entity_id)
        .bind(&changes)
        .bind(input.status.to_string())
        .bind(&input.ip_address)
        .bind(&input.user_agent)
        .execute(&self.pool)
        .await?;

        let log = sqlx::query_as::<_, AuditLog>(r#"SELECT * FROM audit_logs WHERE id = ?"#)
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;

        Ok(log)
    }

    pub async fn find_by_user(
        &self,
        user_id: &str,
        filter: &AuditLogFilter,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<AuditLogWithUser>> {
        let mut query = String::from(
            "SELECT a.*, u.email AS user_email FROM audit_logs a LEFT JOIN users u ON u.id = \
             a.user_id WHERE a.user_id = ?",
        );
        let mut binds: Vec<String> = vec![user_id.to_string()];

        self.apply_filters(&mut query, &mut binds, filter);

        query.push_str(" ORDER BY a.created_at DESC LIMIT ? OFFSET ?");

        let mut q = sqlx::query_as::<_, AuditLogWithUser>(&query);
        for bind in &binds {
            q = q.bind(bind);
        }
        q = q.bind(limit).bind(offset);

        let logs = q.fetch_all(&self.pool).await?;
        Ok(logs)
    }

    pub async fn find_all(
        &self,
        filter: &AuditLogFilter,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<AuditLogWithUser>> {
        let mut query = String::from(
            "SELECT a.*, u.email AS user_email FROM audit_logs a LEFT JOIN users u ON u.id = \
             a.user_id WHERE 1=1",
        );
        let mut binds: Vec<String> = vec![];

        self.apply_filters(&mut query, &mut binds, filter);

        query.push_str(" ORDER BY a.created_at DESC LIMIT ? OFFSET ?");

        let mut q = sqlx::query_as::<_, AuditLogWithUser>(&query);
        for bind in &binds {
            q = q.bind(bind);
        }
        q = q.bind(limit).bind(offset);

        let logs = q.fetch_all(&self.pool).await?;
        Ok(logs)
    }

    pub async fn count_by_user(&self, user_id: &str, filter: &AuditLogFilter) -> AppResult<i64> {
        let mut query = String::from("SELECT COUNT(*) FROM audit_logs a WHERE a.user_id = ?");
        let mut binds: Vec<String> = vec![user_id.to_string()];

        self.apply_filters(&mut query, &mut binds, filter);

        let mut q = sqlx::query_as::<_, (i64,)>(&query);
        for bind in &binds {
            q = q.bind(bind);
        }

        let (count,) = q.fetch_one(&self.pool).await?;
        Ok(count)
    }

    pub async fn count_all(&self, filter: &AuditLogFilter) -> AppResult<i64> {
        let mut query = String::from("SELECT COUNT(*) FROM audit_logs a WHERE 1=1");
        let mut binds: Vec<String> = vec![];

        self.apply_filters(&mut query, &mut binds, filter);

        let mut q = sqlx::query_as::<_, (i64,)>(&query);
        for bind in &binds {
            q = q.bind(bind);
        }

        let (count,) = q.fetch_one(&self.pool).await?;
        Ok(count)
    }

    fn apply_filters(&self, query: &mut String, binds: &mut Vec<String>, filter: &AuditLogFilter) {
        if let Some(action) = &filter.action {
            query.push_str(" AND a.action = ?");
            binds.push(action.clone());
        }

        if let Some(entity_type) = &filter.entity_type {
            query.push_str(" AND a.entity_type = ?");
            binds.push(entity_type.clone());
        }

        if let Some(status) = &filter.status {
            query.push_str(" AND a.status = ?");
            binds.push(status.clone());
        }

        if let Some(start_date) = &filter.start_date {
            query.push_str(" AND a.created_at >= ?");
            binds.push(start_date.clone());
        }

        if let Some(end_date) = &filter.end_date {
            query.push_str(" AND a.created_at <= ?");
            binds.push(end_date.clone());
        }
    }
}
