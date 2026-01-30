use std::sync::Arc;

use chrono::Utc;

use super::postgres::PostgresMetricsCollector;
use super::redis::RedisMetricsCollector;
use crate::domain::models::{
    Database, MetricsHistory, QueryLogsResponse, TimeRange, UnifiedMetrics, UnifiedMetricsResponse,
};
use crate::error::{AppError, AppResult};
use crate::infrastructure::docker::DockerManager;
use crate::repositories::{DatabaseRepository, MetricsRepository, ProjectRepository};

#[derive(Clone)]
pub struct MetricsService {
    database_repo: DatabaseRepository,
    project_repo: ProjectRepository,
    metrics_repo: MetricsRepository,
    docker: Arc<DockerManager>,
    postgres_collector: Arc<PostgresMetricsCollector>,
    redis_collector: Arc<RedisMetricsCollector>,
    valkey_collector: Arc<RedisMetricsCollector>,
}

impl MetricsService {
    pub fn new(
        database_repo: DatabaseRepository,
        project_repo: ProjectRepository,
        metrics_repo: MetricsRepository,
        docker: Arc<DockerManager>,
        encryption_key_hex: &str,
    ) -> Self {
        let encryption_key: [u8; 32] = hex::decode(encryption_key_hex)
            .expect("Invalid encryption key hex")
            .try_into()
            .expect("Encryption key must be 32 bytes");

        Self {
            database_repo,
            project_repo,
            metrics_repo,
            docker,
            postgres_collector: Arc::new(PostgresMetricsCollector::new(encryption_key)),
            redis_collector: Arc::new(RedisMetricsCollector::new(encryption_key, false)),
            valkey_collector: Arc::new(RedisMetricsCollector::new(encryption_key, true)),
        }
    }

    pub async fn check_access(&self, database_id: &str, user_id: &str) -> AppResult<bool> {
        let project_id = self
            .database_repo
            .get_project_id(database_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", database_id)))?;

        self.project_repo.is_owner(&project_id, user_id).await
    }

    pub async fn get_current_metrics(
        &self,
        database_id: &str,
    ) -> AppResult<UnifiedMetricsResponse> {
        let database = self
            .database_repo
            .find_by_id(database_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", database_id)))?;

        if database.container_status != "running" || database.container_id.is_none() {
            return Err(AppError::Validation(
                "Database must be running to get metrics".to_string(),
            ));
        }

        let metrics = self.collect_metrics(&database).await?;

        Ok(UnifiedMetricsResponse {
            database_id: database_id.to_string(),
            metrics,
        })
    }

    pub async fn get_metrics_history(
        &self,
        database_id: &str,
        time_range: TimeRange,
    ) -> AppResult<MetricsHistory> {
        let database = self
            .database_repo
            .find_by_id(database_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", database_id)))?;

        let points = self
            .metrics_repo
            .get_history(database_id, time_range)
            .await?;

        let now = Utc::now();
        let duration = chrono::Duration::seconds(time_range.duration_secs());
        let start = now - duration;

        Ok(MetricsHistory {
            database_id: database.id,
            time_range,
            start_time: start.to_rfc3339(),
            end_time: now.to_rfc3339(),
            points,
        })
    }

    pub async fn get_query_logs(
        &self,
        database_id: &str,
        limit: i32,
        sort_by: &str,
    ) -> AppResult<QueryLogsResponse> {
        let database = self
            .database_repo
            .find_by_id(database_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", database_id)))?;

        if database.container_status != "running" {
            return Err(AppError::Validation(
                "Database must be running to get query logs".to_string(),
            ));
        }

        if database.database_type != "postgres" {
            return Err(AppError::Validation(
                "Query logs are only available for PostgreSQL databases".to_string(),
            ));
        }

        let entries = self
            .postgres_collector
            .get_query_logs(&database, limit, sort_by)
            .await?;

        let total_queries = self.postgres_collector.count_query_logs(&database).await?;

        Ok(QueryLogsResponse {
            database_id: database_id.to_string(),
            entries,
            total_queries,
        })
    }

    pub async fn snapshot_metrics(&self, database_id: &str) -> AppResult<()> {
        let database = self
            .database_repo
            .find_by_id(database_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", database_id)))?;

        if database.container_status != "running" || database.container_id.is_none() {
            return Ok(());
        }

        let metrics = self.collect_metrics(&database).await?;
        self.metrics_repo
            .save_snapshot(database_id, &database.database_type, &metrics)
            .await?;

        Ok(())
    }

    async fn collect_metrics(&self, database: &Database) -> AppResult<UnifiedMetrics> {
        let container_id = database
            .container_id
            .as_ref()
            .ok_or_else(|| AppError::Internal("Database has no container".to_string()))?;

        let docker_stats = self.docker.get_container_stats(container_id).await?;

        match database.database_type.as_str() {
            "postgres" => {
                self.postgres_collector
                    .collect_metrics(database, &docker_stats)
                    .await
            },
            "redis" => {
                self.redis_collector
                    .collect_metrics(database, &docker_stats)
                    .await
            },
            "valkey" => {
                self.valkey_collector
                    .collect_metrics(database, &docker_stats)
                    .await
            },
            _ => {
                self.postgres_collector
                    .collect_metrics(database, &docker_stats)
                    .await
            },
        }
    }
}
