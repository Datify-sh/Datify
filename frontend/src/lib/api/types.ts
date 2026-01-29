export interface LoginRequest {
  email: string;
  password: string;
}

export interface RegisterRequest {
  email: string;
  password: string;
  name: string;
}

export interface RefreshRequest {
  refresh_token: string;
}

export interface AuthTokens {
  access_token: string;
  refresh_token: string;
  token_type: string;
  expires_in: number;
}

export interface UserResponse {
  id: string;
  email: string;
  name: string;
  role: "admin" | "user";
  email_verified: boolean;
  created_at: string;
}

export interface LoginResponse {
  user: UserResponse;
  tokens: AuthTokens;
}

export interface CreateProjectRequest {
  name: string;
  description?: string | null;
  settings?: Record<string, unknown> | null;
}

export interface UpdateProjectRequest {
  name?: string | null;
  description?: string | null;
  settings?: Record<string, unknown> | null;
}

export interface ProjectResponse {
  id: string;
  name: string;
  slug: string;
  description?: string | null;
  settings: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface ProjectWithStats extends ProjectResponse {
  database_count: number;
}

export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  limit: number;
  offset: number;
}

export type PostgresVersion = string;
export type DatabaseType = "postgres" | "valkey";

export interface ResourceLimits {
  cpu_limit: number;
  memory_limit_mb: number;
  storage_limit_mb: number;
}

export interface ConnectionInfo {
  host: string;
  port: number;
  username: string;
  password: string;
  database: string;
  connection_string: string;
}

export interface CreateDatabaseRequest {
  name: string;
  database_type?: DatabaseType;
  postgres_version?: string;
  valkey_version?: string;
  password?: string;
  cpu_limit?: number;
  memory_limit_mb?: number;
  storage_limit_mb?: number;
  public_exposed?: boolean;
}

export interface ChangePasswordRequest {
  current_password: string;
  new_password: string;
}

export interface UpdateDatabaseRequest {
  name?: string | null;
  cpu_limit?: number | null;
  memory_limit_mb?: number | null;
  storage_limit_mb?: number | null;
  public_exposed?: boolean | null;
}

export interface BranchInfo {
  name: string;
  is_default: boolean;
  parent_id?: string | null;
  forked_at?: string | null;
}

export interface DatabaseResponse {
  id: string;
  project_id: string;
  name: string;
  database_type: DatabaseType;
  postgres_version: string;
  valkey_version?: string | null;
  status: string;
  resources: ResourceLimits;
  storage_used_mb?: number;
  public_exposed?: boolean;
  connection?: ConnectionInfo | null;
  created_at: string;
  updated_at: string;
  branch: BranchInfo;
}

export interface BranchResponse {
  id: string;
  name: string;
  is_default: boolean;
  status: string;
  parent_id?: string | null;
  forked_at?: string | null;
  created_at: string;
}

export interface CreateBranchRequest {
  name: string;
  include_data?: boolean;
}

export type LogType = "setup" | "runtime" | "system";

export interface LogEntryResponse {
  log_type: LogType;
  stream: string;
  message: string;
  timestamp?: string | null;
}

export interface LogsResponse {
  database_id: string;
  container_id?: string | null;
  entries: LogEntryResponse[];
  has_more: boolean;
}

export interface CheckStatus {
  status: string;
  message?: string | null;
}

export interface HealthChecks {
  database: CheckStatus;
  docker: CheckStatus;
  caddy: CheckStatus;
}

export interface HealthResponse {
  status: string;
  version: string;
}

export interface ReadyResponse {
  status: string;
  checks: HealthChecks;
}

export interface SystemInfoResponse {
  cpu_cores: number;
  total_memory_mb: number;
}

export interface PostgresVersionInfo {
  version: string;
  tag: string;
  is_latest: boolean;
}

export interface PostgresVersionsResponse {
  versions: PostgresVersionInfo[];
  default_version: string;
}

export interface ValkeyVersionInfo {
  version: string;
  tag: string;
  is_latest: boolean;
}

export interface ValkeyVersionsResponse {
  versions: ValkeyVersionInfo[];
  default_version: string;
}

export interface ApiError {
  message: string;
  status: number;
}

export type TimeRange =
  | "realtime"
  | "last_5_min"
  | "last_15_min"
  | "last_30_min"
  | "last_1_hour"
  | "last_24_hours";

export interface QueryMetrics {
  total_queries: number;
  queries_per_sec: number;
  avg_latency_ms: number;
  max_latency_ms: number;
}

export interface RowMetrics {
  rows_read: number;
  rows_written: number;
  total_rows: number;
}

export interface TableMetrics {
  total_tables: number;
  largest_table_bytes: number;
  total_indexes: number;
}

export interface StorageMetrics {
  database_size_bytes: number;
  container_storage_bytes: number;
  storage_limit_bytes: number;
  storage_percent: number;
}

export interface ConnectionMetrics {
  active_connections: number;
  idle_connections: number;
  max_connections: number;
  connection_percent: number;
}

export interface ResourceMetrics {
  cpu_percent: number;
  memory_used_bytes: number;
  memory_limit_bytes: number;
  memory_percent: number;
}

export interface DatabaseMetrics {
  timestamp: string;
  queries: QueryMetrics;
  rows: RowMetrics;
  tables: TableMetrics;
  storage: StorageMetrics;
  connections: ConnectionMetrics;
  resources: ResourceMetrics;
}

export interface MetricsResponse {
  database_id: string;
  metrics: DatabaseMetrics;
}

export interface MetricsHistoryPoint {
  timestamp: string;
  total_queries: number;
  queries_per_sec: number;
  avg_latency_ms: number;
  rows_read: number;
  rows_written: number;
  cpu_percent: number;
  memory_percent: number;
  active_connections: number;
}

export interface MetricsHistory {
  database_id: string;
  time_range: TimeRange;
  start_time: string;
  end_time: string;
  points: MetricsHistoryPoint[];
}

export interface QueryLogEntry {
  query: string;
  calls: number;
  total_time_ms: number;
  avg_time_ms: number;
  min_time_ms: number;
  max_time_ms: number;
  rows: number;
  rows_per_call: number;
}

export interface QueryLogsResponse {
  database_id: string;
  entries: QueryLogEntry[];
  total_queries: number;
}

export type MetricsStreamMessage =
  | { type: "connected"; database_id: string }
  | { type: "metrics"; metrics: DatabaseMetrics }
  | { type: "error"; message: string };

// SQL Editor types
export interface SchemaInfo {
  tables: TableInfo[];
  views: ViewInfo[];
}

export interface TableInfo {
  schema: string;
  name: string;
  columns: ColumnDetail[];
  indexes: IndexInfo[];
  row_count_estimate: number;
  size_bytes: number;
}

export interface ViewInfo {
  schema: string;
  name: string;
  columns: ColumnDetail[];
}

export interface ColumnDetail {
  name: string;
  data_type: string;
  nullable: boolean;
  default_value?: string | null;
  is_primary_key: boolean;
}

export interface IndexInfo {
  name: string;
  columns: string[];
  is_unique: boolean;
  is_primary: boolean;
}

export interface ExecuteQueryRequest {
  sql: string;
  limit?: number;
  timeout_ms?: number;
}

export interface ColumnInfo {
  name: string;
  type: string;
}

export interface QueryResult {
  columns: ColumnInfo[];
  rows: unknown[][];
  row_count: number;
  execution_time_ms: number;
  truncated: boolean;
}

export interface TablePreview {
  schema: string;
  table: string;
  columns: ColumnInfo[];
  rows: unknown[][];
  total_rows: number;
  limit: number;
  offset: number;
}
