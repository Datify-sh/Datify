use std::sync::Arc;
use std::time::Instant;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use tokio_postgres::{types::Type, Client, NoTls};

use crate::domain::models::{
    ColumnDetail, ColumnInfo, Database, IndexInfo, QueryResult, SchemaInfo, TableInfo,
    TablePreview, ViewInfo,
};
use crate::error::{AppError, AppResult};
use crate::infrastructure::docker::DockerManager;
use crate::repositories::{DatabaseRepository, ProjectRepository};

const MAX_SQL_LEN: usize = 100_000;

#[derive(Clone)]
pub struct SqlService {
    database_repo: DatabaseRepository,
    project_repo: ProjectRepository,
    #[allow(dead_code)]
    docker: Arc<DockerManager>,
    encryption_key: [u8; 32],
}

impl SqlService {
    pub fn new(
        database_repo: DatabaseRepository,
        project_repo: ProjectRepository,
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

    pub async fn check_access(
        &self,
        database_id: &str,
        user_id: &str,
        is_admin: bool,
    ) -> AppResult<bool> {
        if is_admin {
            return Ok(true);
        }
        let project_id = self
            .database_repo
            .get_project_id(database_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", database_id)))?;

        self.project_repo.is_owner(&project_id, user_id).await
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

    pub async fn get_schema(&self, database_id: &str) -> AppResult<SchemaInfo> {
        let database = self
            .database_repo
            .find_by_id(database_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", database_id)))?;

        if database.container_status != "running" {
            return Err(AppError::Validation(
                "Database must be running to get schema".to_string(),
            ));
        }

        let client = self.connect_to_database(&database).await?;

        let tables = self.get_tables(&client).await?;
        let views = self.get_views(&client).await?;

        Ok(SchemaInfo { tables, views })
    }

    async fn get_tables(&self, client: &Client) -> AppResult<Vec<TableInfo>> {
        // Get all user tables with basic info
        let rows = client
            .query(
                r#"
                SELECT
                    t.table_schema,
                    t.table_name,
                    COALESCE(c.reltuples::bigint, 0) as row_estimate,
                    COALESCE(pg_total_relation_size(c.oid), 0) as size_bytes
                FROM information_schema.tables t
                LEFT JOIN pg_class c ON c.relname = t.table_name
                LEFT JOIN pg_namespace n ON n.oid = c.relnamespace AND n.nspname = t.table_schema
                WHERE t.table_schema NOT IN ('pg_catalog', 'information_schema')
                  AND t.table_type = 'BASE TABLE'
                ORDER BY t.table_schema, t.table_name
                "#,
                &[],
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to query tables: {}", e)))?;

        let mut tables = Vec::new();

        for row in rows {
            let schema: String = row.get(0);
            let name: String = row.get(1);
            let row_estimate: i64 = row.get(2);
            let size_bytes: i64 = row.get(3);

            let columns = self.get_columns(client, &schema, &name).await?;
            let indexes = self.get_indexes(client, &schema, &name).await?;

            tables.push(TableInfo {
                schema,
                name,
                columns,
                indexes,
                row_count_estimate: row_estimate,
                size_bytes,
            });
        }

        Ok(tables)
    }

    async fn get_views(&self, client: &Client) -> AppResult<Vec<ViewInfo>> {
        let rows = client
            .query(
                r#"
                SELECT table_schema, table_name
                FROM information_schema.views
                WHERE table_schema NOT IN ('pg_catalog', 'information_schema')
                ORDER BY table_schema, table_name
                "#,
                &[],
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to query views: {}", e)))?;

        let mut views = Vec::new();

        for row in rows {
            let schema: String = row.get(0);
            let name: String = row.get(1);
            let columns = self.get_columns(client, &schema, &name).await?;

            views.push(ViewInfo {
                schema,
                name,
                columns,
            });
        }

        Ok(views)
    }

    async fn get_columns(
        &self,
        client: &Client,
        schema: &str,
        table: &str,
    ) -> AppResult<Vec<ColumnDetail>> {
        let rows = client
            .query(
                r#"
                SELECT
                    c.column_name,
                    c.data_type,
                    c.is_nullable = 'YES' as nullable,
                    c.column_default,
                    COALESCE(pk.is_pk, false) as is_primary_key
                FROM information_schema.columns c
                LEFT JOIN (
                    SELECT kcu.column_name, true as is_pk
                    FROM information_schema.table_constraints tc
                    JOIN information_schema.key_column_usage kcu
                        ON tc.constraint_name = kcu.constraint_name
                        AND tc.table_schema = kcu.table_schema
                    WHERE tc.constraint_type = 'PRIMARY KEY'
                      AND tc.table_schema = $1
                      AND tc.table_name = $2
                ) pk ON c.column_name = pk.column_name
                WHERE c.table_schema = $1 AND c.table_name = $2
                ORDER BY c.ordinal_position
                "#,
                &[&schema, &table],
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to query columns: {}", e)))?;

        let columns = rows
            .iter()
            .map(|row| ColumnDetail {
                name: row.get(0),
                data_type: row.get(1),
                nullable: row.get(2),
                default_value: row.get(3),
                is_primary_key: row.get(4),
            })
            .collect();

        Ok(columns)
    }

    async fn get_indexes(
        &self,
        client: &Client,
        schema: &str,
        table: &str,
    ) -> AppResult<Vec<IndexInfo>> {
        let rows = client
            .query(
                r#"
                SELECT
                    i.relname as index_name,
                    array_agg(a.attname ORDER BY x.n) as columns,
                    ix.indisunique as is_unique,
                    ix.indisprimary as is_primary
                FROM pg_class t
                JOIN pg_namespace n ON n.oid = t.relnamespace
                JOIN pg_index ix ON ix.indrelid = t.oid
                JOIN pg_class i ON i.oid = ix.indexrelid
                CROSS JOIN LATERAL unnest(ix.indkey) WITH ORDINALITY AS x(attnum, n)
                JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = x.attnum
                WHERE n.nspname = $1 AND t.relname = $2
                GROUP BY i.relname, ix.indisunique, ix.indisprimary
                ORDER BY i.relname
                "#,
                &[&schema, &table],
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to query indexes: {}", e)))?;

        let indexes = rows
            .iter()
            .map(|row| IndexInfo {
                name: row.get(0),
                columns: row.get::<_, Vec<String>>(1),
                is_unique: row.get(2),
                is_primary: row.get(3),
            })
            .collect();

        Ok(indexes)
    }

    pub async fn execute_query(
        &self,
        database_id: &str,
        sql: &str,
        limit: i32,
        timeout_ms: i32,
    ) -> AppResult<QueryResult> {
        let trimmed = sql.trim();
        if trimmed.is_empty() {
            return Err(AppError::Validation(
                "SQL query cannot be empty".to_string(),
            ));
        }
        if trimmed.len() > MAX_SQL_LEN {
            return Err(AppError::Validation("SQL query is too large".to_string()));
        }

        let database = self
            .database_repo
            .find_by_id(database_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", database_id)))?;

        if database.container_status != "running" {
            return Err(AppError::Validation(
                "Database must be running to execute queries".to_string(),
            ));
        }

        // Basic SQL injection prevention - disallow dangerous statements
        let sql_upper = trimmed.to_uppercase();
        let dangerous_patterns = [
            "DROP DATABASE",
            "DROP SCHEMA",
            "TRUNCATE",
            "ALTER SYSTEM",
            "COPY FROM",
            "COPY TO",
        ];

        for pattern in dangerous_patterns {
            if sql_upper.contains(pattern) {
                return Err(AppError::Validation(format!(
                    "Statement '{}' is not allowed",
                    pattern
                )));
            }
        }

        let client = self.connect_to_database(&database).await?;

        // Set statement timeout
        client
            .execute(&format!("SET statement_timeout = {}", timeout_ms), &[])
            .await
            .map_err(|e| AppError::Internal(format!("Failed to set timeout: {}", e)))?;

        let start = Instant::now();

        let fetch_limit = limit + 1;
        let is_select = matches!(main_statement_keyword(trimmed).as_deref(), Some("SELECT"));
        let limited_sql = if is_select {
            format!(
                "SELECT * FROM ({}) AS datify_query LIMIT {}",
                trimmed.trim_end_matches(';'),
                fetch_limit
            )
        } else {
            trimmed.to_string()
        };

        let stmt = client
            .prepare(&limited_sql)
            .await
            .map_err(|e| AppError::Validation(format!("Query preparation failed: {}", e)))?;

        let columns: Vec<ColumnInfo> = stmt
            .columns()
            .iter()
            .map(|col| ColumnInfo {
                name: col.name().to_string(),
                data_type: type_to_string(col.type_()),
            })
            .collect();

        let rows = client
            .query(&limited_sql, &[])
            .await
            .map_err(|e| AppError::Validation(format!("Query execution failed: {}", e)))?;

        let execution_time_ms = start.elapsed().as_secs_f64() * 1000.0;

        let total_rows = rows.len() as i64;
        let truncated = total_rows > limit as i64;
        let take_rows = total_rows.min(limit as i64) as usize;

        let result_rows: Vec<Vec<serde_json::Value>> = rows
            .iter()
            .take(take_rows)
            .map(|row| (0..row.len()).map(|i| row_value_to_json(row, i)).collect())
            .collect();

        Ok(QueryResult {
            columns,
            rows: result_rows,
            row_count: total_rows,
            execution_time_ms,
            truncated,
        })
    }

    pub async fn preview_table(
        &self,
        database_id: &str,
        schema: &str,
        table: &str,
        limit: i32,
        offset: i32,
    ) -> AppResult<TablePreview> {
        let database = self
            .database_repo
            .find_by_id(database_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", database_id)))?;

        if database.container_status != "running" {
            return Err(AppError::Validation(
                "Database must be running to preview tables".to_string(),
            ));
        }

        // Validate schema and table names to prevent SQL injection
        if !is_valid_identifier(schema) || !is_valid_identifier(table) {
            return Err(AppError::Validation(
                "Invalid schema or table name".to_string(),
            ));
        }

        let client = self.connect_to_database(&database).await?;

        // Get total row count
        let count_query = format!(
            "SELECT COUNT(*) FROM \"{}\".\"{}\"",
            schema.replace('"', "\"\""),
            table.replace('"', "\"\"")
        );
        let count_row = client
            .query_one(&count_query, &[])
            .await
            .map_err(|e| AppError::Internal(format!("Failed to count rows: {}", e)))?;
        let total_rows: i64 = count_row.get(0);

        // Get preview data
        let preview_query = format!(
            "SELECT * FROM \"{}\".\"{}\" LIMIT {} OFFSET {}",
            schema.replace('"', "\"\""),
            table.replace('"', "\"\""),
            limit,
            offset
        );

        let rows = client
            .query(&preview_query, &[])
            .await
            .map_err(|e| AppError::Internal(format!("Failed to query table: {}", e)))?;

        let columns: Vec<ColumnInfo> = if rows.is_empty() {
            // Get columns from information_schema if table is empty
            let col_rows = client
                .query(
                    "SELECT column_name, data_type FROM information_schema.columns
                     WHERE table_schema = $1 AND table_name = $2
                     ORDER BY ordinal_position",
                    &[&schema, &table],
                )
                .await
                .map_err(|e| AppError::Internal(format!("Failed to get columns: {}", e)))?;

            col_rows
                .iter()
                .map(|row| ColumnInfo {
                    name: row.get(0),
                    data_type: row.get(1),
                })
                .collect()
        } else {
            rows[0]
                .columns()
                .iter()
                .map(|col| ColumnInfo {
                    name: col.name().to_string(),
                    data_type: type_to_string(col.type_()),
                })
                .collect()
        };

        let result_rows: Vec<Vec<serde_json::Value>> = rows
            .iter()
            .map(|row| (0..row.len()).map(|i| row_value_to_json(row, i)).collect())
            .collect();

        Ok(TablePreview {
            schema: schema.to_string(),
            table: table.to_string(),
            columns,
            rows: result_rows,
            total_rows,
            limit,
            offset,
        })
    }
}

fn main_statement_keyword(sql: &str) -> Option<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut token = String::new();
    let mut depth: i32 = 0;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut chars = sql.chars().peekable();

    while let Some(c) = chars.next() {
        if in_line_comment {
            if c == '\n' {
                in_line_comment = false;
            }
            continue;
        }

        if in_block_comment {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block_comment = false;
            }
            continue;
        }

        if in_single {
            if c == '\'' {
                if chars.peek() == Some(&'\'') {
                    chars.next();
                } else {
                    in_single = false;
                }
            }
            continue;
        }

        if in_double {
            if c == '"' {
                in_double = false;
            }
            continue;
        }

        if c == '-' && chars.peek() == Some(&'-') {
            chars.next();
            in_line_comment = true;
            continue;
        }

        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            in_block_comment = true;
            continue;
        }

        if c == '\'' {
            in_single = true;
            continue;
        }

        if c == '"' {
            in_double = true;
            continue;
        }

        match c {
            '(' => {
                if depth == 0 && !token.is_empty() {
                    tokens.push(token.to_uppercase());
                    token.clear();
                }
                depth += 1;
            },
            ')' => {
                if depth == 0 && !token.is_empty() {
                    tokens.push(token.to_uppercase());
                    token.clear();
                }
                if depth > 0 {
                    depth -= 1;
                }
            },
            _ => {
                if depth == 0 && (c.is_alphanumeric() || c == '_') {
                    token.push(c);
                } else if depth == 0 && !token.is_empty() {
                    tokens.push(token.to_uppercase());
                    token.clear();
                }
            },
        }
    }

    if depth == 0 && !token.is_empty() {
        tokens.push(token.to_uppercase());
    }

    let mut iter = tokens.into_iter();
    let first = iter.next()?;

    if first == "WITH" {
        let mut next = iter.next();
        if next.as_deref() == Some("RECURSIVE") {
            next = iter.next();
        }
        while let Some(tok) = next {
            if matches!(tok.as_str(), "SELECT" | "INSERT" | "UPDATE" | "DELETE") {
                return Some(tok);
            }
            next = iter.next();
        }
        None
    } else {
        Some(first)
    }
}

fn type_to_string(t: &Type) -> String {
    match *t {
        Type::BOOL => "boolean".to_string(),
        Type::INT2 => "smallint".to_string(),
        Type::INT4 => "integer".to_string(),
        Type::INT8 => "bigint".to_string(),
        Type::FLOAT4 => "real".to_string(),
        Type::FLOAT8 => "double precision".to_string(),
        Type::NUMERIC => "numeric".to_string(),
        Type::VARCHAR => "character varying".to_string(),
        Type::TEXT => "text".to_string(),
        Type::CHAR => "character".to_string(),
        Type::TIMESTAMP => "timestamp".to_string(),
        Type::TIMESTAMPTZ => "timestamp with time zone".to_string(),
        Type::DATE => "date".to_string(),
        Type::TIME => "time".to_string(),
        Type::TIMETZ => "time with time zone".to_string(),
        Type::UUID => "uuid".to_string(),
        Type::JSON => "json".to_string(),
        Type::JSONB => "jsonb".to_string(),
        Type::BYTEA => "bytea".to_string(),
        _ => t.name().to_string(),
    }
}

fn row_value_to_json(row: &tokio_postgres::Row, index: usize) -> serde_json::Value {
    let col_type = row.columns()[index].type_();

    // Try to get value based on type
    match *col_type {
        Type::BOOL => row
            .try_get::<_, Option<bool>>(index)
            .ok()
            .flatten()
            .map(serde_json::Value::Bool)
            .unwrap_or(serde_json::Value::Null),
        Type::INT2 => row
            .try_get::<_, Option<i16>>(index)
            .ok()
            .flatten()
            .map(|v| serde_json::Value::Number(v.into()))
            .unwrap_or(serde_json::Value::Null),
        Type::INT4 => row
            .try_get::<_, Option<i32>>(index)
            .ok()
            .flatten()
            .map(|v| serde_json::Value::Number(v.into()))
            .unwrap_or(serde_json::Value::Null),
        Type::INT8 => row
            .try_get::<_, Option<i64>>(index)
            .ok()
            .flatten()
            .map(|v| serde_json::Value::Number(v.into()))
            .unwrap_or(serde_json::Value::Null),
        Type::FLOAT4 => row
            .try_get::<_, Option<f32>>(index)
            .ok()
            .flatten()
            .map(|v| {
                serde_json::Number::from_f64(v as f64)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            })
            .unwrap_or(serde_json::Value::Null),
        Type::FLOAT8 => row
            .try_get::<_, Option<f64>>(index)
            .ok()
            .flatten()
            .map(|v| {
                serde_json::Number::from_f64(v)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            })
            .unwrap_or(serde_json::Value::Null),
        Type::JSON | Type::JSONB => {
            // Get JSON as string and parse it
            row.try_get::<_, Option<String>>(index)
                .ok()
                .flatten()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(serde_json::Value::Null)
        },
        _ => {
            // For all other types, try to get as string
            row.try_get::<_, Option<String>>(index)
                .ok()
                .flatten()
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null)
        },
    }
}

fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() || s.len() > 128 {
        return false;
    }
    // Allow alphanumeric, underscores, and must start with letter or underscore
    let first = s.chars().next().unwrap();
    if !first.is_alphabetic() && first != '_' {
        return false;
    }
    s.chars().all(|c| c.is_alphanumeric() || c == '_')
}
