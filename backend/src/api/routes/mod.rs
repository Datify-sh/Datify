use std::sync::Arc;

use axum::http::{header, HeaderValue, Method};
use axum::{
    middleware, Extension,
    routing::{get, post},
    Router,
};
use sqlx::sqlite::SqlitePool;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::api::handlers::{
    self, AuditLogServiceState, AuthServiceState, DatabaseServiceState, HealthState, LogsState,
    MetricsState, ProjectServiceState, SqlState, TerminalState,
};
use crate::config::Settings;
use crate::domain::services::{
    AuditLogService, AuthService, DatabaseService, MetricsService, ProjectService, SqlService,
};
use crate::infrastructure::docker::DockerManager;
use crate::middleware::{
    auth_middleware, auth_rate_limit_middleware, rate_limit_middleware,
    security_headers_middleware, AuthState, RateLimitState,
};
use crate::repositories::Repositories;

pub async fn create_router(
    db_pool: SqlitePool,
    docker: DockerManager,
    settings: Arc<Settings>,
) -> Router {
    let repositories = Repositories::new(db_pool.clone());

    let auth_service = Arc::new(AuthService::new(
        repositories.users.clone(),
        repositories.tokens.clone(),
        settings.clone(),
    ));

    let project_service = Arc::new(ProjectService::new(
        repositories.projects.clone(),
        repositories.databases.clone(),
    ));

    let docker = Arc::new(docker);

    let database_service = Arc::new(DatabaseService::new(
        repositories.databases.clone(),
        repositories.projects.clone(),
        docker.clone(),
        settings.docker.data_dir.clone(),
        settings.docker.public_host.clone(),
        &settings.security.encryption_key,
    ));

    let metrics_service = Arc::new(MetricsService::new(
        repositories.databases.clone(),
        repositories.projects.clone(),
        repositories.metrics.clone(),
        docker.clone(),
        &settings.security.encryption_key,
    ));

    let sql_service = Arc::new(SqlService::new(
        repositories.databases.clone(),
        repositories.projects.clone(),
        docker.clone(),
        &settings.security.encryption_key,
    ));

    let audit_log_service = Arc::new(AuditLogService::new(repositories.audit_logs.clone()));

    let auth_state = AuthState {
        auth_service: auth_service.clone(),
    };

    let rate_limit_state = RateLimitState::new(settings.as_ref());

    let health_state = HealthState {
        db_pool: db_pool.clone(),
        docker: docker.clone(),
    };

    let system_routes = Router::new()
        .route(
            "/",
            get(handlers::system_info).with_state(settings.docker.public_host.clone()),
        )
        .route("/postgres-versions", get(handlers::get_postgres_versions))
        .route("/valkey-versions", get(handlers::get_valkey_versions))
        .route("/redis-versions", get(handlers::get_redis_versions));

    let public_routes = Router::new()
        .route("/health", get(handlers::health))
        .route("/ready", get(handlers::ready).with_state(health_state));

    let auth_routes = Router::new()
        .route("/register", post(handlers::register))
        .route("/login", post(handlers::login))
        .route("/refresh", post(handlers::refresh))
        .layer(Extension(audit_log_service.clone()))
        .layer(middleware::from_fn_with_state(
            rate_limit_state.clone(),
            auth_rate_limit_middleware,
        ))
        .with_state(auth_service.clone() as AuthServiceState);

    let me_routes = Router::new()
        .route("/me", get(handlers::me))
        .with_state(auth_service.clone() as AuthServiceState);

    let logout_routes = Router::new()
        .route("/logout", post(handlers::logout))
        .route("/logout-all", post(handlers::logout_all))
        .with_state(auth_service.clone() as AuthServiceState);

    let project_routes = Router::new()
        .route(
            "/",
            get(handlers::list_projects).post(handlers::create_project),
        )
        .route(
            "/{id}",
            get(handlers::get_project)
                .put(handlers::update_project)
                .delete(handlers::delete_project),
        )
        .with_state(project_service.clone() as ProjectServiceState);

    let project_database_routes = Router::new()
        .route(
            "/",
            get(handlers::list_databases).post(handlers::create_database),
        )
        .with_state(database_service.clone() as DatabaseServiceState);

    let logs_state = LogsState {
        database_service: database_service.clone(),
        docker: docker.clone(),
    };

    let terminal_state = TerminalState {
        database_service: database_service.clone(),
        docker: docker.clone(),
    };

    let metrics_state = MetricsState {
        metrics_service: metrics_service.clone(),
    };

    let sql_state = SqlState {
        sql_service: sql_service.clone(),
    };

    let database_routes = Router::new()
        .route(
            "/{id}",
            get(handlers::get_database)
                .put(handlers::update_database)
                .delete(handlers::delete_database),
        )
        .route("/{id}/start", post(handlers::start_database))
        .route("/{id}/stop", post(handlers::stop_database))
        .route(
            "/{id}/change-password",
            post(handlers::change_database_password),
        )
        .route(
            "/{id}/branches",
            get(handlers::list_branches).post(handlers::create_branch),
        )
        .route("/{id}/sync-from-parent", post(handlers::sync_from_parent))
        .with_state(database_service.clone() as DatabaseServiceState);

    let logs_routes = Router::new()
        .route("/{id}/logs", get(handlers::get_database_logs))
        .route("/{id}/logs/stream", get(handlers::stream_database_logs))
        .with_state(logs_state);

    let terminal_routes = Router::new()
        .route("/{id}/terminal", get(handlers::database_terminal))
        .route("/{id}/psql", get(handlers::database_psql))
        .route("/{id}/valkey-cli", get(handlers::database_valkey_cli))
        .route("/{id}/redis-cli", get(handlers::database_redis_cli))
        .with_state(terminal_state);

    let metrics_routes = Router::new()
        .route("/{id}/metrics", get(handlers::get_database_metrics))
        .route(
            "/{id}/metrics/history",
            get(handlers::get_database_metrics_history),
        )
        .route(
            "/{id}/metrics/stream",
            get(handlers::stream_database_metrics),
        )
        .route("/{id}/queries", get(handlers::get_database_queries))
        .with_state(metrics_state);

    let sql_routes = Router::new()
        .route("/{id}/schema", get(handlers::get_database_schema))
        .route("/{id}/query", post(handlers::execute_query))
        .route(
            "/{id}/tables/{schema}/{table}/preview",
            get(handlers::preview_table),
        )
        .with_state(sql_state);

    let audit_log_routes = Router::new()
        .route("/", get(handlers::list_audit_logs))
        .with_state(audit_log_service.clone() as AuditLogServiceState);

    let protected_routes = Router::new()
        .merge(me_routes)
        .nest("/auth", logout_routes)
        .nest("/system", system_routes)
        .nest("/projects", project_routes)
        .nest("/projects/{project_id}/databases", project_database_routes)
        .nest("/databases", database_routes)
        .nest("/databases", logs_routes)
        .nest("/databases", terminal_routes)
        .nest("/databases", metrics_routes)
        .nest("/databases", sql_routes)
        .nest("/audit-logs", audit_log_routes)
        .layer(Extension(audit_log_service.clone()))
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_middleware,
        ));

    let api_v1 = Router::new()
        .nest("/auth", auth_routes)
        .merge(protected_routes);

    let api_routes = Router::new()
        .merge(public_routes)
        .nest("/api/v1", api_v1)
        .layer(middleware::from_fn_with_state(
            rate_limit_state,
            rate_limit_middleware,
        ));

    let static_dir = std::path::Path::new("public");
    let app = if static_dir.exists() {
        tracing::info!("Serving frontend from ./public");
        let serve_dir =
            ServeDir::new("public").not_found_service(ServeFile::new("public/index.html"));
        api_routes.fallback_service(serve_dir)
    } else {
        tracing::warn!("Frontend directory ./public not found, serving API only");
        api_routes
    };

    let origins: Vec<HeaderValue> = settings
        .cors
        .allowed_origins
        .0
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();

    app.layer(middleware::from_fn(security_headers_middleware))
        .layer(
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::COOKIE])
                .allow_credentials(true),
        )
        .layer(TraceLayer::new_for_http())
}
