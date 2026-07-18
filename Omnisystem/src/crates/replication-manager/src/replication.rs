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
    failover_policies: Arc<DashMap<Uuid, FailoverPolicy>>,
}

impl ReplicationManager {
    pub fn new() -> Self {
        Self {
            configs: Arc::new(DashMap::new()),
            replicas: Arc::new(DashMap::new()),
            failover_events: Arc::new(DashMap::new()),
            consistency_checks: Arc::new(DashMap::new()),
            failover_policies: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_failover_policy(&self, policy: &FailoverPolicy) -> ReplicationResult<()> {
        self.failover_policies.insert(policy.policy_id, policy.clone());
        Ok(())
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

    /// Compute real replication metrics from currently tracked state,
    /// rather than leaving the ReplicationMetrics type declared but never
    /// constructed anywhere.
    pub fn get_metrics(&self) -> ReplicationMetrics {
        let replicas: Vec<ReplicaStatus> = self.replicas.iter().map(|e| e.value().clone()).collect();

        let replication_lag_ms = if replicas.is_empty() {
            0
        } else {
            // Treat lag_bytes as a proxy for lag time in this in-memory
            // model (1 byte ~= 1ms of replay lag), averaged across replicas.
            replicas.iter().map(|r| r.lag_bytes).sum::<u64>() / replicas.len() as u64
        };

        let sync_success_rate = if replicas.is_empty() {
            1.0
        } else {
            let synced = replicas.iter().filter(|r| r.is_synced).count();
            synced as f32 / replicas.len() as f32
        };

        let failovers_total = self.failover_events.len() as u32;

        // A failover event whose reason mentions data loss is counted as
        // a real data-loss event, rather than a made-up constant.
        let data_loss_events = self
            .failover_events
            .iter()
            .filter(|e| e.value().reason.to_lowercase().contains("data loss"))
            .count() as u32;

        ReplicationMetrics {
            metrics_id: Uuid::new_v4(),
            replication_lag_ms,
            sync_success_rate,
            failovers_total,
            data_loss_events,
        }
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

    #[tokio::test]
    async fn test_check_consistency_counts_real_inconsistencies() {
        let manager = ReplicationManager::new();
        manager
            .register_replica(&ReplicaStatus {
                replica_id: Uuid::new_v4(),
                node_name: "synced".to_string(),
                lag_bytes: 0,
                is_synced: true,
                last_sync: Utc::now(),
            })
            .await
            .unwrap();
        manager
            .register_replica(&ReplicaStatus {
                replica_id: Uuid::new_v4(),
                node_name: "lagging".to_string(),
                lag_bytes: 500,
                is_synced: false,
                last_sync: Utc::now(),
            })
            .await
            .unwrap();

        let check_id = manager.check_consistency("primary1").await.unwrap();
        let check = manager.consistency_checks.get(&check_id).unwrap();
        assert_eq!(check.replicas_checked, 2);
        assert_eq!(check.inconsistencies_found, 1);
    }

    #[tokio::test]
    async fn test_get_metrics_reflects_real_replica_state() {
        let manager = ReplicationManager::new();
        let r1 = Uuid::new_v4();
        let r2 = Uuid::new_v4();
        manager
            .register_replica(&ReplicaStatus { replica_id: r1, node_name: "a".to_string(), lag_bytes: 0, is_synced: true, last_sync: Utc::now() })
            .await
            .unwrap();
        manager
            .register_replica(&ReplicaStatus { replica_id: r2, node_name: "b".to_string(), lag_bytes: 200, is_synced: false, last_sync: Utc::now() })
            .await
            .unwrap();

        manager.trigger_failover("primary1", "b", "network partition caused data loss").await.unwrap();

        let metrics = manager.get_metrics();
        assert_eq!(metrics.replication_lag_ms, 100); // average of 0 and 200
        assert_eq!(metrics.sync_success_rate, 0.5);
        assert_eq!(metrics.failovers_total, 1);
        assert_eq!(metrics.data_loss_events, 1);
    }

    #[tokio::test]
    async fn test_register_failover_policy() {
        let manager = ReplicationManager::new();
        let policy = FailoverPolicy {
            policy_id: Uuid::new_v4(),
            name: "aggressive".to_string(),
            auto_failover_enabled: true,
            failover_timeout_seconds: 30,
        };

        manager.register_failover_policy(&policy).await.unwrap();
        assert_eq!(manager.failover_policies.len(), 1);
    }

    #[tokio::test]
    async fn test_update_replica_lag_unknown_replica_fails() {
        let manager = ReplicationManager::new();
        let result = manager.update_replica_lag(Uuid::new_v4(), 100).await;
        assert!(matches!(result, Err(ReplicationError::ReplicaNotFound)));
    }
}
