use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct PodId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct DeploymentId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct ServiceId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct NodeId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum PodPhase {
    Pending,
    Running,
    Succeeded,
    Failed,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum DeploymentStrategy {
    RollingUpdate,
    Recreate,
    BlueGreen,
    Canary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pod {
    pub id: PodId,
    pub name: String,
    pub deployment_id: Option<DeploymentId>,
    pub phase: PodPhase,
    pub containers: Vec<String>,
    pub node_id: Option<NodeId>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub ready: bool,
    pub restart_count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PodSpec {
    pub name: String,
    pub image: String,
    pub replicas: u32,
    pub labels: HashMap<String, String>,
    pub cpu_request_millicores: u64,
    pub memory_request_bytes: u64,
    pub cpu_limit_millicores: u64,
    pub memory_limit_bytes: u64,
    pub ports: Vec<u16>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Deployment {
    pub id: DeploymentId,
    pub name: String,
    pub spec: PodSpec,
    pub strategy: DeploymentStrategy,
    pub desired_replicas: u32,
    pub ready_replicas: u32,
    pub updated_replicas: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: DeploymentStatus,
    pub revision: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DeploymentStatus {
    Pending,
    Progressing,
    Available,
    Failed,
    Paused,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Service {
    pub id: ServiceId,
    pub name: String,
    pub selector: HashMap<String, String>,
    pub ports: Vec<ServicePort>,
    pub cluster_ip: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServicePort {
    pub name: String,
    pub port: u16,
    pub target_port: u16,
    pub protocol: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub name: String,
    pub capacity_cpu_millicores: u64,
    pub capacity_memory_bytes: u64,
    pub available_cpu_millicores: u64,
    pub available_memory_bytes: u64,
    pub status: NodeStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum NodeStatus {
    Ready,
    NotReady,
    Unknown,
    Cordoned,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScalingPolicy {
    pub target_cpu_percent: f64,
    pub target_memory_percent: f64,
    pub min_replicas: u32,
    pub max_replicas: u32,
    pub scale_up_threshold: f64,
    pub scale_down_threshold: f64,
    pub scale_up_cooldown_secs: u64,
    pub scale_down_cooldown_secs: u64,
}

impl Default for ScalingPolicy {
    fn default() -> Self {
        Self {
            target_cpu_percent: 70.0,
            target_memory_percent: 80.0,
            min_replicas: 1,
            max_replicas: 10,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.3,
            scale_up_cooldown_secs: 60,
            scale_down_cooldown_secs: 300,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthProbe {
    pub probe_type: ProbeType,
    pub initial_delay_secs: u32,
    pub timeout_secs: u32,
    pub period_secs: u32,
    pub success_threshold: u32,
    pub failure_threshold: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ProbeType {
    Http(String),
    Tcp(u16),
    Exec(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RolloutHistory {
    pub revision: u32,
    pub image: String,
    pub timestamp: DateTime<Utc>,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeploymentStatus2 {
    pub observed_generation: u32,
    pub replicas: u32,
    pub updated_replicas: u32,
    pub ready_replicas: u32,
    pub available_replicas: u32,
    pub unavailable_replicas: u32,
}
