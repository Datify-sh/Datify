use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use crate::domain::models::{MetricsHistoryPoint, MetricsSnapshot, TimeRange, UnifiedMetrics};
use crate::error::AppResult;

#[derive(Clone)]
pub struct MetricsRepository {
    pool: SqlitePool,
}

impl MetricsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn save_snapshot(
        &self,
        database_id: &str,
        database_type: &str,
        metrics: &UnifiedMetrics,
    ) -> AppResult<()> {
        let id = Uuid::new_v4().to_string();

        match metrics {
            UnifiedMetrics::Postgres(m) => {
                sqlx::query(
                    r#"
                    INSERT INTO metrics_snapshots (
                        id, database_id, database_type, timestamp,
                        total_queries, queries_per_sec, avg_latency_ms,
                        rows_read, rows_written,
                        cpu_percent, memory_percent, memory_used_bytes,
                        active_connections, storage_used_bytes
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&id)
                .bind(database_id)
                .bind(database_type)
                .bind(&m.timestamp)
                .bind(m.queries.total_queries)
                .bind(m.queries.queries_per_sec)
                .bind(m.queries.avg_latency_ms)
                .bind(m.rows.rows_read)
                .bind(m.rows.rows_written)
                .bind(m.resources.cpu_percent)
                .bind(m.resources.memory_percent)
                .bind(m.resources.memory_used_bytes)
                .bind(m.connections.active_connections)
                .bind(m.storage.database_size_bytes)
                .execute(&self.pool)
                .await?;
            },
            UnifiedMetrics::Redis(m) | UnifiedMetrics::Valkey(m) => {
                let db_type = match metrics {
                    UnifiedMetrics::Redis(_) => "redis",
                    UnifiedMetrics::Valkey(_) => "valkey",
                    _ => database_type,
                };
                sqlx::query(
                    r#"
                    INSERT INTO metrics_snapshots (
                        id, database_id, database_type, timestamp,
                        total_queries, queries_per_sec, avg_latency_ms,
                        rows_read, rows_written,
                        cpu_percent, memory_percent, memory_used_bytes,
                        active_connections, storage_used_bytes,
                        total_keys, keyspace_hits, keyspace_misses,
                        total_commands, ops_per_sec, used_memory, connected_clients
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&id)
                .bind(database_id)
                .bind(db_type)
                .bind(&m.timestamp)
                .bind(0i64)
                .bind(m.commands.ops_per_sec)
                .bind(0.0f64)
                .bind(0i64)
                .bind(0i64)
                .bind(m.resources.cpu_percent)
                .bind(m.resources.memory_percent)
                .bind(m.resources.memory_used_bytes)
                .bind(m.clients.connected_clients)
                .bind(m.memory.used_memory)
                .bind(m.keys.total_keys)
                .bind(m.commands.keyspace_hits)
                .bind(m.commands.keyspace_misses)
                .bind(m.commands.total_commands)
                .bind(m.commands.ops_per_sec)
                .bind(m.memory.used_memory)
                .bind(m.clients.connected_clients)
                .execute(&self.pool)
                .await?;
            },
        }

        Ok(())
    }

    pub async fn get_history(
        &self,
        database_id: &str,
        time_range: TimeRange,
    ) -> AppResult<Vec<MetricsHistoryPoint>> {
        let duration_secs = time_range.duration_secs();
        let interval_secs = time_range.interval_secs();

        let start_time = format!("-{} seconds", duration_secs);

        let snapshots = if interval_secs > 15 {
            sqlx::query_as::<_, MetricsSnapshot>(
                r#"
                WITH numbered AS (
                    SELECT *,
                        (CAST(strftime('%s', timestamp) AS INTEGER) / ?) * ? as time_bucket
                    FROM metrics_snapshots
                    WHERE database_id = ?
                    AND timestamp >= datetime('now', ?)
                ),
                grouped AS (
                    SELECT *,
                        ROW_NUMBER() OVER (PARTITION BY time_bucket ORDER BY timestamp DESC) as rn
                    FROM numbered
                )
                SELECT id, database_id, timestamp, total_queries, queries_per_sec, avg_latency_ms,
                       rows_read, rows_written, cpu_percent, memory_percent, memory_used_bytes,
                       active_connections, storage_used_bytes
                FROM grouped
                WHERE rn = 1
                ORDER BY timestamp ASC
                "#,
            )
            .bind(interval_secs)
            .bind(interval_secs)
            .bind(database_id)
            .bind(&start_time)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, MetricsSnapshot>(
                r#"
                SELECT id, database_id, timestamp, total_queries, queries_per_sec, avg_latency_ms,
                       rows_read, rows_written, cpu_percent, memory_percent, memory_used_bytes,
                       active_connections, storage_used_bytes
                FROM metrics_snapshots
                WHERE database_id = ?
                AND timestamp >= datetime('now', ?)
                ORDER BY timestamp ASC
                "#,
            )
            .bind(database_id)
            .bind(&start_time)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(snapshots.into_iter().map(|s| s.into()).collect())
    }

    pub async fn get_latest(&self, database_id: &str) -> AppResult<Option<MetricsSnapshot>> {
        let snapshot = sqlx::query_as::<_, MetricsSnapshot>(
            r#"
            SELECT id, database_id, timestamp, total_queries, queries_per_sec, avg_latency_ms,
                   rows_read, rows_written, cpu_percent, memory_percent, memory_used_bytes,
                   active_connections, storage_used_bytes
            FROM metrics_snapshots
            WHERE database_id = ?
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
        )
        .bind(database_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(snapshot)
    }

    pub async fn cleanup_old_snapshots(&self) -> AppResult<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM metrics_snapshots
            WHERE timestamp < datetime('now', '-24 hours')
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn count_by_database(&self, database_id: &str) -> AppResult<i64> {
        let count: (i64,) =
            sqlx::query_as(r#"SELECT COUNT(*) FROM metrics_snapshots WHERE database_id = ?"#)
                .bind(database_id)
                .fetch_one(&self.pool)
                .await?;

        Ok(count.0)
    }

    pub async fn delete_by_database(&self, database_id: &str) -> AppResult<u64> {
        let result = sqlx::query(r#"DELETE FROM metrics_snapshots WHERE database_id = ?"#)
            .bind(database_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}
