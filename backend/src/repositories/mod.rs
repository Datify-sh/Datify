mod audit_log;
mod database;
mod metrics;
mod project;
mod token;
mod user;

pub use audit_log::AuditLogRepository;
pub use database::DatabaseRepository;
pub use metrics::MetricsRepository;
pub use project::ProjectRepository;
use sqlx::sqlite::SqlitePool;
pub use token::TokenRepository;
pub use user::UserRepository;

#[derive(Clone)]
pub struct Repositories {
    pub users: UserRepository,
    pub projects: ProjectRepository,
    pub databases: DatabaseRepository,
    pub metrics: MetricsRepository,
    pub tokens: TokenRepository,
    pub audit_logs: AuditLogRepository,
}

impl Repositories {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            users: UserRepository::new(pool.clone()),
            projects: ProjectRepository::new(pool.clone()),
            databases: DatabaseRepository::new(pool.clone()),
            metrics: MetricsRepository::new(pool.clone()),
            tokens: TokenRepository::new(pool.clone()),
            audit_logs: AuditLogRepository::new(pool),
        }
    }
}
