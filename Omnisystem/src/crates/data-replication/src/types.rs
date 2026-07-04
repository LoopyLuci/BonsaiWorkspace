use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Replica {
    pub replica_id: Uuid,
    pub shard_id: Uuid,
    pub node_id: String,
    pub role: ReplicaRole,
    pub created_at: DateTime<Utc>,
    pub is_healthy: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum ReplicaRole {
    Primary,
    Secondary,
    Tertiary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplicationLog {
    pub log_id: Uuid,
    pub shard_id: Uuid,
    pub operation: String,
    pub timestamp: DateTime<Utc>,
    pub replicated_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConflictRecord {
    pub conflict_id: Uuid,
    pub shard_id: Uuid,
    pub replica_1: Uuid,
    pub replica_2: Uuid,
    pub resolution_strategy: ResolutionStrategy,
    pub resolved: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum ResolutionStrategy {
    LastWriteWins,
    FirstWriteWins,
    MergeRequired,
    Abort,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplicationLag {
    pub lag_id: Uuid,
    pub replica_id: Uuid,
    pub lag_ms: u64,
    pub measured_at: DateTime<Utc>,
    pub threshold_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncState {
    pub state_id: Uuid,
    pub replica_id: Uuid,
    pub last_synced: DateTime<Utc>,
    pub pending_changes: u32,
    pub sync_status: SyncStatus,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum SyncStatus {
    InSync,
    Syncing,
    OutOfSync,
    Failed,
}
