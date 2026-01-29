use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Schema introspection response containing all tables and views
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SchemaInfo {
    pub tables: Vec<TableInfo>,
    pub views: Vec<ViewInfo>,
}

/// Information about a database table
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TableInfo {
    pub schema: String,
    pub name: String,
    pub columns: Vec<ColumnDetail>,
    pub indexes: Vec<IndexInfo>,
    pub row_count_estimate: i64,
    pub size_bytes: i64,
}

/// Information about a database view
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ViewInfo {
    pub schema: String,
    pub name: String,
    pub columns: Vec<ColumnDetail>,
}

/// Detailed column information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ColumnDetail {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
}

/// Index information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub is_primary: bool,
}

/// Request body for executing SQL queries
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ExecuteQueryRequest {
    /// The SQL query to execute
    pub sql: String,
    /// Maximum number of rows to return (default: 1000, max: 10000)
    #[serde(default)]
    pub limit: Option<i32>,
    /// Query timeout in milliseconds (default: 30000, max: 60000)
    #[serde(default)]
    pub timeout_ms: Option<i32>,
}

/// Column information in query results
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ColumnInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: String,
}

/// Result of executing a SQL query
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QueryResult {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: i64,
    pub execution_time_ms: f64,
    pub truncated: bool,
}

/// Query parameters for table preview
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct TablePreviewQuery {
    #[serde(default)]
    pub limit: Option<i32>,
    #[serde(default)]
    pub offset: Option<i32>,
}

/// Response for table data preview
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TablePreview {
    pub schema: String,
    pub table: String,
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub total_rows: i64,
    pub limit: i32,
    pub offset: i32,
}
