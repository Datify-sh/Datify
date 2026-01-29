use std::fs;
use std::path::Path;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Datify API",
        version = "0.1.0",
        description = "Self-hosted database branching platform - an open-source alternative to Neon",
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        ),
        contact(
            name = "Datify Contributors",
            url = "https://github.com/datify/datify"
        )
    ),
    servers(
        (url = "http://localhost:8080", description = "Local development server"),
        (url = "/", description = "Current server")
    ),
    paths(
        crate::api::handlers::health,
        crate::api::handlers::ready,
        crate::api::handlers::register,
        crate::api::handlers::login,
        crate::api::handlers::refresh,
        crate::api::handlers::me,
        crate::api::handlers::create_project,
        crate::api::handlers::list_projects,
        crate::api::handlers::get_project,
        crate::api::handlers::update_project,
        crate::api::handlers::delete_project,
        crate::api::handlers::create_database,
        crate::api::handlers::list_databases,
        crate::api::handlers::get_database,
        crate::api::handlers::update_database,
        crate::api::handlers::delete_database,
        crate::api::handlers::start_database,
        crate::api::handlers::stop_database,
        crate::api::handlers::get_database_logs,
        crate::api::handlers::stream_database_logs,
        crate::api::handlers::database_terminal,
        crate::api::handlers::database_psql,
        crate::api::handlers::get_database_metrics,
        crate::api::handlers::get_database_metrics_history,
        crate::api::handlers::get_database_queries,
        crate::api::handlers::stream_database_metrics,
        crate::api::handlers::list_branches,
        crate::api::handlers::create_branch,
        crate::api::handlers::sync_from_parent,
        crate::api::handlers::get_database_schema,
        crate::api::handlers::execute_query,
        crate::api::handlers::preview_table,
    ),
    components(schemas(
        crate::api::handlers::HealthResponse,
        crate::api::handlers::ReadyResponse,
        crate::api::handlers::HealthChecks,
        crate::api::handlers::CheckStatus,
        crate::api::handlers::RegisterRequest,
        crate::api::handlers::LoginRequest,
        crate::api::handlers::RefreshRequest,
        crate::domain::models::UserResponse,
        crate::domain::models::UserRole,
        crate::domain::services::AuthTokens,
        crate::domain::services::LoginResponse,
        crate::domain::models::ProjectResponse,
        crate::domain::models::ProjectWithStats,
        crate::domain::models::CreateProjectRequest,
        crate::domain::models::UpdateProjectRequest,
        crate::domain::models::DatabaseResponse,
        crate::domain::models::CreateDatabaseRequest,
        crate::domain::models::UpdateDatabaseRequest,
        crate::domain::models::ConnectionInfo,
        crate::domain::models::ResourceLimits,
        crate::domain::models::LogQueryParams,
        crate::domain::models::LogsResponse,
        crate::domain::models::LogEntryResponse,
        crate::domain::models::LogType,
        crate::domain::models::DatabaseMetrics,
        crate::domain::models::QueryMetrics,
        crate::domain::models::RowMetrics,
        crate::domain::models::TableMetrics,
        crate::domain::models::StorageMetrics,
        crate::domain::models::ConnectionMetrics,
        crate::domain::models::ResourceMetrics,
        crate::domain::models::MetricsResponse,
        crate::domain::models::MetricsHistory,
        crate::domain::models::MetricsHistoryPoint,
        crate::domain::models::MetricsHistoryQuery,
        crate::domain::models::TimeRange,
        crate::domain::models::QueryLogEntry,
        crate::domain::models::QueryLogsResponse,
        crate::domain::models::QueryLogsQuery,
        crate::domain::models::MetricsStreamMessage,
        crate::domain::models::BranchInfo,
        crate::domain::models::BranchResponse,
        crate::domain::models::CreateBranchRequest,
        crate::domain::models::SchemaInfo,
        crate::domain::models::TableInfo,
        crate::domain::models::ViewInfo,
        crate::domain::models::ColumnDetail,
        crate::domain::models::IndexInfo,
        crate::domain::models::ExecuteQueryRequest,
        crate::domain::models::ColumnInfo,
        crate::domain::models::QueryResult,
        crate::domain::models::TablePreviewQuery,
        crate::domain::models::TablePreview,
    )),
    tags(
        (name = "Health", description = "Health check and system status endpoints"),
        (name = "Authentication", description = "User authentication and authorization endpoints"),
        (name = "Projects", description = "Project management endpoints"),
        (name = "Databases", description = "Database instance management endpoints"),
        (name = "Branches", description = "Database branching and forking endpoints"),
        (name = "Logs", description = "Database container logs and streaming endpoints"),
        (name = "Terminal", description = "Interactive terminal and psql access endpoints"),
        (name = "Metrics", description = "Database metrics and query statistics endpoints"),
        (name = "SQL", description = "SQL query execution and schema introspection endpoints")
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};

            let mut http = Http::new(HttpAuthScheme::Bearer);
            http.bearer_format = Some("JWT".to_string());
            http.description = Some("JWT Bearer token authentication".to_string());

            components.add_security_scheme("bearer", SecurityScheme::Http(http));
        }
    }
}

pub fn generate_openapi_json(
    output_path: impl AsRef<Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let openapi_json = ApiDoc::openapi().to_pretty_json()?;

    if let Some(parent) = output_path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(output_path.as_ref(), openapi_json)?;

    println!(
        "OpenAPI specification generated successfully at: {}",
        output_path.as_ref().display()
    );

    Ok(())
}

pub fn get_openapi_spec() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_generation() {
        let spec = get_openapi_spec();
        assert_eq!(spec.info.title, "Datify API");
        assert_eq!(spec.info.version, "0.1.0");
    }

    #[test]
    fn test_openapi_json_output() {
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_openapi.json");

        let result = generate_openapi_json(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());

        std::fs::remove_file(output_path).ok();
    }
}
