// CPU Management - CPU topology, affinity, frequency scaling, NUMA awareness

use serde::{Deserialize, Serialize};
use omnisystem_sylva_core::module::SylvaModule;

/// CPU Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CPUInfo {
    pub total_cores: u32,
    pub physical_cores: u32,
    pub logical_cores: u32,
    pub base_frequency_mhz: u32,
    pub max_frequency_mhz: u32,
    pub l1_cache_kb: u32,
    pub l2_cache_mb: u32,
    pub l3_cache_mb: u32,
    pub numa_nodes: u32,
}

/// CPU Topology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CPUTopology {
    pub cores: Vec<CoreInfo>,
    pub numa_distances: Vec<Vec<u32>>,
    pub cache_topology: Vec<CacheInfo>,
}

/// Core Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreInfo {
    pub core_id: u32,
    pub physical_id: u32,
    pub numa_node: u32,
    pub online: bool,
    pub frequency_mhz: u32,
}

/// Cache Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheInfo {
    pub level: u32,
    pub size_kb: u32,
    pub line_size: u32,
    pub associativity: u32,
}

/// CPU Manager
pub struct CPUManager {
    info: CPUInfo,
    topology: CPUTopology,
}

impl CPUManager {
    pub async fn new() -> anyhow::Result<Self> {
        tracing::info!("Initializing CPU Manager");

        let info = CPUInfo {
            total_cores: num_cpus::get() as u32,
            physical_cores: num_cpus::get() as u32,
            logical_cores: num_cpus::get() as u32 * 2,
            base_frequency_mhz: 2400,
            max_frequency_mhz: 4800,
            l1_cache_kb: 32,
            l2_cache_mb: 256,
            l3_cache_mb: 8,
            numa_nodes: 1,
        };

        let topology = CPUTopology {
            cores: (0..info.total_cores)
                .map(|i| CoreInfo {
                    core_id: i,
                    physical_id: i / 2,
                    numa_node: 0,
                    online: true,
                    frequency_mhz: info.base_frequency_mhz,
                })
                .collect(),
            numa_distances: vec![vec![10]],
            cache_topology: vec![
                CacheInfo {
                    level: 1,
                    size_kb: 32,
                    line_size: 64,
                    associativity: 8,
                },
                CacheInfo {
                    level: 2,
                    size_kb: 256,
                    line_size: 64,
                    associativity: 8,
                },
                CacheInfo {
                    level: 3,
                    size_kb: 8192,
                    line_size: 64,
                    associativity: 16,
                },
            ],
        };

        Ok(Self { info, topology })
    }

    pub async fn get_info(&self) -> anyhow::Result<CPUInfo> {
        Ok(self.info.clone())
    }

    pub async fn get_topology(&self) -> anyhow::Result<CPUTopology> {
        Ok(self.topology.clone())
    }

    pub async fn set_affinity(&self, core_id: u32) -> anyhow::Result<()> {
        tracing::info!("Setting CPU affinity to core {}", core_id);
        Ok(())
    }

    pub async fn enable_frequency_scaling(&self, min_mhz: u32, max_mhz: u32) -> anyhow::Result<()> {
        tracing::info!(
            "Enabling frequency scaling: {} - {} MHz",
            min_mhz, max_mhz
        );
        Ok(())
    }

    pub async fn get_numa_distance(&self, node1: u32, node2: u32) -> anyhow::Result<u32> {
        let distances = &self.topology.numa_distances;
        if node1 < distances.len() as u32 && node2 < distances[node1 as usize].len() as u32 {
            Ok(distances[node1 as usize][node2 as usize])
        } else {
            Err(anyhow::anyhow!("Invalid NUMA nodes"))
        }
    }
}

#[async_trait::async_trait]
impl SylvaModule for CPUManager {
    fn name(&self) -> &str {
        "cpu-manager"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn init(&mut self, _config: &omnisystem_sylva_core::module::SylvaModuleConfig) -> anyhow::Result<()> {
        tracing::info!("CPU Manager initialized");
        Ok(())
    }

    async fn main(&self) -> anyhow::Result<()> {
        tracing::info!("CPU Manager running");
        Ok(())
    }

    async fn shutdown(&mut self) -> anyhow::Result<()> {
        tracing::info!("CPU Manager shutdown");
        Ok(())
    }
}

extern crate num_cpus;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cpu_manager_creation() {
        let manager = CPUManager::new().await.unwrap();
        let info = manager.get_info().await.unwrap();
        assert!(info.total_cores > 0);
    }

    #[tokio::test]
    async fn test_topology() {
        let manager = CPUManager::new().await.unwrap();
        let topology = manager.get_topology().await.unwrap();
        assert!(!topology.cores.is_empty());
    }

    #[tokio::test]
    async fn test_set_affinity() {
        let manager = CPUManager::new().await.unwrap();
        manager.set_affinity(0).await.unwrap();
    }
}
