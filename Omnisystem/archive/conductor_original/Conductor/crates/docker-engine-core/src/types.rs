//! Docker Engine data types
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Container status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerStatus {
    /// Created but not started
    Created,
    /// Currently running
    Running,
    /// Paused
    Paused,
    /// Exited/stopped
    Exited,
    /// Restarting
    Restarting,
    /// Removing
    Removing,
}

/// Container representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    /// Container ID
    pub id: String,
    /// Container name
    pub name: String,
    /// Image reference
    pub image: String,
    /// Current status
    pub status: ContainerStatus,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Port mappings
    pub ports: Vec<PortMapping>,
    /// Volume mounts
    pub volumes: Vec<VolumeMount>,
}

/// Port mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Container port
    pub container_port: u16,
    /// Host port (optional)
    pub host_port: Option<u16>,
    /// Protocol (tcp/udp)
    pub protocol: String,
}

/// Volume mount configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    /// Volume name or path
    pub source: String,
    /// Mount path in container
    pub destination: String,
    /// Read-only flag
    pub read_only: bool,
}

/// Configuration for creating a container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Container name
    pub name: String,
    /// Image reference
    pub image: String,
    /// Port mappings
    pub ports: Option<Vec<PortMapping>>,
    /// Volume mounts
    pub volumes: Option<Vec<VolumeMount>>,
    /// Environment variables
    pub environment: Option<HashMap<String, String>>,
}

/// Container statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStats {
    /// Container ID
    pub container_id: String,
    /// CPU percentage
    pub cpu_percent: f64,
    /// Memory in MB
    pub memory_mb: u64,
    /// Network input bytes
    pub network_in: u64,
    /// Network output bytes
    pub network_out: u64,
}

/// Docker image representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    /// Image ID
    pub id: String,
    /// Repository tags
    pub repo_tags: Vec<String>,
    /// Size in bytes
    pub size: u64,
    /// Creation timestamp
    pub created: DateTime<Utc>,
}

/// Docker network representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Network {
    /// Network ID
    pub id: String,
    /// Network name
    pub name: String,
    /// Driver type
    pub driver: String,
    /// IPAM configuration
    pub ipam: IpamConfig,
}

/// IPAM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpamConfig {
    /// Subnet
    pub subnet: Option<String>,
    /// Gateway
    pub gateway: Option<String>,
}

/// Configuration for creating a network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Network name
    pub name: String,
    /// Driver type (bridge, overlay, etc)
    pub driver: Option<String>,
    /// IPAM config
    pub ipam: Option<IpamConfig>,
}

/// Docker volume representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volume {
    /// Volume name
    pub name: String,
    /// Driver type
    pub driver: String,
    /// Mount point
    pub mountpoint: String,
}

/// Configuration for creating a volume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeConfig {
    /// Volume name
    pub name: String,
    /// Driver type
    pub driver: Option<String>,
}

/// Image build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageBuildConfig {
    /// Dockerfile path
    pub dockerfile_path: String,
    /// Build context path
    pub context_path: String,
    /// Image tags
    pub tags: Vec<String>,
    /// Build arguments
    pub build_args: Option<HashMap<String, String>>,
}

/// Command execution output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecOutput {
    /// Exit code
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
}
