use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use bollard::exec::StartExecOptions;
use bollard::models::{
    ContainerCreateBody, CreateImageInfo, ExecConfig, HostConfig, NetworkCreateRequest,
    PortBinding, RestartPolicy, RestartPolicyNameEnum,
};
use bollard::query_parameters::{
    CreateContainerOptionsBuilder, CreateImageOptionsBuilder, ListContainersOptionsBuilder,
    ListNetworksOptionsBuilder, LogsOptionsBuilder, RemoveContainerOptionsBuilder,
    ResizeExecOptions, StatsOptionsBuilder, StopContainerOptionsBuilder,
};
use bollard::Docker;
use bytes::Bytes;
use futures::{Stream, StreamExt, TryStreamExt};

use crate::config::Settings;
use crate::error::{AppError, AppResult};

const MAX_LOG_LINES: usize = 10_000;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ImagePullProgress {
    pub status: String,
    pub progress: Option<String>,
    pub id: Option<String>,
    pub current: Option<i64>,
    pub total: Option<i64>,
}

impl From<CreateImageInfo> for ImagePullProgress {
    fn from(info: CreateImageInfo) -> Self {
        let current = info.progress_detail.as_ref().and_then(|p| p.current);
        let total = info.progress_detail.as_ref().and_then(|p| p.total);
        let progress = match (current, total) {
            (Some(c), Some(t)) if t > 0 => Some(format!("{}/{}", c, t)),
            _ => None,
        };
        Self {
            status: info.status.unwrap_or_default(),
            progress,
            id: info.id,
            current,
            total,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LogEntry {
    pub timestamp: Option<String>,
    pub stream: String,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ContainerLogs {
    pub container_id: String,
    pub entries: Vec<LogEntry>,
    pub has_more: bool,
}

#[derive(Clone)]
pub struct DockerManager {
    docker: Arc<Docker>,
    settings: Arc<Settings>,
}

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

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub ports: HashMap<String, Option<Vec<PortBinding>>>,
}

impl DockerManager {
    pub async fn new(settings: Arc<Settings>) -> AppResult<Self> {
        let docker = Docker::connect_with_socket(
            &settings.docker.socket_path,
            120,
            bollard::API_DEFAULT_VERSION,
        )
        .map_err(|e| AppError::Docker(format!("Failed to connect to Docker: {}", e)))?;

        let manager = Self {
            docker: Arc::new(docker),
            settings,
        };

        manager.ensure_network().await?;

        Ok(manager)
    }

    async fn ensure_network(&self) -> AppResult<()> {
        let network_name = &self.settings.docker.network_name;

        let options = ListNetworksOptionsBuilder::default().build();
        let networks = self
            .docker
            .list_networks(Some(options))
            .await
            .map_err(|e| AppError::Docker(format!("Failed to list networks: {}", e)))?;

        let exists = networks
            .iter()
            .any(|n| n.name.as_deref() == Some(network_name));

        if !exists {
            tracing::info!("Creating Docker network: {}", network_name);
            let network_config = NetworkCreateRequest {
                name: network_name.clone(),
                driver: Some("bridge".to_string()),
                ..Default::default()
            };
            self.docker
                .create_network(network_config)
                .await
                .map_err(|e| AppError::Docker(format!("Failed to create network: {}", e)))?;
        }

        Ok(())
    }

    /// Check if the postgres image is version 18 or later
    /// PostgreSQL 18+ uses a different data directory structure
    fn is_postgres_18_or_later(image: &str) -> bool {
        // Parse version from image tag like "postgres:18" or "postgres:18-alpine"
        if let Some(tag) = image.split(':').nth(1) {
            // Extract the major version number
            let version_str = tag.split('-').next().unwrap_or(tag);
            if let Ok(version) = version_str.parse::<u32>() {
                return version >= 18;
            }
        }
        false
    }

    pub async fn pull_image(&self, image: &str) -> AppResult<()> {
        tracing::info!("Pulling Docker image: {}", image);

        let options = CreateImageOptionsBuilder::default()
            .from_image(image)
            .build();

        self.docker
            .create_image(Some(options), None, None)
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| AppError::Docker(format!("Failed to pull image: {}", e)))?;

        Ok(())
    }

    pub async fn create_postgres_container(
        &self,
        config: ContainerConfig,
        password: &str,
    ) -> AppResult<String> {
        self.pull_image(&config.image).await?;

        let mut env = config.env.clone();
        env.push(format!("POSTGRES_PASSWORD={}", password));
        env.push("POSTGRES_HOST_AUTH_METHOD=scram-sha-256".to_string());

        let mut port_bindings = HashMap::new();
        if let Some(exposed_port) = config.exposed_port {
            port_bindings.insert(
                format!("{}/tcp", config.internal_port),
                Some(vec![PortBinding {
                    host_ip: Some("0.0.0.0".to_string()),
                    host_port: Some(exposed_port.to_string()),
                }]),
            );
        }

        // PostgreSQL 18+ changed data directory structure
        // See: https://github.com/docker-library/postgres/pull/1259
        let mount_point = if Self::is_postgres_18_or_later(&config.image) {
            "/var/lib/postgresql"
        } else {
            "/var/lib/postgresql/data"
        };

        let host_config = HostConfig {
            binds: Some(vec![format!("{}:{}", config.data_path, mount_point)]),
            port_bindings: Some(port_bindings),
            network_mode: Some(self.settings.docker.network_name.clone()),
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

        let container = self
            .docker
            .create_container(Some(options), container_body)
            .await
            .map_err(|e| AppError::Docker(format!("Failed to create container: {}", e)))?;

        tracing::info!("Created container {} with ID {}", config.name, container.id);

        Ok(container.id)
    }

    pub async fn create_valkey_container(
        &self,
        config: ContainerConfig,
        password: &str,
    ) -> AppResult<String> {
        self.pull_image(&config.image).await?;

        let mut port_bindings = HashMap::new();
        if let Some(exposed_port) = config.exposed_port {
            port_bindings.insert(
                format!("{}/tcp", config.internal_port),
                Some(vec![PortBinding {
                    host_ip: Some("0.0.0.0".to_string()),
                    host_port: Some(exposed_port.to_string()),
                }]),
            );
        }

        let host_config = HostConfig {
            binds: Some(vec![format!("{}:/data", config.data_path)]),
            port_bindings: Some(port_bindings),
            network_mode: Some(self.settings.docker.network_name.clone()),
            memory: Some(config.memory_limit_mb * 1024 * 1024),
            nano_cpus: Some((config.cpu_limit * 1_000_000_000.0) as i64),
            restart_policy: Some(RestartPolicy {
                name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
                maximum_retry_count: None,
            }),
            ..Default::default()
        };

        let exposed_ports = vec![format!("{}/tcp", config.internal_port)];

        let cmd = vec![
            "valkey-server".to_string(),
            "--requirepass".to_string(),
            password.to_string(),
            "--appendonly".to_string(),
            "yes".to_string(),
        ];

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

        let container = self
            .docker
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

    pub fn valkey_image(&self) -> &str {
        &self.settings.docker.valkey_image
    }

    pub async fn create_pgbouncer_container(
        &self,
        name: &str,
        postgres_container: &str,
        postgres_password: &str,
        exposed_port: u16,
        memory_limit_mb: i64,
    ) -> AppResult<String> {
        let image = &self.settings.docker.pgbouncer_image;
        self.pull_image(image).await?;

        let env = vec![
            format!(
                "DATABASE_URL=postgres://postgres:{}@{}:5432/postgres?connect_timeout=10",
                postgres_password, postgres_container
            ),
            "POOL_MODE=transaction".to_string(),
            "MAX_CLIENT_CONN=100".to_string(),
            "DEFAULT_POOL_SIZE=20".to_string(),
            "AUTH_TYPE=scram-sha-256".to_string(),
        ];

        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            "5432/tcp".to_string(),
            Some(vec![PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: Some(exposed_port.to_string()),
            }]),
        );

        let host_config = HostConfig {
            port_bindings: Some(port_bindings),
            network_mode: Some(self.settings.docker.network_name.clone()),
            memory: Some(memory_limit_mb * 1024 * 1024),
            restart_policy: Some(RestartPolicy {
                name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
                maximum_retry_count: None,
            }),
            ..Default::default()
        };

        let container_body = ContainerCreateBody {
            image: Some(image.clone()),
            hostname: Some(name.to_string()),
            env: Some(env),
            host_config: Some(host_config),
            ..Default::default()
        };

        let options = CreateContainerOptionsBuilder::default().name(name).build();

        let container = self
            .docker
            .create_container(Some(options), container_body)
            .await
            .map_err(|e| {
                AppError::Docker(format!("Failed to create PgBouncer container: {}", e))
            })?;

        tracing::info!(
            "Created PgBouncer container {} with ID {}",
            name,
            container.id
        );

        Ok(container.id)
    }

    pub async fn start_container(&self, container_id: &str) -> AppResult<()> {
        self.docker
            .start_container(container_id, None)
            .await
            .map_err(|e| AppError::Docker(format!("Failed to start container: {}", e)))?;

        tracing::info!("Started container {}", container_id);
        Ok(())
    }

    pub async fn stop_container(&self, container_id: &str) -> AppResult<()> {
        let options = StopContainerOptionsBuilder::default().t(30).build();
        self.docker
            .stop_container(container_id, Some(options))
            .await
            .map_err(|e| AppError::Docker(format!("Failed to stop container: {}", e)))?;

        tracing::info!("Stopped container {}", container_id);
        Ok(())
    }

    pub async fn remove_container(&self, container_id: &str, force: bool) -> AppResult<()> {
        let options = RemoveContainerOptionsBuilder::default()
            .force(force)
            .v(true)
            .build();

        self.docker
            .remove_container(container_id, Some(options))
            .await
            .map_err(|e| AppError::Docker(format!("Failed to remove container: {}", e)))?;

        tracing::info!("Removed container {}", container_id);
        Ok(())
    }

    pub async fn get_container_status(&self, container_id: &str) -> AppResult<String> {
        let container = self
            .docker
            .inspect_container(container_id, None)
            .await
            .map_err(|e| AppError::Docker(format!("Failed to inspect container: {}", e)))?;

        let state = container
            .state
            .and_then(|s| s.status)
            .map(|s| format!("{:?}", s).to_lowercase())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(state)
    }

    pub async fn container_exists(&self, container_id: &str) -> AppResult<bool> {
        match self.docker.inspect_container(container_id, None).await {
            Ok(_) => Ok(true),
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => Ok(false),
            Err(e) => Err(AppError::Docker(format!(
                "Failed to check container: {}",
                e
            ))),
        }
    }

    pub async fn list_containers(
        &self,
        name_filter: Option<&str>,
    ) -> AppResult<Vec<ContainerInfo>> {
        let mut builder = ListContainersOptionsBuilder::default().all(true);

        if let Some(name) = name_filter {
            let mut filters = HashMap::new();
            filters.insert("name", vec![name]);
            builder = builder.filters(&filters);
        }

        let options = builder.build();

        let containers = self
            .docker
            .list_containers(Some(options))
            .await
            .map_err(|e| AppError::Docker(format!("Failed to list containers: {}", e)))?;

        let infos = containers
            .into_iter()
            .map(|c| {
                let ports_map: HashMap<String, Option<Vec<PortBinding>>> = c
                    .ports
                    .map(|ports| {
                        ports
                            .into_iter()
                            .map(|p| {
                                let pp = p.private_port;
                                let binding = p.public_port.map(|pub_port| {
                                    vec![PortBinding {
                                        host_ip: p.ip.clone(),
                                        host_port: Some(pub_port.to_string()),
                                    }]
                                });
                                (format!("{}/tcp", pp), binding)
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                ContainerInfo {
                    id: c.id.unwrap_or_default(),
                    name: c
                        .names
                        .and_then(|n: Vec<String>| n.first().cloned())
                        .unwrap_or_default()
                        .trim_start_matches('/')
                        .to_string(),
                    status: c.status.unwrap_or_default(),
                    ports: ports_map,
                }
            })
            .collect();

        Ok(infos)
    }

    pub async fn wait_for_healthy(
        &self,
        container_id: &str,
        timeout_seconds: u64,
    ) -> AppResult<bool> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_seconds);

        while start.elapsed() < timeout {
            let status = self.get_container_status(container_id).await?;

            if status == "running" {
                return Ok(true);
            }

            if status == "exited" || status == "dead" {
                return Ok(false);
            }

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        Ok(false)
    }

    pub fn postgres_image(&self) -> &str {
        &self.settings.docker.postgres_image
    }

    pub fn pull_image_with_progress(
        &self,
        image: &str,
    ) -> Pin<Box<dyn Stream<Item = Result<ImagePullProgress, AppError>> + Send + '_>> {
        let options = CreateImageOptionsBuilder::default()
            .from_image(image)
            .build();

        let stream =
            self.docker
                .create_image(Some(options), None, None)
                .map(|result| match result {
                    Ok(info) => Ok(ImagePullProgress::from(info)),
                    Err(e) => Err(AppError::Docker(format!("Failed to pull image: {}", e))),
                });

        Box::pin(stream)
    }

    pub async fn get_container_logs(
        &self,
        container_id: &str,
        tail: Option<i64>,
        since: Option<i64>,
        timestamps: bool,
    ) -> AppResult<ContainerLogs> {
        let mut builder = LogsOptionsBuilder::default()
            .stdout(true)
            .stderr(true)
            .timestamps(timestamps);

        let tail = tail.filter(|n| *n > 0).map(|n| n.min(MAX_LOG_LINES as i64));

        if let Some(n) = tail {
            builder = builder.tail(n.to_string().as_str());
        }

        if let Some(ts) = since {
            builder = builder.since(ts as i32);
        }

        let options = builder.build();

        let mut log_stream = self.docker.logs(container_id, Some(options));
        let capacity = tail.unwrap_or(100) as usize;
        let mut entries = Vec::with_capacity(capacity);
        let mut has_more = false;

        while let Some(result) = log_stream.next().await {
            let output =
                result.map_err(|e| AppError::Docker(format!("Failed to get logs: {}", e)))?;

            if entries.len() >= MAX_LOG_LINES {
                has_more = true;
                break;
            }

            entries.push(parse_log_output(output, timestamps));
        }

        Ok(ContainerLogs {
            container_id: container_id.to_string(),
            entries,
            has_more,
        })
    }

    pub fn stream_container_logs(
        &self,
        container_id: &str,
        tail: Option<i64>,
    ) -> Pin<Box<dyn Stream<Item = Result<LogEntry, AppError>> + Send + '_>> {
        let mut builder = LogsOptionsBuilder::default()
            .stdout(true)
            .stderr(true)
            .follow(true)
            .timestamps(true);

        let tail = tail.filter(|n| *n > 0).map(|n| n.min(MAX_LOG_LINES as i64));

        if let Some(n) = tail {
            builder = builder.tail(n.to_string().as_str());
        } else {
            builder = builder.tail("100");
        }

        let options = builder.build();
        let container_id = container_id.to_string();

        let stream =
            self.docker
                .logs(&container_id, Some(options))
                .map(move |result| match result {
                    Ok(output) => Ok(parse_log_output(output, true)),
                    Err(e) => Err(AppError::Docker(format!("Log stream error: {}", e))),
                });

        Box::pin(stream)
    }

    pub async fn create_exec(
        &self,
        container_id: &str,
        cmd: Vec<&str>,
        tty: bool,
    ) -> AppResult<String> {
        let config = ExecConfig {
            attach_stdin: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(tty),
            cmd: Some(cmd.into_iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        };

        let exec = self
            .docker
            .create_exec(container_id, config)
            .await
            .map_err(|e| AppError::Docker(format!("Failed to create exec: {}", e)))?;

        Ok(exec.id)
    }

    pub async fn start_exec(
        &self,
        exec_id: &str,
        tty: bool,
    ) -> AppResult<Pin<Box<dyn Stream<Item = Result<Bytes, AppError>> + Send>>> {
        let options = StartExecOptions {
            detach: false,
            tty,
            output_capacity: None,
        };

        let stream = self
            .docker
            .start_exec(exec_id, Some(options))
            .await
            .map_err(|e| AppError::Docker(format!("Failed to start exec: {}", e)))?;

        match stream {
            bollard::exec::StartExecResults::Attached { output, .. } => {
                let mapped_stream = output.map(|result| match result {
                    Ok(output) => {
                        let bytes = match output {
                            bollard::container::LogOutput::StdOut { message } => message,
                            bollard::container::LogOutput::StdErr { message } => message,
                            bollard::container::LogOutput::StdIn { message } => message,
                            bollard::container::LogOutput::Console { message } => message,
                        };
                        Ok(bytes)
                    },
                    Err(e) => Err(AppError::Docker(format!("Exec stream error: {}", e))),
                });
                Ok(Box::pin(mapped_stream))
            },
            bollard::exec::StartExecResults::Detached => Err(AppError::Docker(
                "Exec started in detached mode".to_string(),
            )),
        }
    }

    pub async fn resize_exec(&self, exec_id: &str, width: u16, height: u16) -> AppResult<()> {
        self.docker
            .resize_exec(
                exec_id,
                ResizeExecOptions {
                    w: width as i32,
                    h: height as i32,
                },
            )
            .await
            .map_err(|e| AppError::Docker(format!("Failed to resize exec: {}", e)))?;
        Ok(())
    }

    pub fn docker(&self) -> &Docker {
        &self.docker
    }

    pub async fn fork_database(
        &self,
        source_container: &str,
        target_container: &str,
        source_username: &str,
        source_password: &str,
        target_username: &str,
        target_password: &str,
    ) -> AppResult<()> {
        tracing::info!(
            "Forking database from {} to {}",
            source_container,
            target_container
        );

        let dump_cmd = format!(
            "PGPASSWORD='{}' PGCONNECT_TIMEOUT=10 pg_dump -h {} -U {} -d postgres -Fc",
            source_password, source_container, source_username
        );

        let restore_cmd = format!(
            "PGPASSWORD='{}' PGCONNECT_TIMEOUT=10 pg_restore -h localhost -U {} -d postgres \
             --clean --if-exists --no-owner --no-privileges",
            target_password, target_username
        );

        let full_cmd = format!("{} | {}", dump_cmd, restore_cmd);

        let exec_config = ExecConfig {
            attach_stdin: Some(false),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(false),
            cmd: Some(vec!["sh".to_string(), "-c".to_string(), full_cmd]),
            ..Default::default()
        };

        let exec = self
            .docker
            .create_exec(target_container, exec_config)
            .await
            .map_err(|e| AppError::Docker(format!("Failed to create fork exec: {}", e)))?;

        let output = self
            .docker
            .start_exec(&exec.id, None)
            .await
            .map_err(|e| AppError::Docker(format!("Failed to start fork exec: {}", e)))?;

        if let bollard::exec::StartExecResults::Attached { mut output, .. } = output {
            while let Some(result) = output.next().await {
                match result {
                    Ok(log) => {
                        let msg = match log {
                            bollard::container::LogOutput::StdOut { message } => {
                                String::from_utf8_lossy(&message).to_string()
                            },
                            bollard::container::LogOutput::StdErr { message } => {
                                String::from_utf8_lossy(&message).to_string()
                            },
                            _ => String::new(),
                        };
                        if !msg.is_empty() {
                            tracing::debug!("Fork output: {}", msg.trim());
                        }
                    },
                    Err(e) => {
                        tracing::warn!("Fork stream error: {}", e);
                    },
                }
            }
        }

        let exec_inspect = self
            .docker
            .inspect_exec(&exec.id)
            .await
            .map_err(|e| AppError::Docker(format!("Failed to inspect fork exec: {}", e)))?;

        if let Some(exit_code) = exec_inspect.exit_code {
            if exit_code != 0 {
                return Err(AppError::Docker(format!(
                    "Fork failed with exit code {}",
                    exit_code
                )));
            }
        }

        tracing::info!("Database fork completed successfully");
        Ok(())
    }

    pub async fn get_container_stats(&self, container_id: &str) -> AppResult<ContainerStats> {
        let options = StatsOptionsBuilder::default()
            .stream(false)
            .one_shot(true)
            .build();

        let mut stats_stream = self.docker.stats(container_id, Some(options));

        if let Some(stats_result) = stats_stream.next().await {
            let stats = stats_result
                .map_err(|e| AppError::Docker(format!("Failed to get container stats: {}", e)))?;

            let cpu_percent = calculate_cpu_percent(&stats);
            let (memory_used, memory_limit, memory_percent) = calculate_memory_stats(&stats);

            Ok(ContainerStats {
                cpu_percent,
                memory_used_bytes: memory_used,
                memory_limit_bytes: memory_limit,
                memory_percent,
            })
        } else {
            Err(AppError::Docker(
                "No stats available for container".to_string(),
            ))
        }
    }
}

fn parse_log_output(output: bollard::container::LogOutput, timestamps: bool) -> LogEntry {
    let (stream, message) = match output {
        bollard::container::LogOutput::StdOut { message } => ("stdout", message),
        bollard::container::LogOutput::StdErr { message } => ("stderr", message),
        bollard::container::LogOutput::StdIn { message } => ("stdin", message),
        bollard::container::LogOutput::Console { message } => ("console", message),
    };

    let mut message_str = String::from_utf8_lossy(&message).into_owned();

    if timestamps && message_str.len() > 30 {
        if let Some(space_index) = message_str.find(' ') {
            let timestamp = Some(message_str[..space_index].to_string());
            let mut msg = message_str[space_index + 1..].to_string();
            let trimmed_len = msg.trim_end().len();
            msg.truncate(trimmed_len);
            return LogEntry {
                timestamp,
                stream: stream.to_string(),
                message: msg,
            };
        }
    }

    let trimmed_len = message_str.trim_end().len();
    message_str.truncate(trimmed_len);

    LogEntry {
        timestamp: None,
        stream: stream.to_string(),
        message: message_str,
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ContainerStats {
    pub cpu_percent: f64,
    pub memory_used_bytes: i64,
    pub memory_limit_bytes: i64,
    pub memory_percent: f64,
}

fn calculate_cpu_percent(stats: &bollard::models::ContainerStatsResponse) -> f64 {
    let cpu_stats = match &stats.cpu_stats {
        Some(c) => c,
        None => return 0.0,
    };
    let precpu_stats = match &stats.precpu_stats {
        Some(p) => p,
        None => return 0.0,
    };

    let cpu_usage = cpu_stats.cpu_usage.as_ref();
    let precpu_usage = precpu_stats.cpu_usage.as_ref();

    let cpu_delta = match (cpu_usage, precpu_usage) {
        (Some(c), Some(p)) => c.total_usage.unwrap_or(0) as f64 - p.total_usage.unwrap_or(0) as f64,
        _ => return 0.0,
    };

    let system_delta = cpu_stats.system_cpu_usage.unwrap_or(0) as f64
        - precpu_stats.system_cpu_usage.unwrap_or(0) as f64;

    let num_cpus = cpu_stats.online_cpus.unwrap_or(1) as f64;

    if system_delta > 0.0 && cpu_delta > 0.0 {
        (cpu_delta / system_delta) * num_cpus * 100.0
    } else {
        0.0
    }
}

fn calculate_memory_stats(stats: &bollard::models::ContainerStatsResponse) -> (i64, i64, f64) {
    let memory_stats = match &stats.memory_stats {
        Some(m) => m,
        None => return (0, 0, 0.0),
    };

    let memory_used = memory_stats.usage.unwrap_or(0) as i64;
    let memory_limit = memory_stats.limit.unwrap_or(0) as i64;

    let memory_percent = if memory_limit > 0 {
        (memory_used as f64 / memory_limit as f64) * 100.0
    } else {
        0.0
    };

    (memory_used, memory_limit, memory_percent)
}
