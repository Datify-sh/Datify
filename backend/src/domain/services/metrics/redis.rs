use std::collections::HashMap;
use std::sync::Arc;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use chrono::Utc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use crate::domain::models::{
    ClientMetrics, CommandMetrics, Database, KeyMetrics, KeyValueMetrics, MemoryMetrics,
    ReplicationMetrics, ResourceMetrics, UnifiedMetrics,
};
use crate::error::{AppError, AppResult};
use crate::infrastructure::docker::ContainerStats;

pub struct RedisMetricsCollector {
    encryption_key: Arc<[u8; 32]>,
    is_valkey: bool,
}

impl RedisMetricsCollector {
    pub fn new(encryption_key: [u8; 32], is_valkey: bool) -> Self {
        Self {
            encryption_key: Arc::new(encryption_key),
            is_valkey,
        }
    }

    fn decrypt_password(&self, encrypted: &str) -> AppResult<String> {
        let data = hex::decode(encrypted)
            .map_err(|e| AppError::Internal(format!("Invalid encrypted data: {}", e)))?;

        if data.len() < 12 {
            return Err(AppError::Internal("Encrypted data too short".to_string()));
        }

        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let cipher = Aes256Gcm::new_from_slice(&*self.encryption_key)
            .map_err(|e| AppError::Internal(format!("Decryption init failed: {}", e)))?;

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AppError::Internal(format!("Decryption failed: {}", e)))?;

        String::from_utf8(plaintext)
            .map_err(|e| AppError::Internal(format!("Invalid UTF-8 in password: {}", e)))
    }

    async fn connect(&self, database: &Database) -> AppResult<TcpStream> {
        let container_name = database.container_name();
        let addr = format!("{}:6379", container_name);

        let stream = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            TcpStream::connect(&addr),
        )
        .await
        .map_err(|_| AppError::Internal("Connection timeout".to_string()))?
        .map_err(|e| AppError::Internal(format!("Failed to connect to Redis: {}", e)))?;

        Ok(stream)
    }

    async fn send_command(stream: &mut TcpStream, args: &[&str]) -> AppResult<String> {
        let mut cmd = format!("*{}\r\n", args.len());
        for arg in args {
            cmd.push_str(&format!("${}\r\n{}\r\n", arg.len(), arg));
        }

        stream
            .write_all(cmd.as_bytes())
            .await
            .map_err(|e| AppError::Internal(format!("Failed to send command: {}", e)))?;

        Self::read_response(stream).await
    }

    async fn read_response(stream: &mut TcpStream) -> AppResult<String> {
        let mut reader = BufReader::new(stream);
        let mut line = String::new();

        reader
            .read_line(&mut line)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to read response: {}", e)))?;

        let line = line.trim();

        match line.chars().next() {
            Some('+') => Ok(line[1..].to_string()),
            Some('-') => Err(AppError::Internal(format!("Redis error: {}", &line[1..]))),
            Some('$') => {
                let len: i64 = line[1..].parse().unwrap_or(-1);
                if len < 0 {
                    return Ok(String::new());
                }
                let mut buf = vec![0u8; len as usize + 2];
                tokio::io::AsyncReadExt::read_exact(&mut reader, &mut buf)
                    .await
                    .map_err(|e| AppError::Internal(format!("Failed to read bulk: {}", e)))?;
                Ok(String::from_utf8_lossy(&buf[..len as usize]).to_string())
            },
            _ => Ok(line.to_string()),
        }
    }

    fn parse_info(info: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for line in info.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once(':') {
                map.insert(key.to_string(), value.to_string());
            }
        }
        map
    }

    fn parse_keyspace(info: &HashMap<String, String>) -> (i64, i64) {
        let mut total_keys = 0i64;
        let mut keys_with_expiry = 0i64;

        for (key, value) in info.iter() {
            if key.starts_with("db") {
                for part in value.split(',') {
                    if let Some(kv) = part.strip_prefix("keys=") {
                        total_keys += kv.parse::<i64>().unwrap_or(0);
                    } else if let Some(kv) = part.strip_prefix("expires=") {
                        keys_with_expiry += kv.parse::<i64>().unwrap_or(0);
                    }
                }
            }
        }

        (total_keys, keys_with_expiry)
    }

    async fn collect_kv_metrics(&self, database: &Database) -> AppResult<KvMetrics> {
        let mut stream = self.connect(database).await?;

        if let Some(encrypted) = &database.password_encrypted {
            let password = self.decrypt_password(encrypted)?;
            let auth_result = Self::send_command(&mut stream, &["AUTH", &password]).await;
            if let Err(e) = auth_result {
                tracing::warn!("Redis AUTH failed: {}", e);
            }
        }

        let info = Self::send_command(&mut stream, &["INFO"]).await?;
        let parsed = Self::parse_info(&info);

        let (total_keys, keys_with_expiry) = Self::parse_keyspace(&parsed);

        let expired_keys = parsed
            .get("expired_keys")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let evicted_keys = parsed
            .get("evicted_keys")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let total_commands = parsed
            .get("total_commands_processed")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let ops_per_sec = parsed
            .get("instantaneous_ops_per_sec")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0);

        let keyspace_hits: i64 = parsed
            .get("keyspace_hits")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let keyspace_misses: i64 = parsed
            .get("keyspace_misses")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let hit_rate = if keyspace_hits + keyspace_misses > 0 {
            (keyspace_hits as f64 / (keyspace_hits + keyspace_misses) as f64) * 100.0
        } else {
            0.0
        };

        let used_memory = parsed
            .get("used_memory")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let used_memory_rss = parsed
            .get("used_memory_rss")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let used_memory_peak = parsed
            .get("used_memory_peak")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let max_memory = parsed
            .get("maxmemory")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let memory_fragmentation_ratio = parsed
            .get("mem_fragmentation_ratio")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1.0);

        let connected_clients = parsed
            .get("connected_clients")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let blocked_clients = parsed
            .get("blocked_clients")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let max_clients = parsed
            .get("maxclients")
            .and_then(|v| v.parse().ok())
            .unwrap_or(10000);

        let role = parsed
            .get("role")
            .cloned()
            .unwrap_or_else(|| "master".to_string());
        let connected_slaves = parsed
            .get("connected_slaves")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        Ok(KvMetrics {
            keys: KeyMetrics {
                total_keys,
                keys_with_expiry,
                expired_keys,
                evicted_keys,
            },
            commands: CommandMetrics {
                total_commands,
                ops_per_sec,
                keyspace_hits,
                keyspace_misses,
                hit_rate,
            },
            memory: MemoryMetrics {
                used_memory,
                used_memory_rss,
                used_memory_peak,
                max_memory,
                memory_fragmentation_ratio,
            },
            clients: ClientMetrics {
                connected_clients,
                blocked_clients,
                max_clients,
            },
            replication: ReplicationMetrics {
                role,
                connected_slaves,
            },
        })
    }

    pub async fn collect_metrics(
        &self,
        database: &Database,
        docker_stats: &ContainerStats,
    ) -> AppResult<UnifiedMetrics> {
        let kv_metrics = match self.collect_kv_metrics(database).await {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("Failed to collect Redis metrics: {}", e);
                KvMetrics::default()
            },
        };

        let timestamp = Utc::now().to_rfc3339();

        let metrics = KeyValueMetrics {
            timestamp,
            keys: kv_metrics.keys,
            commands: kv_metrics.commands,
            memory: kv_metrics.memory,
            clients: kv_metrics.clients,
            replication: kv_metrics.replication,
            resources: ResourceMetrics {
                cpu_percent: docker_stats.cpu_percent,
                memory_used_bytes: docker_stats.memory_used_bytes,
                memory_limit_bytes: docker_stats.memory_limit_bytes,
                memory_percent: docker_stats.memory_percent,
            },
        };

        if self.is_valkey {
            Ok(UnifiedMetrics::Valkey(metrics))
        } else {
            Ok(UnifiedMetrics::Redis(metrics))
        }
    }
}

#[derive(Default)]
struct KvMetrics {
    keys: KeyMetrics,
    commands: CommandMetrics,
    memory: MemoryMetrics,
    clients: ClientMetrics,
    replication: ReplicationMetrics,
}
