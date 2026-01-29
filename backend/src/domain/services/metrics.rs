use std::sync::Arc;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use chrono::Utc;
use tokio_postgres::{Client, NoTls};

use crate::domain::models::{
    ConnectionMetrics, Database, DatabaseMetrics, MetricsHistory, MetricsResponse, QueryLogEntry,
    QueryLogsResponse, QueryMetrics, ResourceMetrics, RowMetrics, StorageMetrics, TableMetrics,
    TimeRange,
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
    encryption_key: [u8; 32],
}

impl MetricsService {
    pub fn new(
        database_repo: DatabaseRepository,
        project_repo: ProjectRepository,
        metrics_repo: MetricsRepository,
        docker: Arc<DockerManager>,
        encryption_key_hex: &str,
    ) -> Self {
        let encryption_key = hex::decode(encryption_key_hex)
            .expect("Invalid encryption key hex")
            .try_into()
            .expect("Encryption key must be 32 bytes");

        Self {
            database_repo,
            project_repo,
            metrics_repo,
            docker,
            encryption_key,
        }
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

    pub async fn check_access(&self, database_id: &str, user_id: &str) -> AppResult<bool> {
        let project_id = self
            .database_repo
            .get_project_id(database_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", database_id)))?;

        self.project_repo.is_owner(&project_id, user_id).await
    }

    pub async fn get_current_metrics(&self, database_id: &str) -> AppResult<MetricsResponse> {
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

        Ok(MetricsResponse {
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

        let client = self.connect_to_database(&database).await?;

        let extension_exists = client
            .query_one(
                "SELECT 1 FROM pg_extension WHERE extname = 'pg_stat_statements'",
                &[],
            )
            .await
            .is_ok();

        if !extension_exists {
            let _ = client
                .execute("CREATE EXTENSION IF NOT EXISTS pg_stat_statements", &[])
                .await;
            return Ok(QueryLogsResponse {
                database_id: database_id.to_string(),
                entries: vec![],
                total_queries: 0,
            });
        }

        let order_by = match sort_by {
            "avg_time" => "mean_exec_time DESC",
            "calls" => "calls DESC",
            _ => "total_exec_time DESC",
        };

        let query = format!(
            r#"
            SELECT
                query,
                calls,
                total_exec_time as total_time_ms,
                mean_exec_time as avg_time_ms,
                min_exec_time as min_time_ms,
                max_exec_time as max_time_ms,
                rows,
                CASE WHEN calls > 0 THEN rows::float8 / calls ELSE 0 END as rows_per_call
            FROM pg_stat_statements
            WHERE userid = (SELECT usesysid FROM pg_user WHERE usename = current_user)
              AND query NOT LIKE '%pg_stat_statements%'
              AND query NOT LIKE '%pg_stat_user_tables%'
              AND query NOT LIKE '%pg_stat_user_indexes%'
              AND query NOT LIKE '%pg_stat_activity%'
              AND query NOT LIKE '%pg_database_size%'
              AND query NOT LIKE '%pg_settings%'
            ORDER BY {}
            LIMIT $1
            "#,
            order_by
        );

        let rows = client
            .query(&query, &[&(limit as i64)])
            .await
            .map_err(|e| {
                AppError::Internal(format!("Failed to query pg_stat_statements: {}", e))
            })?;

        let entries: Vec<QueryLogEntry> = rows
            .iter()
            .map(|row| QueryLogEntry {
                query: truncate_query(row.get::<_, &str>("query")),
                calls: row.get::<_, i64>("calls"),
                total_time_ms: row.get::<_, f64>("total_time_ms"),
                avg_time_ms: row.get::<_, f64>("avg_time_ms"),
                min_time_ms: row.get::<_, f64>("min_time_ms"),
                max_time_ms: row.get::<_, f64>("max_time_ms"),
                rows: row.get::<_, i64>("rows"),
                rows_per_call: row.get::<_, f64>("rows_per_call"),
            })
            .collect();

        let count_row = client
            .query_one(
                r#"
                SELECT COUNT(*) FROM pg_stat_statements
                WHERE userid = (SELECT usesysid FROM pg_user WHERE usename = current_user)
                  AND query NOT LIKE '%pg_stat_statements%'
                  AND query NOT LIKE '%pg_stat_user_tables%'
                  AND query NOT LIKE '%pg_stat_user_indexes%'
                  AND query NOT LIKE '%pg_stat_activity%'
                  AND query NOT LIKE '%pg_database_size%'
                  AND query NOT LIKE '%pg_settings%'
                "#,
                &[],
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to count queries: {}", e)))?;

        let total_queries: i64 = count_row.get(0);

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
            .save_snapshot(database_id, &metrics)
            .await?;

        Ok(())
    }

    async fn collect_metrics(&self, database: &Database) -> AppResult<DatabaseMetrics> {
        let container_id = database
            .container_id
            .as_ref()
            .ok_or_else(|| AppError::Internal("Database has no container".to_string()))?;

        let docker_stats = self.docker.get_container_stats(container_id).await?;

        let pg_metrics = match self.connect_to_database(database).await {
            Ok(client) => self.collect_pg_metrics(&client, database).await,
            Err(e) => {
                tracing::warn!("Failed to connect to database for metrics: {}", e);
                PgMetrics::default()
            },
        };

        let timestamp = Utc::now().to_rfc3339();

        Ok(DatabaseMetrics {
            timestamp,
            queries: pg_metrics.queries,
            rows: pg_metrics.rows,
            tables: pg_metrics.tables,
            storage: StorageMetrics {
                database_size_bytes: pg_metrics.database_size_bytes,
                container_storage_bytes: docker_stats.memory_used_bytes,
                storage_limit_bytes: (database.storage_limit_mb as i64) * 1024 * 1024,
                storage_percent: if database.storage_limit_mb > 0 {
                    (pg_metrics.database_size_bytes as f64)
                        / ((database.storage_limit_mb as i64 * 1024 * 1024) as f64)
                        * 100.0
                } else {
                    0.0
                },
            },
            connections: pg_metrics.connections,
            resources: ResourceMetrics {
                cpu_percent: docker_stats.cpu_percent,
                memory_used_bytes: docker_stats.memory_used_bytes,
                memory_limit_bytes: docker_stats.memory_limit_bytes,
                memory_percent: docker_stats.memory_percent,
            },
        })
    }

    async fn collect_pg_metrics(&self, client: &Client, _database: &Database) -> PgMetrics {
        let mut metrics = PgMetrics::default();

        if let Ok(row) = client
            .query_one("SELECT pg_database_size(current_database())", &[])
            .await
        {
            metrics.database_size_bytes = row.get::<_, i64>(0);
        }

        let extension_exists = client
            .query_one(
                "SELECT 1 FROM pg_extension WHERE extname = 'pg_stat_statements'",
                &[],
            )
            .await
            .is_ok();

        if !extension_exists {
            let _ = client
                .execute("CREATE EXTENSION IF NOT EXISTS pg_stat_statements", &[])
                .await;
        }

        if let Ok(rows) = client
            .query(
                r#"
                SELECT
                    COALESCE(SUM(calls)::bigint, 0) as total_calls,
                    COALESCE(AVG(mean_exec_time), 0)::float8 as avg_time,
                    COALESCE(MAX(max_exec_time), 0)::float8 as max_time
                FROM pg_stat_statements
                WHERE userid = (SELECT usesysid FROM pg_user WHERE usename = current_user)
                  AND query NOT LIKE '%pg_stat_statements%'
                  AND query NOT LIKE '%pg_stat_user_tables%'
                  AND query NOT LIKE '%pg_stat_user_indexes%'
                  AND query NOT LIKE '%pg_stat_activity%'
                  AND query NOT LIKE '%pg_database_size%'
                  AND query NOT LIKE '%pg_settings%'
                "#,
                &[],
            )
            .await
        {
            if let Some(row) = rows.first() {
                metrics.queries.total_queries = row.get::<_, i64>(0);
                metrics.queries.avg_latency_ms = row.get::<_, f64>(1);
                metrics.queries.max_latency_ms = row.get::<_, f64>(2);
            }
        }

        if let Ok(rows) = client
            .query(
                r#"
                SELECT
                    COALESCE(SUM(seq_tup_read + idx_tup_fetch), 0)::bigint as rows_read,
                    COALESCE(SUM(n_tup_ins + n_tup_upd + n_tup_del), 0)::bigint as rows_written,
                    COALESCE(SUM(n_live_tup), 0)::bigint as total_rows
                FROM pg_stat_user_tables
                "#,
                &[],
            )
            .await
        {
            if let Some(row) = rows.first() {
                metrics.rows.rows_read = row.get::<_, i64>(0);
                metrics.rows.rows_written = row.get::<_, i64>(1);
                metrics.rows.total_rows = row.get::<_, i64>(2);
            }
        }

        if let Ok(rows) = client
            .query(
                r#"
                SELECT
                    COUNT(*) as table_count,
                    COALESCE(MAX(pg_total_relation_size(relid)), 0) as largest_table
                FROM pg_stat_user_tables
                "#,
                &[],
            )
            .await
        {
            if let Some(row) = rows.first() {
                metrics.tables.total_tables = row.get::<_, i64>(0);
                metrics.tables.largest_table_bytes = row.get::<_, i64>(1);
            }
        }

        if let Ok(row) = client
            .query_one("SELECT COUNT(*) FROM pg_stat_user_indexes", &[])
            .await
        {
            metrics.tables.total_indexes = row.get::<_, i64>(0);
        }

        if let Ok(rows) = client
            .query(
                r#"
                SELECT
                    COUNT(*) FILTER (WHERE state = 'active') as active,
                    COUNT(*) FILTER (WHERE state = 'idle') as idle,
                    (SELECT setting::int FROM pg_settings WHERE name = 'max_connections') as max_conn
                FROM pg_stat_activity
                WHERE datname = current_database()
                "#,
                &[],
            )
            .await
        {
            if let Some(row) = rows.first() {
                metrics.connections.active_connections = row.get::<_, i64>(0) as i32;
                metrics.connections.idle_connections = row.get::<_, i64>(1) as i32;
                metrics.connections.max_connections = row.get::<_, i32>(2);
                let total = metrics.connections.active_connections + metrics.connections.idle_connections;
                metrics.connections.connection_percent = if metrics.connections.max_connections > 0 {
                    (total as f64 / metrics.connections.max_connections as f64) * 100.0
                } else {
                    0.0
                };
            }
        }

        metrics
    }

    async fn connect_to_database(&self, database: &Database) -> AppResult<Client> {
        let encrypted = database
            .password_encrypted
            .as_ref()
            .ok_or_else(|| AppError::Internal("Database password not found".to_string()))?;

        let password = self.decrypt_password(encrypted)?;

        let container_name = database.container_name();
        let connection_string = format!(
            "host={} port=5432 user={} password={} dbname=postgres connect_timeout=10",
            container_name, database.username, password
        );

        let (client, connection) = tokio_postgres::connect(&connection_string, NoTls)
            .await
            .map_err(|e| {
                tracing::error!("PostgreSQL connection failed to {}: {}", container_name, e);
                AppError::Internal(format!("Failed to connect to database: {}", e))
            })?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                tracing::error!("Database connection error: {}", e);
            }
        });

        Ok(client)
    }
}

#[derive(Default)]
struct PgMetrics {
    queries: QueryMetrics,
    rows: RowMetrics,
    tables: TableMetrics,
    connections: ConnectionMetrics,
    database_size_bytes: i64,
}

fn truncate_query(query: &str) -> String {
    let query = query.trim();
    if query.len() > 200 {
        format!("{}...", &query[..197])
    } else {
        query.to_string()
    }
}
