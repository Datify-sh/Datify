use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use crate::domain::models::{DatabaseMetrics, MetricsHistoryPoint, MetricsSnapshot, TimeRange};
use crate::error::AppResult;

#[derive(Clone)]
pub struct MetricsRepository {
    pool: SqlitePool,
}

impl MetricsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Save a metrics snapshot for a database
    pub async fn save_snapshot(
        &self,
        database_id: &str,
        metrics: &DatabaseMetrics,
    ) -> AppResult<MetricsSnapshot> {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO metrics_snapshots (
                id, database_id, timestamp,
                total_queries, queries_per_sec, avg_latency_ms,
                rows_read, rows_written,
                cpu_percent, memory_percent, memory_used_bytes,
                active_connections, storage_used_bytes
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(database_id)
        .bind(&metrics.timestamp)
        .bind(metrics.queries.total_queries)
        .bind(metrics.queries.queries_per_sec)
        .bind(metrics.queries.avg_latency_ms)
        .bind(metrics.rows.rows_read)
        .bind(metrics.rows.rows_written)
        .bind(metrics.resources.cpu_percent)
        .bind(metrics.resources.memory_percent)
        .bind(metrics.resources.memory_used_bytes)
        .bind(metrics.connections.active_connections)
        .bind(metrics.storage.database_size_bytes)
        .execute(&self.pool)
        .await?;

        let snapshot =
            sqlx::query_as::<_, MetricsSnapshot>(r#"SELECT * FROM metrics_snapshots WHERE id = ?"#)
                .bind(&id)
                .fetch_one(&self.pool)
                .await?;

        Ok(snapshot)
    }

    /// Get metrics history for a database within a time range
    pub async fn get_history(
        &self,
        database_id: &str,
        time_range: TimeRange,
    ) -> AppResult<Vec<MetricsHistoryPoint>> {
        let duration_secs = time_range.duration_secs();
        let interval_secs = time_range.interval_secs();

        // Calculate the start time based on the time range
        let start_time = format!("-{} seconds", duration_secs);

        // For larger time ranges, we need to aggregate/sample the data
        // We use window functions to pick representative samples at the desired interval
        let snapshots = if interval_secs > 15 {
            // For intervals larger than our collection interval, we sample
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
            // For small intervals, return all data points
            sqlx::query_as::<_, MetricsSnapshot>(
                r#"
                SELECT * FROM metrics_snapshots
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

    /// Get the most recent metrics snapshot for a database
    pub async fn get_latest(&self, database_id: &str) -> AppResult<Option<MetricsSnapshot>> {
        let snapshot = sqlx::query_as::<_, MetricsSnapshot>(
            r#"
            SELECT * FROM metrics_snapshots
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

    /// Clean up old metrics snapshots (older than 24 hours)
    /// This is also done via trigger, but can be called manually
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

    /// Get count of metrics snapshots for a database
    pub async fn count_by_database(&self, database_id: &str) -> AppResult<i64> {
        let count: (i64,) =
            sqlx::query_as(r#"SELECT COUNT(*) FROM metrics_snapshots WHERE database_id = ?"#)
                .bind(database_id)
                .fetch_one(&self.pool)
                .await?;

        Ok(count.0)
    }

    /// Delete all metrics for a database (called when database is deleted)
    pub async fn delete_by_database(&self, database_id: &str) -> AppResult<u64> {
        let result = sqlx::query(r#"DELETE FROM metrics_snapshots WHERE database_id = ?"#)
            .bind(database_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}
