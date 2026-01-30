use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use datify::config::Settings;
use datify::infrastructure::docker::DockerManager;
use datify::{spawn_metrics_collector, MetricsService, Repositories};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tokio::net::TcpListener;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file from project root
    dotenvy::dotenv().ok();

    let settings = Arc::new(Settings::new()?);

    init_logging(settings.as_ref());

    tracing::info!("Starting Datify v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!(
        "Configuration loaded from environment: {}",
        std::env::var("DATIFY_ENV").unwrap_or_else(|_| "default".into())
    );

    tracing::info!("Connecting to database: {}", settings.database.url);
    let db_pool = init_database(settings.as_ref()).await?;

    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .map_err(|e| anyhow::anyhow!("Migration failed: {}", e))?;
    tracing::info!("Migrations completed successfully");

    tracing::info!("Connecting to Docker daemon...");
    let docker = DockerManager::new(settings.clone()).await?;
    tracing::info!("Docker connection established");

    let repositories = Repositories::new(db_pool.clone());
    let docker_arc = std::sync::Arc::new(docker.clone());

    let metrics_service = std::sync::Arc::new(MetricsService::new(
        repositories.databases.clone(),
        repositories.projects.clone(),
        repositories.metrics.clone(),
        docker_arc.clone(),
        &settings.security.encryption_key,
    ));

    let shutdown_token = CancellationToken::new();

    let _metrics_collector = spawn_metrics_collector(
        metrics_service,
        repositories.databases.clone(),
        15,
        shutdown_token.clone(),
    );
    tracing::info!("Background metrics collector started");

    let app = datify::create_router(db_pool, docker, settings.clone()).await;

    let addr: SocketAddr = settings.server.address().parse()?;
    tracing::info!("Starting HTTP server on {}", addr);

    let listener = TcpListener::bind(addr).await?;

    tracing::info!("Datify is ready to accept connections");
    tracing::info!("API available at http://{}/api/v1", addr);
    tracing::info!("Health check at http://{}/health", addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal(shutdown_token))
    .await?;

    Ok(())
}

fn init_logging(settings: &Settings) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&settings.logging.level));

    let subscriber = tracing_subscriber::registry().with(env_filter);

    if settings.logging.format == "json" {
        subscriber
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        subscriber
            .with(tracing_subscriber::fmt::layer().pretty())
            .init();
    }
}

async fn init_database(settings: &Settings) -> anyhow::Result<sqlx::SqlitePool> {
    let connect_options: SqliteConnectOptions = settings.database.url.parse()?;
    let connect_options = connect_options
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(30));

    let pool = SqlitePoolOptions::new()
        .max_connections(settings.database.max_connections)
        .min_connections(settings.database.min_connections)
        .acquire_timeout(Duration::from_secs(30))
        .connect_with(connect_options)
        .await?;

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;

    Ok(pool)
}

async fn shutdown_signal(cancel_token: CancellationToken) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown");
    cancel_token.cancel();
}
