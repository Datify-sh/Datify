use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimeRange {
    Realtime,
    Last5Min,
    Last15Min,
    Last30Min,
    Last1Hour,
    Last24Hours,
}

impl TimeRange {
    pub fn duration_secs(&self) -> i64 {
        match self {
            TimeRange::Realtime => 60,
            TimeRange::Last5Min => 300,
            TimeRange::Last15Min => 900,
            TimeRange::Last30Min => 1800,
            TimeRange::Last1Hour => 3600,
            TimeRange::Last24Hours => 86400,
        }
    }

    pub fn interval_secs(&self) -> i64 {
        match self {
            TimeRange::Realtime => 1,
            TimeRange::Last5Min => 5,
            TimeRange::Last15Min => 15,
            TimeRange::Last30Min => 30,
            TimeRange::Last1Hour => 60,
            TimeRange::Last24Hours => 300,
        }
    }
}

impl Default for TimeRange {
    fn default() -> Self {
        TimeRange::Last15Min
    }
}

impl std::str::FromStr for TimeRange {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "realtime" => Ok(TimeRange::Realtime),
            "last_5_min" => Ok(TimeRange::Last5Min),
            "last_15_min" => Ok(TimeRange::Last15Min),
            "last_30_min" => Ok(TimeRange::Last30Min),
            "last_1_hour" => Ok(TimeRange::Last1Hour),
            "last_24_hours" => Ok(TimeRange::Last24Hours),
            _ => Err(format!("Invalid time range: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct QueryMetrics {
    pub total_queries: i64,
    pub queries_per_sec: f64,
    pub avg_latency_ms: f64,
    pub max_latency_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct RowMetrics {
    pub rows_read: i64,
    pub rows_written: i64,
    pub total_rows: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct TableMetrics {
    pub total_tables: i64,
    pub largest_table_bytes: i64,
    pub total_indexes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct StorageMetrics {
    pub database_size_bytes: i64,
    pub container_storage_bytes: i64,
    pub storage_limit_bytes: i64,
    pub storage_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ConnectionMetrics {
    pub active_connections: i32,
    pub idle_connections: i32,
    pub max_connections: i32,
    pub connection_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ResourceMetrics {
    pub cpu_percent: f64,
    pub memory_used_bytes: i64,
    pub memory_limit_bytes: i64,
    pub memory_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct DatabaseMetrics {
    pub timestamp: String,
    pub queries: QueryMetrics,
    pub rows: RowMetrics,
    pub tables: TableMetrics,
    pub storage: StorageMetrics,
    pub connections: ConnectionMetrics,
    pub resources: ResourceMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetricsHistoryPoint {
    pub timestamp: String,
    pub total_queries: i64,
    pub queries_per_sec: f64,
    pub avg_latency_ms: f64,
    pub rows_read: i64,
    pub rows_written: i64,
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub active_connections: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetricsHistory {
    pub database_id: String,
    pub time_range: TimeRange,
    pub start_time: String,
    pub end_time: String,
    pub points: Vec<MetricsHistoryPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QueryLogEntry {
    pub query: String,
    pub calls: i64,
    pub total_time_ms: f64,
    pub avg_time_ms: f64,
    pub min_time_ms: f64,
    pub max_time_ms: f64,
    pub rows: i64,
    pub rows_per_call: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QueryLogsResponse {
    pub database_id: String,
    pub entries: Vec<QueryLogEntry>,
    pub total_queries: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetricsResponse {
    pub database_id: String,
    pub metrics: DatabaseMetrics,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MetricsHistoryQuery {
    #[serde(default)]
    pub range: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct QueryLogsQuery {
    #[serde(default)]
    pub limit: Option<i32>,
    #[serde(default)]
    pub sort_by: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct MetricsSnapshot {
    pub id: String,
    pub database_id: String,
    pub timestamp: String,
    pub total_queries: i64,
    pub queries_per_sec: f64,
    pub avg_latency_ms: f64,
    pub rows_read: i64,
    pub rows_written: i64,
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub memory_used_bytes: i64,
    pub active_connections: i32,
    pub storage_used_bytes: i64,
}

impl From<MetricsSnapshot> for MetricsHistoryPoint {
    fn from(s: MetricsSnapshot) -> Self {
        MetricsHistoryPoint {
            timestamp: s.timestamp,
            total_queries: s.total_queries,
            queries_per_sec: s.queries_per_sec,
            avg_latency_ms: s.avg_latency_ms,
            rows_read: s.rows_read,
            rows_written: s.rows_written,
            cpu_percent: s.cpu_percent,
            memory_percent: s.memory_percent,
            active_connections: s.active_connections,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MetricsStreamMessage {
    Connected { database_id: String },
    Metrics { metrics: DatabaseMetrics },
    Error { message: String },
}
