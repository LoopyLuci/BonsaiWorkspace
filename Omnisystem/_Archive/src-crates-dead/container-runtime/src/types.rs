use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct ImageId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct ContainerId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct RegistryUrl(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum ImageFormat {
    Docker,
    Oci,
}

impl ImageFormat {
    pub fn to_string(&self) -> &'static str {
        match self {
            ImageFormat::Docker => "docker",
            ImageFormat::Oci => "oci",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ContainerState {
    Created,
    Running,
    Paused,
    Stopped,
    Exited,
    Dead,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerImage {
    pub id: ImageId,
    pub name: String,
    pub tag: String,
    pub digest: String,
    pub format: ImageFormat,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
    pub layers: Vec<ImageLayer>,
    pub config: ImageConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageLayer {
    pub digest: String,
    pub size_bytes: u64,
    pub media_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageConfig {
    pub entrypoint: Vec<String>,
    pub cmd: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: String,
    pub user: String,
    pub exposed_ports: Vec<u16>,
    pub volumes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Container {
    pub id: ContainerId,
    pub image_id: ImageId,
    pub name: String,
    pub state: ContainerState,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub stopped_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub config: ContainerConfig,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerConfig {
    pub image: String,
    pub cmd: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: String,
    pub ports: Vec<PortBinding>,
    pub volumes: Vec<VolumeMount>,
    pub cpu_limit_millicores: u64,
    pub memory_limit_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortBinding {
    pub container_port: u16,
    pub host_port: u16,
    pub protocol: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VolumeMount {
    pub source: String,
    pub destination: String,
    pub read_only: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub url: RegistryUrl,
    pub username: Option<String>,
    pub password: Option<String>,
    pub insecure: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImagePullOptions {
    pub registry: RegistryUrl,
    pub username: Option<String>,
    pub password: Option<String>,
    pub timeout_secs: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImagePushOptions {
    pub registry: RegistryUrl,
    pub username: Option<String>,
    pub password: Option<String>,
    pub timeout_secs: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerStats {
    pub container_id: ContainerId,
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub memory_limit_bytes: u64,
    pub network_in_bytes: u64,
    pub network_out_bytes: u64,
    pub block_read_bytes: u64,
    pub block_write_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageManifest {
    pub schema_version: u32,
    pub media_type: String,
    pub config_digest: String,
    pub layers: Vec<ImageLayer>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub data_root: String,
    pub log_driver: String,
    pub storage_driver: String,
    pub max_concurrent_pulls: usize,
    pub max_concurrent_pushes: usize,
    pub default_registry: RegistryUrl,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            data_root: "/var/lib/containers".to_string(),
            log_driver: "json-file".to_string(),
            storage_driver: "overlay2".to_string(),
            max_concurrent_pulls: 5,
            max_concurrent_pushes: 3,
            default_registry: RegistryUrl("docker.io".to_string()),
        }
    }
}
