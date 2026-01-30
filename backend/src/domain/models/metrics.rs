use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TimeRange {
    Realtime,
    Last5Min,
    #[default]
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
    Metrics { metrics: UnifiedMetrics },
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct KeyMetrics {
    pub total_keys: i64,
    pub keys_with_expiry: i64,
    pub expired_keys: i64,
    pub evicted_keys: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct CommandMetrics {
    pub total_commands: i64,
    pub ops_per_sec: f64,
    pub keyspace_hits: i64,
    pub keyspace_misses: i64,
    pub hit_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ClientMetrics {
    pub connected_clients: i32,
    pub blocked_clients: i32,
    pub max_clients: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct MemoryMetrics {
    pub used_memory: i64,
    pub used_memory_rss: i64,
    pub used_memory_peak: i64,
    pub max_memory: i64,
    pub memory_fragmentation_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ReplicationMetrics {
    pub role: String,
    pub connected_slaves: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct KeyValueMetrics {
    pub timestamp: String,
    pub keys: KeyMetrics,
    pub commands: CommandMetrics,
    pub memory: MemoryMetrics,
    pub clients: ClientMetrics,
    pub replication: ReplicationMetrics,
    pub resources: ResourceMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "database_type", rename_all = "snake_case")]
pub enum UnifiedMetrics {
    Postgres(DatabaseMetrics),
    Redis(KeyValueMetrics),
    Valkey(KeyValueMetrics),
}

impl Default for UnifiedMetrics {
    fn default() -> Self {
        UnifiedMetrics::Postgres(DatabaseMetrics::default())
    }
}

impl UnifiedMetrics {
    pub fn timestamp(&self) -> &str {
        match self {
            UnifiedMetrics::Postgres(m) => &m.timestamp,
            UnifiedMetrics::Redis(m) | UnifiedMetrics::Valkey(m) => &m.timestamp,
        }
    }

    pub fn cpu_percent(&self) -> f64 {
        match self {
            UnifiedMetrics::Postgres(m) => m.resources.cpu_percent,
            UnifiedMetrics::Redis(m) | UnifiedMetrics::Valkey(m) => m.resources.cpu_percent,
        }
    }

    pub fn memory_percent(&self) -> f64 {
        match self {
            UnifiedMetrics::Postgres(m) => m.resources.memory_percent,
            UnifiedMetrics::Redis(m) | UnifiedMetrics::Valkey(m) => m.resources.memory_percent,
        }
    }

    pub fn memory_used_bytes(&self) -> i64 {
        match self {
            UnifiedMetrics::Postgres(m) => m.resources.memory_used_bytes,
            UnifiedMetrics::Redis(m) | UnifiedMetrics::Valkey(m) => m.resources.memory_used_bytes,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UnifiedMetricsResponse {
    pub database_id: String,
    pub metrics: UnifiedMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct KvMetricsHistoryPoint {
    pub timestamp: String,
    pub total_keys: i64,
    pub ops_per_sec: f64,
    pub hit_rate: f64,
    pub used_memory: i64,
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub connected_clients: i32,
}

#[derive(Debug, Clone, FromRow)]
pub struct KvMetricsSnapshot {
    pub id: String,
    pub database_id: String,
    pub database_type: String,
    pub timestamp: String,
    pub total_keys: i64,
    pub keyspace_hits: i64,
    pub keyspace_misses: i64,
    pub total_commands: i64,
    pub ops_per_sec: f64,
    pub used_memory: i64,
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub memory_used_bytes: i64,
    pub connected_clients: i32,
}

impl From<KvMetricsSnapshot> for KvMetricsHistoryPoint {
    fn from(s: KvMetricsSnapshot) -> Self {
        let hit_rate = if s.keyspace_hits + s.keyspace_misses > 0 {
            (s.keyspace_hits as f64 / (s.keyspace_hits + s.keyspace_misses) as f64) * 100.0
        } else {
            0.0
        };
        KvMetricsHistoryPoint {
            timestamp: s.timestamp,
            total_keys: s.total_keys,
            ops_per_sec: s.ops_per_sec,
            hit_rate,
            used_memory: s.used_memory,
            cpu_percent: s.cpu_percent,
            memory_percent: s.memory_percent,
            connected_clients: s.connected_clients,
        }
    }
}
