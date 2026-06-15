use crate::{ReplicationConfig, ReplicaStatus, FailoverEvent, ConsistencyCheck, ReplicationMetrics, FailoverPolicy, ReplicationError, ReplicationResult, ReplicationMode};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct ReplicationManager {
    configs: Arc<DashMap<Uuid, ReplicationConfig>>,
    replicas: Arc<DashMap<Uuid, ReplicaStatus>>,
    failover_events: Arc<DashMap<Uuid, FailoverEvent>>,
    consistency_checks: Arc<DashMap<Uuid, ConsistencyCheck>>,
}

impl ReplicationManager {
    pub fn new() -> Self {
        Self {
            configs: Arc::new(DashMap::new()),
            replicas: Arc::new(DashMap::new()),
            failover_events: Arc::new(DashMap::new()),
            consistency_checks: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_replication_config(&self, config: &ReplicationConfig) -> ReplicationResult<()> {
        self.configs.insert(config.config_id, config.clone());
        Ok(())
    }

    pub async fn register_replica(&self, replica: &ReplicaStatus) -> ReplicationResult<()> {
        self.replicas.insert(replica.replica_id, replica.clone());
        Ok(())
    }

    pub async fn update_replica_lag(&self, replica_id: Uuid, lag_bytes: u64) -> ReplicationResult<()> {
        if let Some(mut replica) = self.replicas.get_mut(&replica_id) {
            replica.lag_bytes = lag_bytes;
            replica.is_synced = lag_bytes == 0;
            Ok(())
        } else {
            Err(ReplicationError::ReplicaNotFound)
        }
    }

    pub async fn check_consistency(&self, primary_node: &str) -> ReplicationResult<Uuid> {
        let mut inconsistencies = 0;
        let mut replicas_checked = 0;

        for entry in self.replicas.iter() {
            let replica = entry.value();
            replicas_checked += 1;
            if !replica.is_synced {
                inconsistencies += 1;
            }
        }

        let check = ConsistencyCheck {
            check_id: Uuid::new_v4(),
            primary_node: primary_node.to_string(),
            replicas_checked,
            inconsistencies_found: inconsistencies,
            timestamp: Utc::now(),
        };

        let check_id = check.check_id;
        self.consistency_checks.insert(check_id, check);
        Ok(check_id)
    }

    pub async fn trigger_failover(&self, primary_node: &str, new_primary: &str, reason: &str) -> ReplicationResult<Uuid> {
        let event = FailoverEvent {
            event_id: Uuid::new_v4(),
            primary_node: primary_node.to_string(),
            new_primary: new_primary.to_string(),
            timestamp: Utc::now(),
            reason: reason.to_string(),
        };

        let event_id = event.event_id;
        self.failover_events.insert(event_id, event);
        Ok(event_id)
    }

    pub async fn get_replica_status(&self, replica_id: Uuid) -> ReplicationResult<ReplicaStatus> {
        self.replicas
            .get(&replica_id)
            .map(|r| r.clone())
            .ok_or(ReplicationError::ReplicaNotFound)
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
    async fn test_create_replication_config() {
        let manager = ReplicationManager::new();
        let config = ReplicationConfig {
            config_id: Uuid::new_v4(),
            primary_node: "primary1".to_string(),
            replica_nodes: vec!["replica1".to_string(), "replica2".to_string()],
            replication_mode: ReplicationMode::Synchronous,
            enabled: true,
        };

        manager.create_replication_config(&config).await.unwrap();
    }

    #[tokio::test]
    async fn test_register_replica() {
        let manager = ReplicationManager::new();
        let replica = ReplicaStatus {
            replica_id: Uuid::new_v4(),
            node_name: "replica1".to_string(),
            lag_bytes: 0,
            is_synced: true,
            last_sync: Utc::now(),
        };

        manager.register_replica(&replica).await.unwrap();
        assert_eq!(manager.replica_count(), 1);
    }

    #[tokio::test]
    async fn test_update_replica_lag() {
        let manager = ReplicationManager::new();
        let replica_id = Uuid::new_v4();
        let replica = ReplicaStatus {
            replica_id,
            node_name: "replica2".to_string(),
            lag_bytes: 0,
            is_synced: true,
            last_sync: Utc::now(),
        };

        manager.register_replica(&replica).await.unwrap();
        manager.update_replica_lag(replica_id, 1000).await.unwrap();

        let updated = manager.get_replica_status(replica_id).await.unwrap();
        assert_eq!(updated.lag_bytes, 1000);
    }

    #[tokio::test]
    async fn test_trigger_failover() {
        let manager = ReplicationManager::new();
        let event_id = manager.trigger_failover("primary1", "replica1", "Primary crashed").await.unwrap();
        assert!(!event_id.is_nil());
    }
}
