use crate::{
    DatabaseError, DatabaseId, DatabaseResult, ReplicationConfig, ReplicationManager,
    ReplicationStatus,
};
use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct ReplicationManagerImpl {
    configs: Arc<DashMap<String, ReplicationConfig>>,
    status: Arc<DashMap<String, ReplicationStatus>>,
    replication_lags: Arc<DashMap<String, AtomicU64>>,
}

impl ReplicationManagerImpl {
    pub fn new() -> Self {
        Self {
            configs: Arc::new(DashMap::new()),
            status: Arc::new(DashMap::new()),
            replication_lags: Arc::new(DashMap::new()),
        }
    }

    pub fn replica_count(&self) -> usize {
        self.configs
            .iter()
            .filter(|entry| entry.enabled)
            .count()
    }
}

impl Default for ReplicationManagerImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ReplicationManager for ReplicationManagerImpl {
    async fn configure_replication(
        &self,
        database_id: &DatabaseId,
        config: ReplicationConfig,
    ) -> DatabaseResult<()> {
        if !config.enabled && config.replication_factor > 1 {
            return Err(DatabaseError::InvalidConfiguration(
                "Cannot have factor > 1 with disabled replication".to_string(),
            ));
        }

        self.configs.insert(database_id.0.to_string(), config.clone());

        // Initialize status
        let status = ReplicationStatus {
            database_id: database_id.clone(),
            is_primary: true,
            replication_lag_secs: 0,
            last_sync: Utc::now(),
            healthy_replicas: 0,
            total_replicas: config.replicas.len() as u32,
        };

        self.status.insert(database_id.0.to_string(), status);
        self.replication_lags
            .insert(database_id.0.to_string(), AtomicU64::new(0));

        Ok(())
    }

    async fn get_replication_status(
        &self,
        database_id: &DatabaseId,
    ) -> DatabaseResult<ReplicationStatus> {
        self.status
            .get(&database_id.0.to_string())
            .map(|entry| entry.clone())
            .ok_or_else(|| {
                DatabaseError::DatabaseNotFound(format!(
                    "Replication status not found: {}",
                    database_id.0
                ))
            })
    }

    async fn promote_replica(
        &self,
        replica_id: &str,
    ) -> DatabaseResult<DatabaseId> {
        // Find which database has this replica
        for entry in self.configs.iter() {
            let config = entry.value();
            if config.replicas.iter().any(|r| r == replica_id) {
                let db_id = DatabaseId(
                    uuid::Uuid::parse_str(entry.key())
                        .map_err(|_| DatabaseError::InvalidConfiguration("Invalid UUID".to_string()))?,
                );

                // Promote replica to primary
                if let Some(mut status) = self.status.get_mut(&db_id.0.to_string()) {
                    status.is_primary = true;
                    status.healthy_replicas = status.total_replicas - 1;
                }

                let new_replica_id = DatabaseId(uuid::Uuid::new_v4());
                return Ok(new_replica_id);
            }
        }

        Err(DatabaseError::ReplicationFailed(format!(
            "Replica not found: {}",
            replica_id
        )))
    }

    async fn add_replica(
        &self,
        primary_id: &DatabaseId,
        _replica_host: String,
    ) -> DatabaseResult<String> {
        let mut config = self
            .configs
            .get_mut(&primary_id.0.to_string())
            .ok_or_else(|| {
                DatabaseError::DatabaseNotFound(format!(
                    "Primary database not found: {}",
                    primary_id.0
                ))
            })?;

        let replica_id = uuid::Uuid::new_v4().to_string();
        config.replicas.push(replica_id.clone());

        if let Some(mut status) = self.status.get_mut(&primary_id.0.to_string()) {
            status.total_replicas = config.replicas.len() as u32;
            status.healthy_replicas = (config.replicas.len() as u32).saturating_sub(1);
        }

        Ok(replica_id)
    }

    async fn remove_replica(&self, replica_id: &str) -> DatabaseResult<()> {
        for mut entry in self.configs.iter_mut() {
            let db_id = entry.key().clone();
            let config = entry.value_mut();
            config.replicas.retain(|r| r != replica_id);
            let replica_count = config.replicas.len() as u32;
            drop(config);

            if let Some(mut status) = self.status.get_mut(&db_id) {
                status.total_replicas = replica_count;
            }
        }

        Ok(())
    }

    async fn check_replication_lag(
        &self,
        database_id: &DatabaseId,
    ) -> DatabaseResult<u64> {
        self.replication_lags
            .get(&database_id.0.to_string())
            .map(|entry| entry.load(Ordering::Relaxed))
            .ok_or_else(|| {
                DatabaseError::DatabaseNotFound(format!(
                    "Replication lag not found: {}",
                    database_id.0
                ))
            })
    }

    async fn trigger_failover(&self, database_id: &DatabaseId) -> DatabaseResult<()> {
        let config = self
            .configs
            .get(&database_id.0.to_string())
            .ok_or_else(|| {
                DatabaseError::DatabaseNotFound(format!(
                    "Database not found: {}",
                    database_id.0
                ))
            })?;

        if !config.failover_enabled {
            return Err(DatabaseError::ReplicationFailed(
                "Failover not enabled".to_string(),
            ));
        }

        if config.replicas.is_empty() {
            return Err(DatabaseError::ReplicationFailed(
                "No replicas available".to_string(),
            ));
        }

        // Promote first healthy replica (would be used in production for actual failover)
        let _new_primary_replica = config.replicas[0].clone();

        if let Some(mut status) = self.status.get_mut(&database_id.0.to_string()) {
            status.is_primary = false;
            status.last_sync = Utc::now();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_configure_replication() {
        let manager = ReplicationManagerImpl::new();
        let db_id = DatabaseId(Uuid::new_v4());

        let config = ReplicationConfig {
            enabled: true,
            replication_factor: 3,
            replication_lag_tolerance_secs: 10,
            failover_enabled: true,
            replicas: vec!["replica1".to_string(), "replica2".to_string()],
        };

        let result = manager
            .configure_replication(&db_id, config)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_replication_config() {
        let manager = ReplicationManagerImpl::new();
        let db_id = DatabaseId(Uuid::new_v4());

        let config = ReplicationConfig {
            enabled: false,
            replication_factor: 3, // Invalid: factor > 1 but disabled
            replication_lag_tolerance_secs: 10,
            failover_enabled: false,
            replicas: vec![],
        };

        let result = manager
            .configure_replication(&db_id, config)
            .await;

        assert!(matches!(
            result,
            Err(DatabaseError::InvalidConfiguration(_))
        ));
    }

    #[tokio::test]
    async fn test_get_replication_status() {
        let manager = ReplicationManagerImpl::new();
        let db_id = DatabaseId(Uuid::new_v4());

        let config = ReplicationConfig {
            enabled: true,
            replication_factor: 3,
            replication_lag_tolerance_secs: 10,
            failover_enabled: true,
            replicas: vec!["replica1".to_string()],
        };

        manager
            .configure_replication(&db_id, config)
            .await
            .unwrap();

        let status = manager.get_replication_status(&db_id).await.unwrap();
        assert!(status.is_primary);
        assert_eq!(status.total_replicas, 1);
    }

    #[tokio::test]
    async fn test_add_replica() {
        let manager = ReplicationManagerImpl::new();
        let db_id = DatabaseId(Uuid::new_v4());

        let config = ReplicationConfig {
            enabled: true,
            replication_factor: 3,
            replication_lag_tolerance_secs: 10,
            failover_enabled: true,
            replicas: vec!["replica1".to_string()],
        };

        manager
            .configure_replication(&db_id, config)
            .await
            .unwrap();

        let new_replica_id = manager
            .add_replica(&db_id, "replica2_host".to_string())
            .await
            .unwrap();

        let status = manager.get_replication_status(&db_id).await.unwrap();
        assert_eq!(status.total_replicas, 2);
        assert!(!new_replica_id.is_empty());
    }

    #[tokio::test]
    async fn test_remove_replica() {
        let manager = ReplicationManagerImpl::new();
        let db_id = DatabaseId(Uuid::new_v4());

        let config = ReplicationConfig {
            enabled: true,
            replication_factor: 3,
            replication_lag_tolerance_secs: 10,
            failover_enabled: true,
            replicas: vec!["replica1".to_string(), "replica2".to_string()],
        };

        manager
            .configure_replication(&db_id, config)
            .await
            .unwrap();

        manager.remove_replica("replica1").await.unwrap();

        let status = manager.get_replication_status(&db_id).await.unwrap();
        assert_eq!(status.total_replicas, 1);
    }

    #[tokio::test]
    async fn test_check_replication_lag() {
        let manager = ReplicationManagerImpl::new();
        let db_id = DatabaseId(Uuid::new_v4());

        let config = ReplicationConfig {
            enabled: true,
            replication_factor: 3,
            replication_lag_tolerance_secs: 10,
            failover_enabled: true,
            replicas: vec!["replica1".to_string()],
        };

        manager
            .configure_replication(&db_id, config)
            .await
            .unwrap();

        let lag = manager.check_replication_lag(&db_id).await.unwrap();
        assert_eq!(lag, 0);
    }

    #[tokio::test]
    async fn test_trigger_failover() {
        let manager = ReplicationManagerImpl::new();
        let db_id = DatabaseId(Uuid::new_v4());

        let config = ReplicationConfig {
            enabled: true,
            replication_factor: 3,
            replication_lag_tolerance_secs: 10,
            failover_enabled: true,
            replicas: vec!["replica1".to_string()],
        };

        manager
            .configure_replication(&db_id, config)
            .await
            .unwrap();

        manager.trigger_failover(&db_id).await.unwrap();

        let status = manager.get_replication_status(&db_id).await.unwrap();
        assert!(!status.is_primary);
    }
}
