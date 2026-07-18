/// Multi-Region Replication
///
/// Cross-region data replication, geo-distributed clusters, disaster recovery

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// Region configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub name: String,
    pub primary: bool,
    pub replicas: u32,
    pub latency_ms: u32,
}

/// Multi-region configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiRegionConfig {
    pub regions: HashMap<String, Region>,
    pub replication_factor: u32,
}

impl MultiRegionConfig {
    /// Create multi-region config
    pub fn new(replication_factor: u32) -> Result<Self> {
        info!(
            "Initializing Multi-Region Config with replication factor: {}",
            replication_factor
        );
        Ok(Self {
            regions: HashMap::new(),
            replication_factor,
        })
    }

    /// Add region
    pub fn add_region(&mut self, name: String, primary: bool, replicas: u32) -> Result<()> {
        info!("Adding region: {} (primary: {})", name, primary);
        self.regions.insert(
            name.clone(),
            Region {
                name,
                primary,
                replicas,
                latency_ms: 0,
            },
        );
        Ok(())
    }

    /// Get primary region
    pub fn primary_region(&self) -> Option<&Region> {
        self.regions.values().find(|r| r.primary)
    }

    /// Get replica regions
    pub fn replica_regions(&self) -> Vec<&Region> {
        self.regions.values().filter(|r| !r.primary).collect()
    }
}

/// Region replication manager
pub struct RegionReplicationManager {
    pub config: MultiRegionConfig,
    replication_lag: HashMap<String, u64>,
}

impl RegionReplicationManager {
    /// Create replication manager
    pub fn new(config: MultiRegionConfig) -> Result<Self> {
        info!("Initializing Region Replication Manager");
        Ok(Self {
            config,
            replication_lag: HashMap::new(),
        })
    }

    /// Replicate to all regions
    pub async fn replicate_to_all_regions(&mut self, data: &[u8]) -> Result<()> {
        info!(
            "Replicating {} bytes to all regions",
            data.len()
        );

        // Replicate to primary synchronously
        if let Some(primary) = self.config.primary_region() {
            self.replicate_to_region(&primary.name, data).await?;
            self.replication_lag.insert(primary.name.clone(), 0);
        }

        // Replicate to replicas asynchronously
        for replica_region in self.config.replica_regions() {
            self.replicate_to_region(&replica_region.name, data).await?;
            let lag = replica_region.latency_ms as u64;
            self.replication_lag.insert(replica_region.name.clone(), lag);
        }

        Ok(())
    }

    /// Replicate to specific region
    async fn replicate_to_region(&self, region_name: &str, _data: &[u8]) -> Result<()> {
        if let Some(region) = self.config.regions.get(region_name) {
            info!(
                "Replicating to region: {} (latency: {}ms)",
                region.name, region.latency_ms
            );

            // Simulate network latency
            tokio::time::sleep(tokio::time::Duration::from_millis(
                region.latency_ms as u64,
            ))
            .await;
        }

        Ok(())
    }

    /// Get replication status
    pub fn replication_status(&self) -> HashMap<String, u64> {
        self.replication_lag.clone()
    }

    /// Check if replication is healthy (all regions synced)
    pub fn is_healthy(&self) -> bool {
        // Healthy if all regions have low lag (< 5s)
        self.replication_lag
            .values()
            .all(|&lag| lag < 5000)
    }

    /// Failover to replica region
    pub async fn failover_to_replica(&mut self, replica_name: &str) -> Result<()> {
        info!("Failing over to region: {}", replica_name);

        // Mark current primary as down
        if let Some(primary) = self.config.regions.values_mut().find(|r| r.primary) {
            primary.primary = false;
            info!("Demoted region: {}", primary.name);
        }

        // Promote replica to primary
        if let Some(replica) = self.config.regions.get_mut(replica_name) {
            replica.primary = true;
            info!("Promoted region: {} to primary", replica.name);
        }

        Ok(())
    }

    /// Get recovery point objective (RPO) - acceptable data loss
    pub fn rpo_seconds(&self) -> u32 {
        // RPO = max replication lag
        self.replication_lag
            .values()
            .max()
            .map(|&lag| (lag / 1000) as u32)
            .unwrap_or(0)
    }

    /// Get recovery time objective (RTO) - time to restore
    pub fn rto_seconds(&self) -> u32 {
        // RTO = max network latency between regions
        self.config
            .regions
            .values()
            .map(|r| r.latency_ms)
            .max()
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_region_config() {
        let mut config = MultiRegionConfig::new(3).unwrap();
        config.add_region("us-east".to_string(), true, 3).unwrap();
        config.add_region("us-west".to_string(), false, 3).unwrap();
        config.add_region("eu-west".to_string(), false, 3).unwrap();

        assert_eq!(config.regions.len(), 3);
        assert!(config.primary_region().is_some());
        assert_eq!(config.replica_regions().len(), 2);
    }

    #[tokio::test]
    async fn test_region_replication() {
        let mut config = MultiRegionConfig::new(3).unwrap();
        config.add_region("us-east".to_string(), true, 3).unwrap();
        config.add_region("us-west".to_string(), false, 3).unwrap();

        let mut mgr = RegionReplicationManager::new(config).unwrap();
        let data = vec![1, 2, 3, 4, 5];

        mgr.replicate_to_all_regions(&data).await.unwrap();

        let status = mgr.replication_status();
        assert!(!status.is_empty());
    }

    #[tokio::test]
    async fn test_failover() {
        let mut config = MultiRegionConfig::new(3).unwrap();
        config.add_region("us-east".to_string(), true, 3).unwrap();
        config.add_region("us-west".to_string(), false, 3).unwrap();

        let mut mgr = RegionReplicationManager::new(config).unwrap();
        assert!(mgr.config.primary_region().unwrap().name == "us-east");

        mgr.failover_to_replica("us-west").await.unwrap();
        assert!(mgr.config.primary_region().unwrap().name == "us-west");
    }
}
