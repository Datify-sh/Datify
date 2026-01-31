use crate::domain::models::{
    AuditAction, AuditEntityType, AuditLogFilter, AuditLogResponse, AuditStatus, CreateAuditLog,
};
use crate::error::AppResult;
use crate::repositories::AuditLogRepository;

#[derive(Clone)]
pub struct AuditLogService {
    audit_log_repo: AuditLogRepository,
}

impl AuditLogService {
    pub fn new(audit_log_repo: AuditLogRepository) -> Self {
        Self { audit_log_repo }
    }

    pub fn log(
        &self,
        user_id: String,
        action: AuditAction,
        entity_type: AuditEntityType,
        entity_id: Option<String>,
        changes: Option<serde_json::Value>,
        status: AuditStatus,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) {
        let repo = self.audit_log_repo.clone();
        let input = CreateAuditLog {
            user_id,
            action,
            entity_type,
            entity_id,
            changes,
            status,
            ip_address,
            user_agent,
        };

        tokio::spawn(async move {
            if let Err(e) = repo.create(input).await {
                tracing::error!("Failed to create audit log: {}", e);
            }
        });
    }

    pub async fn list(
        &self,
        user_id: &str,
        is_admin: bool,
        filter: &AuditLogFilter,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<AuditLogResponse>> {
        let logs = if is_admin {
            self.audit_log_repo.find_all(filter, limit, offset).await?
        } else {
            self.audit_log_repo
                .find_by_user(user_id, filter, limit, offset)
                .await?
        };

        Ok(logs.into_iter().map(|log| log.to_response()).collect())
    }

    pub async fn count(
        &self,
        user_id: &str,
        is_admin: bool,
        filter: &AuditLogFilter,
    ) -> AppResult<i64> {
        if is_admin {
            self.audit_log_repo.count_all(filter).await
        } else {
            self.audit_log_repo.count_by_user(user_id, filter).await
        }
    }
}
