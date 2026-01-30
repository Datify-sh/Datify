use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use chrono::Utc;
use deadpool_postgres::{ManagerConfig, Pool, RecyclingMethod};
use tokio::sync::RwLock;
use tokio_postgres::NoTls;

use crate::domain::models::{
    ConnectionMetrics, Database, DatabaseMetrics, QueryMetrics, ResourceMetrics, RowMetrics,
    StorageMetrics, TableMetrics, UnifiedMetrics,
};
use crate::error::{AppError, AppResult};
use crate::infrastructure::docker::ContainerStats;

const POOL_MAX_SIZE: usize = 2;
const POOL_TIMEOUT_SECS: u64 = 10;
const POOL_CACHE_TTL_SECS: u64 = 300;

struct CachedPool {
    pool: Pool,
    last_used: Instant,
}

pub struct PostgresMetricsCollector {
    encryption_key: Arc<[u8; 32]>,
    pools: Arc<RwLock<HashMap<String, CachedPool>>>,
}

impl PostgresMetricsCollector {
    pub fn new(encryption_key: [u8; 32]) -> Self {
        Self {
            encryption_key: Arc::new(encryption_key),
            pools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn cleanup_stale_pools(&self) {
        let mut pools = self.pools.write().await;
        let now = Instant::now();
        pools.retain(|_, cached| {
            now.duration_since(cached.last_used) < Duration::from_secs(POOL_CACHE_TTL_SECS)
        });
    }

    fn decrypt_password(&self, encrypted: &str) -> AppResult<String> {
        let data = hex::decode(encrypted)
            .map_err(|e| AppError::Internal(format!("Invalid encrypted data: {}", e)))?;

        if data.len() < 12 {
            return Err(AppError::Internal("Encrypted data too short".to_string()));
        }

        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let cipher = Aes256Gcm::new_from_slice(&*self.encryption_key)
            .map_err(|e| AppError::Internal(format!("Decryption init failed: {}", e)))?;

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AppError::Internal(format!("Decryption failed: {}", e)))?;

        String::from_utf8(plaintext)
            .map_err(|e| AppError::Internal(format!("Invalid UTF-8 in password: {}", e)))
    }

    async fn get_pool(&self, database: &Database) -> AppResult<Pool> {
        let db_id = &database.id;

        {
            let mut pools = self.pools.write().await;
            if let Some(cached) = pools.get_mut(db_id) {
                cached.last_used = Instant::now();
                return Ok(cached.pool.clone());
            }
        }

        let pool = self.create_pool(database).await?;

        {
            let mut pools = self.pools.write().await;
            pools.insert(
                db_id.clone(),
                CachedPool {
                    pool: pool.clone(),
                    last_used: Instant::now(),
                },
            );
        }

        Ok(pool)
    }

    async fn create_pool(&self, database: &Database) -> AppResult<Pool> {
        let encrypted = database
            .password_encrypted
            .as_ref()
            .ok_or_else(|| AppError::Internal("Database password not found".to_string()))?;

        let password = self.decrypt_password(encrypted)?;
        let container_name = database.container_name();

        let mut cfg = deadpool_postgres::Config::new();
        cfg.host = Some(container_name.clone());
        cfg.port = Some(5432);
        cfg.user = Some(database.username.clone());
        cfg.password = Some(password);
        cfg.dbname = Some("postgres".to_string());
        cfg.connect_timeout = Some(Duration::from_secs(POOL_TIMEOUT_SECS));
        cfg.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        let pool = cfg
            .create_pool(Some(deadpool_postgres::Runtime::Tokio1), NoTls)
            .map_err(|e| AppError::Internal(format!("Failed to create pool: {}", e)))?;

        pool.resize(POOL_MAX_SIZE);

        Ok(pool)
    }

    async fn get_client(&self, database: &Database) -> AppResult<deadpool_postgres::Client> {
        let pool = self.get_pool(database).await?;
        pool.get().await.map_err(|e| {
            tracing::error!("Failed to get connection from pool: {}", e);
            AppError::Internal(format!("Failed to get database connection: {}", e))
        })
    }

    async fn detect_pg_stat_statements_version(
        &self,
        client: &deadpool_postgres::Client,
    ) -> Option<String> {
        let row = client
            .query_one(
                "SELECT extversion FROM pg_extension WHERE extname = 'pg_stat_statements'",
                &[],
            )
            .await
            .ok()?;
        row.get::<_, Option<String>>(0)
    }

    fn use_modern_columns(version: Option<&str>) -> bool {
        match version {
            Some(v) => {
                let parts: Vec<&str> = v.split('.').collect();
                if let Some(major) = parts.first() {
                    if let Ok(major_num) = major.parse::<i32>() {
                        return major_num >= 1
                            && parts
                                .get(1)
                                .and_then(|m| m.parse::<i32>().ok())
                                .unwrap_or(0)
                                >= 8;
                    }
                }
                false
            },
            None => true,
        }
    }

    async fn collect_pg_metrics(&self, client: &deadpool_postgres::Client) -> PgMetrics {
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

        let version = self.detect_pg_stat_statements_version(client).await;
        let use_modern = Self::use_modern_columns(version.as_deref());

        let (mean_time_col, max_time_col) = if use_modern {
            ("mean_exec_time", "max_exec_time")
        } else {
            ("mean_time", "max_time")
        };

        let query = format!(
            r#"
            SELECT
                COALESCE(SUM(calls)::bigint, 0) as total_calls,
                COALESCE(AVG({}), 0)::float8 as avg_time,
                COALESCE(MAX({}), 0)::float8 as max_time
            FROM pg_stat_statements
            WHERE userid = (SELECT usesysid FROM pg_user WHERE usename = current_user)
              AND query NOT LIKE '%pg_stat_statements%'
              AND query NOT LIKE '%pg_stat_user_tables%'
              AND query NOT LIKE '%pg_stat_user_indexes%'
              AND query NOT LIKE '%pg_stat_activity%'
              AND query NOT LIKE '%pg_database_size%'
              AND query NOT LIKE '%pg_settings%'
            "#,
            mean_time_col, max_time_col
        );

        if let Ok(rows) = client.query(&query, &[]).await {
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
                let total =
                    metrics.connections.active_connections + metrics.connections.idle_connections;
                metrics.connections.connection_percent = if metrics.connections.max_connections > 0
                {
                    (total as f64 / metrics.connections.max_connections as f64) * 100.0
                } else {
                    0.0
                };
            }
        }

        metrics
    }

    pub async fn collect_metrics(
        &self,
        database: &Database,
        docker_stats: &ContainerStats,
    ) -> AppResult<UnifiedMetrics> {
        let pg_metrics = match self.get_client(database).await {
            Ok(client) => self.collect_pg_metrics(&client).await,
            Err(e) => {
                tracing::warn!("Failed to get connection for metrics: {}", e);
                PgMetrics::default()
            },
        };

        let timestamp = Utc::now().to_rfc3339();

        Ok(UnifiedMetrics::Postgres(DatabaseMetrics {
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
        }))
    }

    pub async fn get_query_logs(
        &self,
        database: &Database,
        limit: i32,
        sort_by: &str,
    ) -> AppResult<Vec<crate::domain::models::QueryLogEntry>> {
        let client = self.get_client(database).await?;

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
            return Ok(vec![]);
        }

        let version = self.detect_pg_stat_statements_version(&client).await;
        let use_modern = Self::use_modern_columns(version.as_deref());

        let (total_time_col, mean_time_col, min_time_col, max_time_col) = if use_modern {
            (
                "total_exec_time",
                "mean_exec_time",
                "min_exec_time",
                "max_exec_time",
            )
        } else {
            ("total_time", "mean_time", "min_time", "max_time")
        };

        let order_by = match sort_by {
            "avg_time" => format!("{} DESC", mean_time_col),
            "calls" => "calls DESC".to_string(),
            _ => format!("{} DESC", total_time_col),
        };

        let query = format!(
            r#"
            SELECT
                query,
                calls,
                {} as total_time_ms,
                {} as avg_time_ms,
                {} as min_time_ms,
                {} as max_time_ms,
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
            total_time_col, mean_time_col, min_time_col, max_time_col, order_by
        );

        let rows = client
            .query(&query, &[&(limit as i64)])
            .await
            .map_err(|e| {
                AppError::Internal(format!("Failed to query pg_stat_statements: {}", e))
            })?;

        let entries: Vec<crate::domain::models::QueryLogEntry> = rows
            .iter()
            .map(|row| crate::domain::models::QueryLogEntry {
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

        Ok(entries)
    }

    pub async fn count_query_logs(&self, database: &Database) -> AppResult<i64> {
        let client = self.get_client(database).await?;

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

        Ok(count_row.get(0))
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
