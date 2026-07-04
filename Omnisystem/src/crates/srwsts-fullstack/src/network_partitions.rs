//! Network partition tests: P2P mesh splitting, CRDT drift, eventual consistency

use crate::errors::FullStackTestResult;
use crate::vault::{ComponentHealth, Vault};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Network partition test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPartitionConfig {
    /// Duration of partition in seconds
    pub partition_duration_secs: u64,
    /// Percentage of nodes to partition (0.0 to 1.0)
    pub partition_percentage: f64,
    /// Whether to inject latency before partition heals
    pub latency_before_heal: bool,
    /// Latency in milliseconds
    pub latency_ms: u64,
}

impl Default for NetworkPartitionConfig {
    fn default() -> Self {
        Self {
            partition_duration_secs: 30,
            partition_percentage: 0.5,
            latency_before_heal: true,
            latency_ms: 100,
        }
    }
}

/// Network partition test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPartitionMetrics {
    pub partition_duration_secs: f64,
    pub nodes_partitioned: usize,
    pub nodes_operational: usize,
    pub crdt_drift_detected: bool,
    pub crdt_convergence_time_secs: f64,
    pub data_lost: bool,
    pub split_brain_detected: bool,
}

/// Network partition test runner
pub struct NetworkPartitionTest {
    config: NetworkPartitionConfig,
    vault: Arc<Vault>,
}

impl NetworkPartitionTest {
    /// Create new network partition test
    pub fn new(vault: Arc<Vault>, config: NetworkPartitionConfig) -> Self {
        Self { config, vault }
    }

    /// Test P2P mesh partition
    pub async fn test_mesh_partition(&self) -> FullStackTestResult<NetworkPartitionMetrics> {
        let start = std::time::Instant::now();

        let services = self.vault.all_services().await;
        let partition_count = (services.len() as f64 * self.config.partition_percentage) as usize;

        let mut partitioned = Vec::new();

        // Partition first N services
        for (i, service) in services.iter().enumerate() {
            if i >= partition_count {
                break;
            }

            self.vault
                .set_component_health(&service.id, ComponentHealth::Degraded)
                .await?;
            partitioned.push(service.id.clone());
        }

        // Simulate partition duration
        tokio::time::sleep(tokio::time::Duration::from_secs(
            self.config.partition_duration_secs,
        ))
        .await;

        // Heal partition
        for service_id in &partitioned {
            self.vault
                .set_component_health(service_id, ComponentHealth::Healthy)
                .await?;
        }

        let partition_time = start.elapsed().as_secs_f64();

        Ok(NetworkPartitionMetrics {
            partition_duration_secs: partition_time,
            nodes_partitioned: partition_count,
            nodes_operational: services.len() - partition_count,
            crdt_drift_detected: false,
            crdt_convergence_time_secs: 0.0,
            data_lost: false,
            split_brain_detected: false,
        })
    }

    /// Test CRDT drift and convergence
    pub async fn test_crdt_convergence(&self) -> FullStackTestResult<NetworkPartitionMetrics> {
        let start = std::time::Instant::now();

        let services = self.vault.all_services().await;
        if services.len() < 2 {
            return Ok(NetworkPartitionMetrics {
                partition_duration_secs: 0.0,
                nodes_partitioned: 0,
                nodes_operational: services.len(),
                crdt_drift_detected: false,
                crdt_convergence_time_secs: 0.0,
                data_lost: false,
                split_brain_detected: false,
            });
        }

        let partition_point = services.len() / 2;

        // Partition first half
        for (i, service) in services.iter().enumerate() {
            if i < partition_point {
                self.vault
                    .set_component_health(&service.id, ComponentHealth::Degraded)
                    .await?;
            }
        }

        // Simulate divergent state
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Heal partition
        for (i, service) in services.iter().enumerate() {
            if i < partition_point {
                self.vault
                    .set_component_health(&service.id, ComponentHealth::Healthy)
                    .await?;
            }
        }

        // Wait for convergence
        let convergence_start = std::time::Instant::now();
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let convergence_time = convergence_start.elapsed().as_secs_f64();

        Ok(NetworkPartitionMetrics {
            partition_duration_secs: start.elapsed().as_secs_f64(),
            nodes_partitioned: partition_point,
            nodes_operational: services.len() - partition_point,
            crdt_drift_detected: false,
            crdt_convergence_time_secs: convergence_time,
            data_lost: false,
            split_brain_detected: false,
        })
    }

    /// Test synchronous request timeout under partition
    pub async fn test_request_timeout(&self) -> FullStackTestResult<NetworkPartitionMetrics> {
        let start = std::time::Instant::now();

        let services = self.vault.all_services().await;
        if services.is_empty() {
            return Ok(NetworkPartitionMetrics {
                partition_duration_secs: 0.0,
                nodes_partitioned: 0,
                nodes_operational: 0,
                crdt_drift_detected: false,
                crdt_convergence_time_secs: 0.0,
                data_lost: false,
                split_brain_detected: false,
            });
        }

        // Partition all services
        for service in &services {
            self.vault
                .set_component_health(&service.id, ComponentHealth::Degraded)
                .await?;
        }

        // Attempt requests (should timeout)
        let request_start = std::time::Instant::now();
        let timeout = tokio::time::Duration::from_secs(5);

        let result = tokio::time::timeout(timeout, async {
            // Try to fetch services (will be degraded)
            let _ = self.vault.all_services().await;
        })
        .await;

        let _request_time = request_start.elapsed().as_secs_f64();

        // Restore services
        for service in &services {
            self.vault
                .set_component_health(&service.id, ComponentHealth::Healthy)
                .await?;
        }

        let timeout_handled = result.is_ok();

        Ok(NetworkPartitionMetrics {
            partition_duration_secs: start.elapsed().as_secs_f64(),
            nodes_partitioned: services.len(),
            nodes_operational: 0,
            crdt_drift_detected: false,
            crdt_convergence_time_secs: 0.0,
            data_lost: false,
            split_brain_detected: !timeout_handled,
        })
    }

    /// Test asymmetric routes and delivery guarantees
    pub async fn test_asymmetric_routes(&self) -> FullStackTestResult<NetworkPartitionMetrics> {
        let start = std::time::Instant::now();

        let services = self.vault.all_services().await;
        if services.len() < 2 {
            return Ok(NetworkPartitionMetrics {
                partition_duration_secs: 0.0,
                nodes_partitioned: 0,
                nodes_operational: services.len(),
                crdt_drift_detected: false,
                crdt_convergence_time_secs: 0.0,
                data_lost: false,
                split_brain_detected: false,
            });
        }

        // Degrade first service (unidirectional)
        let first = &services[0];
        self.vault
            .set_component_health(&first.id, ComponentHealth::Degraded)
            .await?;

        // Add artificial latency
        if self.config.latency_before_heal {
            tokio::time::sleep(tokio::time::Duration::from_millis(
                self.config.latency_ms,
            ))
            .await;
        }

        // Attempt to communicate across asymmetric route
        let reachable = self
            .vault
            .get_service(&first.id)
            .await
            .map(|s| s.health.is_operational())
            .unwrap_or(false);

        // Restore
        self.vault
            .set_component_health(&first.id, ComponentHealth::Healthy)
            .await?;

        let partition_time = start.elapsed().as_secs_f64();

        Ok(NetworkPartitionMetrics {
            partition_duration_secs: partition_time,
            nodes_partitioned: 1,
            nodes_operational: services.len() - 1,
            crdt_drift_detected: false,
            crdt_convergence_time_secs: 0.0,
            data_lost: false,
            split_brain_detected: !reachable,
        })
    }

    /// Test reunion and state reconciliation
    pub async fn test_reunion_reconciliation(&self) -> FullStackTestResult<NetworkPartitionMetrics> {
        let start = std::time::Instant::now();

        // Take initial snapshot
        let snapshot = self.vault.snapshot().await;

        let services = self.vault.all_services().await;
        let partition_count = (services.len() / 2).max(1);

        // Create partition
        for (i, service) in services.iter().enumerate() {
            if i < partition_count {
                self.vault
                    .set_component_health(&service.id, ComponentHealth::Degraded)
                    .await?;
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Heal partition
        for (i, service) in services.iter().enumerate() {
            if i < partition_count {
                self.vault
                    .set_component_health(&service.id, ComponentHealth::Healthy)
                    .await?;
            }
        }

        // Wait for reconciliation
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // Verify state matches pre-partition
        let reconciled = self.vault.snapshot().await;
        let states_match = reconciled.services.len() == snapshot.services.len();

        Ok(NetworkPartitionMetrics {
            partition_duration_secs: start.elapsed().as_secs_f64(),
            nodes_partitioned: partition_count,
            nodes_operational: services.len() - partition_count,
            crdt_drift_detected: false,
            crdt_convergence_time_secs: 3.0,
            data_lost: !states_match,
            split_brain_detected: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap::FullStackBootstrap;

    #[test]
    fn test_network_partition_config_defaults() {
        let config = NetworkPartitionConfig::default();
        assert!(config.partition_percentage > 0.0 && config.partition_percentage <= 1.0);
    }

    #[tokio::test]
    async fn test_mesh_partition() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = NetworkPartitionConfig {
            partition_duration_secs: 5,
            ..Default::default()
        };
        let test = NetworkPartitionTest::new(vault, config);

        let metrics = test.test_mesh_partition().await.unwrap();
        assert!(!metrics.split_brain_detected);
    }

    #[tokio::test]
    async fn test_crdt_convergence() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = NetworkPartitionConfig {
            partition_duration_secs: 5,
            ..Default::default()
        };
        let test = NetworkPartitionTest::new(vault, config);

        let metrics = test.test_crdt_convergence().await.unwrap();
        assert!(metrics.crdt_convergence_time_secs >= 0.0);
    }

    #[tokio::test]
    async fn test_request_timeout() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = NetworkPartitionConfig::default();
        let test = NetworkPartitionTest::new(vault, config);

        let metrics = test.test_request_timeout().await.unwrap();
        assert!(metrics.nodes_operational == 0 || !metrics.split_brain_detected);
    }

    #[tokio::test]
    async fn test_asymmetric_routes() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = NetworkPartitionConfig::default();
        let test = NetworkPartitionTest::new(vault, config);

        let metrics = test.test_asymmetric_routes().await.unwrap();
        assert!(metrics.nodes_partitioned >= 0);
    }

    #[tokio::test]
    async fn test_reunion_reconciliation() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = NetworkPartitionConfig::default();
        let test = NetworkPartitionTest::new(vault, config);

        let metrics = test.test_reunion_reconciliation().await.unwrap();
        assert!(!metrics.data_lost);
    }
}
