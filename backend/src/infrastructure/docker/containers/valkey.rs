use bollard::models::{ContainerCreateBody, HostConfig, RestartPolicy, RestartPolicyNameEnum};
use bollard::query_parameters::CreateContainerOptionsBuilder;
use bollard::Docker;

use super::{create_port_bindings, ContainerConfig, ContainerProvider};
use crate::error::{AppError, AppResult};

pub struct ValkeyContainer;

impl ContainerProvider for ValkeyContainer {
    fn default_image(version: &str) -> String {
        format!("valkey/valkey:{}-alpine", version)
    }

    fn internal_port() -> u16 {
        6379
    }

    fn data_mount_point() -> &'static str {
        "/data"
    }

    fn cli_command() -> Vec<&'static str> {
        vec!["valkey-cli"]
    }

    fn build_cmd(password: &str) -> Vec<String> {
        vec![
            "valkey-server".to_string(),
            "--requirepass".to_string(),
            password.to_string(),
            "--appendonly".to_string(),
            "yes".to_string(),
        ]
    }
}

impl ValkeyContainer {
    pub async fn create(
        docker: &Docker,
        config: ContainerConfig,
        password: &str,
        network_name: &str,
    ) -> AppResult<String> {
        let port_bindings = create_port_bindings(config.internal_port, config.exposed_port);

        let host_config = HostConfig {
            binds: Some(vec![format!(
                "{}:{}",
                config.data_path,
                Self::data_mount_point()
            )]),
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
        let cmd = Self::build_cmd(password);

        let container_body = ContainerCreateBody {
            image: Some(config.image.clone()),
            hostname: Some(config.name.clone()),
            env: Some(config.env.clone()),
            host_config: Some(host_config),
            exposed_ports: Some(exposed_ports),
            cmd: Some(cmd),
            ..Default::default()
        };

        let options = CreateContainerOptionsBuilder::default()
            .name(&config.name)
            .build();

        let container = docker
            .create_container(Some(options), container_body)
            .await
            .map_err(|e| AppError::Docker(format!("Failed to create Valkey container: {}", e)))?;

        tracing::info!(
            "Created Valkey container {} with ID {}",
            config.name,
            container.id
        );

        Ok(container.id)
    }
}

pub struct ValkeyVersion;

impl ValkeyVersion {
    pub fn is_valid(version: &str) -> bool {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() < 2 {
            return false;
        }
        parts[0].parse::<u32>().is_ok() && parts[1].parse::<u32>().is_ok()
    }
}
