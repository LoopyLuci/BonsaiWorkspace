use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplicationConfig {
    pub config_id: Uuid,
    pub primary_node: String,
    pub replica_nodes: Vec<String>,
    pub replication_mode: ReplicationMode,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Copy, Hash)]
pub enum ReplicationMode {
    Synchronous,
    Asynchronous,
    SemiSynchronous,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplicaStatus {
    pub replica_id: Uuid,
    pub node_name: String,
    pub lag_bytes: u64,
    pub is_synced: bool,
    pub last_sync: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FailoverEvent {
    pub event_id: Uuid,
    pub primary_node: String,
    pub new_primary: String,
    pub timestamp: DateTime<Utc>,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsistencyCheck {
    pub check_id: Uuid,
    pub primary_node: String,
    pub replicas_checked: usize,
    pub inconsistencies_found: usize,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplicationMetrics {
    pub metrics_id: Uuid,
    pub replication_lag_ms: u64,
    pub sync_success_rate: f32,
    pub failovers_total: u32,
    pub data_loss_events: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FailoverPolicy {
    pub policy_id: Uuid,
    pub name: String,
    pub auto_failover_enabled: bool,
    pub failover_timeout_seconds: u32,
}
