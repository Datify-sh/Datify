use bollard::models::{ContainerCreateBody, HostConfig, RestartPolicy, RestartPolicyNameEnum};
use bollard::query_parameters::CreateContainerOptionsBuilder;
use bollard::Docker;

use super::{create_port_bindings, ContainerConfig, ContainerProvider};
use crate::error::{AppError, AppResult};

pub struct PostgresContainer;

impl ContainerProvider for PostgresContainer {
    fn default_image(version: &str) -> String {
        format!("postgres:{}-alpine", version)
    }

    fn internal_port() -> u16 {
        5432
    }

    fn data_mount_point() -> &'static str {
        "/var/lib/postgresql/data"
    }

    fn cli_command() -> Vec<&'static str> {
        vec!["psql", "-U", "postgres", "-d", "postgres"]
    }

    fn build_cmd(_password: &str) -> Vec<String> {
        vec![
            "postgres".to_string(),
            "-c".to_string(),
            "shared_preload_libraries=pg_stat_statements".to_string(),
            "-c".to_string(),
            "pg_stat_statements.track=all".to_string(),
        ]
    }
}

impl PostgresContainer {
    pub fn is_postgres_18_or_later(image: &str) -> bool {
        if let Some(tag) = image.split(':').nth(1) {
            let version_str = tag.split('-').next().unwrap_or(tag);
            if let Ok(version) = version_str.parse::<u32>() {
                return version >= 18;
            }
        }
        false
    }

    pub fn get_mount_point(image: &str) -> &'static str {
        if Self::is_postgres_18_or_later(image) {
            "/var/lib/postgresql"
        } else {
            "/var/lib/postgresql/data"
        }
    }

    pub async fn create(
        docker: &Docker,
        config: ContainerConfig,
        password: &str,
        network_name: &str,
    ) -> AppResult<String> {
        let mut env = config.env.clone();
        env.push(format!("POSTGRES_PASSWORD={}", password));
        env.push("POSTGRES_HOST_AUTH_METHOD=scram-sha-256".to_string());

        let port_bindings = create_port_bindings(config.internal_port, config.exposed_port);
        let mount_point = Self::get_mount_point(&config.image);

        let host_config = HostConfig {
            binds: Some(vec![format!("{}:{}", config.data_path, mount_point)]),
            port_bindings: Some(port_bindings),
            network_mode: Some(network_name.to_string()),
            memory: Some(config.memory_limit_mb * 1024 * 1024),
            nano_cpus: Some((config.cpu_limit * 1_000_000_000.0) as i64),
            restart_policy: Some(RestartPolicy {
                name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
                maximum_retry_count: None,
            }),
            ..Default::default()
        };

        let exposed_ports = vec![format!("{}/tcp", config.internal_port)];

        let container_body = ContainerCreateBody {
            image: Some(config.image.clone()),
            hostname: Some(config.name.clone()),
            env: Some(env),
            host_config: Some(host_config),
            exposed_ports: Some(exposed_ports),
            cmd: config.cmd.clone(),
            ..Default::default()
        };

        let options = CreateContainerOptionsBuilder::default()
            .name(&config.name)
            .build();

        let container = docker
            .create_container(Some(options), container_body)
            .await
            .map_err(|e| {
                AppError::Docker(format!("Failed to create PostgreSQL container: {}", e))
            })?;

        tracing::info!(
            "Created PostgreSQL container {} with ID {}",
            config.name,
            container.id
        );

        Ok(container.id)
    }
}

pub struct PostgresVersion;

impl PostgresVersion {
    pub fn is_valid(version: &str) -> bool {
        version.parse::<u32>().is_ok()
    }
}
