pub mod api;
pub mod config;
pub mod domain;
pub mod error;
pub mod infrastructure;
pub mod middleware;
pub mod openapi;
pub mod repositories;

pub use api::create_router;
pub use config::Settings;
pub use domain::services::MetricsService;
pub use error::{AppError, AppResult};
pub use infrastructure::metrics_collector::spawn_metrics_collector;
pub use openapi::{generate_openapi_json, get_openapi_spec};
pub use repositories::{DatabaseRepository, MetricsRepository, Repositories};
