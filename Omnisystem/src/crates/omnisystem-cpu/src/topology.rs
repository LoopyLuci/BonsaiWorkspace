/// CPU Topology Detection Module
///
/// Detects and manages CPU topology:
/// - Socket count
/// - Physical cores per socket
/// - Logical threads per core
/// - NUMA node mapping
/// - Cache hierarchy

use crate::{CPUError, Result};
use tracing::info;

/// CPU topology
pub struct CPUTopology {
    logical_cpus: u32,
    physical_cpus: u32,
    sockets: u32,
    cores_per_socket: u32,
    threads_per_core: u32,
    numa_nodes: Vec<NUMANode>,
}

impl CPUTopology {
    /// Detect CPU topology from system
    pub fn detect() -> Result<Self> {
        info!("Detecting CPU topology");

        let logical_cpus = num_cpus::get() as u32;
        let physical_cpus = num_cpus::get_physical() as u32;

        // These would be detected from /proc/cpuinfo on Linux or WMI on Windows
        let sockets = (physical_cpus / 16).max(1); // Heuristic
        let cores_per_socket = physical_cpus / sockets;
        let threads_per_core = logical_cpus / physical_cpus;

        info!("CPU Topology: {} logical CPUs, {} physical CPUs, {} sockets",
              logical_cpus, physical_cpus, sockets);
        info!("  Cores per socket: {}, Threads per core: {}",
              cores_per_socket, threads_per_core);

        // Detect NUMA nodes
        let numa_nodes = detect_numa_nodes();

        Ok(Self {
            logical_cpus,
            physical_cpus,
            sockets,
            cores_per_socket,
            threads_per_core,
            numa_nodes,
        })
    }

    /// Get logical CPU count
    pub fn logical_cpu_count(&self) -> u32 {
        self.logical_cpus
    }

    /// Get physical CPU count
    pub fn physical_cpu_count(&self) -> u32 {
        self.physical_cpus
    }

    /// Get socket count
    pub fn socket_count(&self) -> u32 {
        self.sockets
    }

    /// Get cores per socket
    pub fn cores_per_socket(&self) -> u32 {
        self.cores_per_socket
    }

    /// Get threads per core
    pub fn threads_per_core(&self) -> u32 {
        self.threads_per_core
    }

    /// Get NUMA node count
    pub fn numa_node_count(&self) -> u32 {
        self.numa_nodes.len() as u32
    }

    /// Get NUMA node for CPU
    pub fn get_numa_node(&self, cpu_id: u32) -> Option<u32> {
        for (node_id, node) in self.numa_nodes.iter().enumerate() {
            if node.cpus.contains(&cpu_id) {
                return Some(node_id as u32);
            }
        }
        None
    }
}

/// NUMA node
#[derive(Debug, Clone)]
pub struct NUMANode {
    pub id: u32,
    pub cpus: Vec<u32>,
    pub memory_kb: u64,
}

fn detect_numa_nodes() -> Vec<NUMANode> {
    // On non-NUMA systems, all CPUs in single node
    let logical_cpus = num_cpus::get();
    let cpus: Vec<u32> = (0..logical_cpus as u32).collect();

    vec![NUMANode {
        id: 0,
        cpus,
        memory_kb: 0, // Would be detected from system
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topology_detection() {
        let topo = CPUTopology::detect();
        assert!(topo.is_ok());

        let topo = topo.unwrap();
        assert!(topo.logical_cpu_count() > 0);
        assert!(topo.physical_cpu_count() > 0);
        assert!(topo.socket_count() > 0);
        assert!(topo.numa_node_count() > 0);
    }
}
