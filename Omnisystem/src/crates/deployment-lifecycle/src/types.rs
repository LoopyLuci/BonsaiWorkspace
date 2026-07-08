use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct RolloutId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct ClusterId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum RolloutStrategy {
    RollingUpdate,
    Recreate,
    BlueGreen,
    Canary,
}

impl RolloutStrategy {
    pub fn to_string(&self) -> &'static str {
        match self {
            RolloutStrategy::RollingUpdate => "rolling-update",
            RolloutStrategy::Recreate => "recreate",
            RolloutStrategy::BlueGreen => "blue-green",
            RolloutStrategy::Canary => "canary",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RolloutStatus {
    Pending,
    InProgress,
    Paused,
    Completed,
    Failed,
    RolledBack,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rollout {
    pub id: RolloutId,
    pub deployment_id: String,
    pub strategy: RolloutStrategy,
    pub old_revision: u32,
    pub new_revision: u32,
    pub status: RolloutStatus,
    pub progress_percent: u8,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub surge_replicas: u32,
    pub unavailable_replicas: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RolloutConfig {
    pub max_surge: u32,
    pub max_unavailable: u32,
    pub min_ready_seconds: u32,
    pub progress_deadline_seconds: u32,
}

impl Default for RolloutConfig {
    fn default() -> Self {
        Self {
            max_surge: 1,
            max_unavailable: 1,
            min_ready_seconds: 0,
            progress_deadline_seconds: 600,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RevisionHistory {
    pub revision: u32,
    pub image: String,
    pub timestamp: DateTime<Utc>,
    pub rollout_status: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ClusterStatus {
    Ready,
    NotReady,
    Connecting,
    Disconnected,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cluster {
    pub id: ClusterId,
    pub name: String,
    pub api_url: String,
    pub status: ClusterStatus,
    pub capacity_replicas: u32,
    pub available_replicas: u32,
    pub region: String,
    pub created_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClusterFederationConfig {
    pub primary_cluster: ClusterId,
    pub sync_interval_secs: u64,
    pub health_check_interval_secs: u64,
    pub failover_enabled: bool,
    pub replication_factor: u32,
}

impl Default for ClusterFederationConfig {
    fn default() -> Self {
        Self {
            primary_cluster: ClusterId("primary".to_string()),
            sync_interval_secs: 30,
            health_check_interval_secs: 10,
            failover_enabled: true,
            replication_factor: 2,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CanaryConfig {
    pub initial_traffic_percent: u8,
    pub increment_percent: u8,
    pub increment_interval_secs: u64,
    pub error_rate_threshold: f64,
}

impl Default for CanaryConfig {
    fn default() -> Self {
        Self {
            initial_traffic_percent: 10,
            increment_percent: 10,
            increment_interval_secs: 60,
            error_rate_threshold: 5.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlueGreenConfig {
    pub blue_version: String,
    pub green_version: String,
    pub active_color: String,
    pub switch_traffic_timeout_secs: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeploymentEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub message: String,
    pub severity: String,
}
