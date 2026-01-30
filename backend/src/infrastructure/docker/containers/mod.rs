mod postgres;
mod redis;
mod valkey;

use std::collections::HashMap;

use bollard::models::{HostConfig, PortBinding, RestartPolicy, RestartPolicyNameEnum};
pub use postgres::*;
pub use redis::*;
pub use valkey::*;

#[derive(Debug, Clone)]
pub struct ContainerConfig {
    pub name: String,
    pub image: String,
    pub env: Vec<String>,
    pub data_path: String,
    pub cpu_limit: f64,
    pub memory_limit_mb: i64,
    pub internal_port: u16,
    pub exposed_port: Option<u16>,
    pub cmd: Option<Vec<String>>,
}

pub trait ContainerProvider {
    fn default_image(version: &str) -> String;
    fn internal_port() -> u16;
    fn data_mount_point() -> &'static str;
    fn cli_command() -> Vec<&'static str>;
    fn build_cmd(password: &str) -> Vec<String>;
}

pub fn create_port_bindings(
    internal_port: u16,
    exposed_port: Option<u16>,
) -> HashMap<String, Option<Vec<PortBinding>>> {
    let mut port_bindings = HashMap::new();
    if let Some(port) = exposed_port {
        port_bindings.insert(
            format!("{}/tcp", internal_port),
            Some(vec![PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: Some(port.to_string()),
            }]),
        );
    }
    port_bindings
}

pub fn create_host_config(
    data_path: &str,
    mount_point: &str,
    port_bindings: HashMap<String, Option<Vec<PortBinding>>>,
    network_name: &str,
    memory_limit_mb: i64,
    cpu_limit: f64,
) -> HostConfig {
    HostConfig {
        binds: Some(vec![format!("{}:{}", data_path, mount_point)]),
        port_bindings: Some(port_bindings),
        network_mode: Some(network_name.to_string()),
        memory: Some(memory_limit_mb * 1024 * 1024),
        nano_cpus: Some((cpu_limit * 1_000_000_000.0) as i64),
        restart_policy: Some(RestartPolicy {
            name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
            maximum_retry_count: None,
        }),
        ..Default::default()
    }
}
