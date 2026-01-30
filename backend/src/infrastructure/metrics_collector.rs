use std::sync::Arc;
use std::time::Duration;

use tokio::time;
use tokio_util::sync::CancellationToken;

use crate::domain::services::MetricsService;
use crate::repositories::DatabaseRepository;

pub struct MetricsCollector {
    metrics_service: Arc<MetricsService>,
    database_repo: DatabaseRepository,
    interval: Duration,
    cancel_token: CancellationToken,
}

impl MetricsCollector {
    pub fn new(
        metrics_service: Arc<MetricsService>,
        database_repo: DatabaseRepository,
        interval_secs: u64,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            metrics_service,
            database_repo,
            interval: Duration::from_secs(interval_secs),
            cancel_token,
        }
    }

    pub async fn run(self) {
        tracing::info!(
            "Starting metrics collector with {}s interval",
            self.interval.as_secs()
        );

        let mut interval = time::interval(self.interval);

        loop {
            tokio::select! {
                _ = self.cancel_token.cancelled() => {
                    tracing::info!("Metrics collector shutting down");
                    break;
                }
                _ = interval.tick() => {
                    if let Err(e) = self.collect_all_metrics().await {
                        tracing::error!("Error collecting metrics: {}", e);
                    }
                }
            }
        }
    }

    async fn collect_all_metrics(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let databases = self
            .database_repo
            .find_all_running()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let count = databases.len();
        if count == 0 {
            return Ok(());
        }

        tracing::debug!("Collecting metrics for {} running databases", count);

        for database in databases {
            if self.cancel_token.is_cancelled() {
                break;
            }
            if let Err(e) = self.metrics_service.snapshot_metrics(&database.id).await {
                tracing::warn!(
                    "Failed to collect metrics for database {}: {}",
                    database.id,
                    e
                );
            }
        }

        Ok(())
    }
}

pub fn spawn_metrics_collector(
    metrics_service: Arc<MetricsService>,
    database_repo: DatabaseRepository,
    interval_secs: u64,
    cancel_token: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    let collector =
        MetricsCollector::new(metrics_service, database_repo, interval_secs, cancel_token);

    tokio::spawn(async move {
        collector.run().await;
    })
}
