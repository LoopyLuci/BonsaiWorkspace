use crate::{Replica, ReplicaRole, ReplicationLog, ConflictRecord, ResolutionStrategy, ReplicationLag, SyncState, SyncStatus, ReplicationError, ReplicationResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct ReplicationManager {
    replicas: Arc<DashMap<Uuid, Replica>>,
    logs: Arc<DashMap<Uuid, ReplicationLog>>,
    conflicts: Arc<DashMap<Uuid, ConflictRecord>>,
    lags: Arc<DashMap<Uuid, ReplicationLag>>,
    sync_states: Arc<DashMap<Uuid, SyncState>>,
}

impl ReplicationManager {
    pub fn new() -> Self {
        Self {
            replicas: Arc::new(DashMap::new()),
            logs: Arc::new(DashMap::new()),
            conflicts: Arc::new(DashMap::new()),
            lags: Arc::new(DashMap::new()),
            sync_states: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_replica(&self, shard_id: Uuid, node_id: &str, role: ReplicaRole) -> ReplicationResult<Replica> {
        let replica = Replica {
            replica_id: Uuid::new_v4(),
            shard_id,
            node_id: node_id.to_string(),
            role,
            created_at: Utc::now(),
            is_healthy: true,
        };

        self.replicas.insert(replica.replica_id, replica.clone());
        Ok(replica)
    }

    pub async fn log_operation(&self, shard_id: Uuid, operation: &str) -> ReplicationResult<ReplicationLog> {
        let log = ReplicationLog {
            log_id: Uuid::new_v4(),
            shard_id,
            operation: operation.to_string(),
            timestamp: Utc::now(),
            replicated_at: None,
        };

        self.logs.insert(log.log_id, log.clone());
        Ok(log)
    }

    pub async fn mark_replicated(&self, log_id: Uuid) -> ReplicationResult<()> {
        if let Some(mut entry) = self.logs.get_mut(&log_id) {
            entry.replicated_at = Some(Utc::now());
        } else {
            return Err(ReplicationError::ReplicationFailed);
        }

        Ok(())
    }

    pub async fn detect_conflict(&self, shard_id: Uuid, replica_1: Uuid, replica_2: Uuid) -> ReplicationResult<ConflictRecord> {
        let conflict = ConflictRecord {
            conflict_id: Uuid::new_v4(),
            shard_id,
            replica_1,
            replica_2,
            resolution_strategy: ResolutionStrategy::LastWriteWins,
            resolved: false,
        };

        self.conflicts.insert(conflict.conflict_id, conflict.clone());
        Ok(conflict)
    }

    pub async fn resolve_conflict(&self, conflict_id: Uuid) -> ReplicationResult<()> {
        if let Some(mut entry) = self.conflicts.get_mut(&conflict_id) {
            entry.resolved = true;
        } else {
            return Err(ReplicationError::ConflictResolutionFailed);
        }

        Ok(())
    }

    pub async fn measure_lag(&self, replica_id: Uuid, lag_ms: u64) -> ReplicationResult<ReplicationLag> {
        let lag = ReplicationLag {
            lag_id: Uuid::new_v4(),
            replica_id,
            lag_ms,
            measured_at: Utc::now(),
            threshold_ms: 1000,
        };

        if lag_ms > lag.threshold_ms {
            return Err(ReplicationError::LagExceeded);
        }

        self.lags.insert(lag.lag_id, lag.clone());
        Ok(lag)
    }

    pub async fn update_sync_state(&self, replica_id: Uuid, status: SyncStatus) -> ReplicationResult<SyncState> {
        let state = SyncState {
            state_id: Uuid::new_v4(),
            replica_id,
            last_synced: Utc::now(),
            pending_changes: 0,
            sync_status: status,
        };

        self.sync_states.insert(state.state_id, state.clone());
        Ok(state)
    }

    pub fn replica_count(&self) -> usize {
        self.replicas.len()
    }
}

impl Default for ReplicationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_replica() {
        let manager = ReplicationManager::new();
        let replica = manager
            .register_replica(Uuid::new_v4(), "node-1", ReplicaRole::Primary)
            .await
            .unwrap();

        assert_eq!(replica.role, ReplicaRole::Primary);
        assert!(replica.is_healthy);
        assert_eq!(manager.replica_count(), 1);
    }

    #[tokio::test]
    async fn test_log_operation() {
        let manager = ReplicationManager::new();
        let shard_id = Uuid::new_v4();
        let log = manager.log_operation(shard_id, "INSERT").await.unwrap();

        assert_eq!(log.operation, "INSERT");
        assert!(log.replicated_at.is_none());
    }

    #[tokio::test]
    async fn test_detect_conflict() {
        let manager = ReplicationManager::new();
        let conflict = manager
            .detect_conflict(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4())
            .await
            .unwrap();

        assert!(!conflict.resolved);
        assert_eq!(conflict.resolution_strategy, ResolutionStrategy::LastWriteWins);
    }

    #[tokio::test]
    async fn test_update_sync_state() {
        let manager = ReplicationManager::new();
        let state = manager.update_sync_state(Uuid::new_v4(), SyncStatus::InSync).await.unwrap();

        assert_eq!(state.sync_status, SyncStatus::InSync);
    }
}
