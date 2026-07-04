//! Memory emulator with cache hierarchy and page table emulation

use crate::errors::{EmulationError, EmulationResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Memory model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryModel {
    /// Total physical memory size in bytes
    pub total_size_bytes: usize,
    /// Page size in bytes
    pub page_size: usize,
    /// Enable NUMA simulation
    pub numa_enabled: bool,
    /// Number of NUMA nodes
    pub numa_nodes: usize,
    /// Base memory latency in nanoseconds
    pub base_latency_ns: u32,
    /// NUMA remote latency multiplier (remote / local)
    pub numa_latency_multiplier: f64,
}

impl Default for MemoryModel {
    fn default() -> Self {
        Self {
            total_size_bytes: 16 * 1024 * 1024 * 1024, // 16 GB
            page_size: 4096,                             // 4 KB
            numa_enabled: true,
            numa_nodes: 2,
            base_latency_ns: 50,  // ~50ns local latency
            numa_latency_multiplier: 2.5, // ~125ns remote latency
        }
    }
}

impl MemoryModel {
    /// Create a configuration for a small memory system (test/embedded)
    pub fn small() -> Self {
        Self {
            total_size_bytes: 2 * 1024 * 1024 * 1024, // 2 GB
            page_size: 4096,
            numa_enabled: false,
            numa_nodes: 1,
            base_latency_ns: 50,
            numa_latency_multiplier: 1.0,
        }
    }

    /// Create a configuration for a large memory system (server/workstation)
    pub fn large() -> Self {
        Self {
            total_size_bytes: 256 * 1024 * 1024 * 1024, // 256 GB
            page_size: 4096,
            numa_enabled: true,
            numa_nodes: 8,
            base_latency_ns: 50,
            numa_latency_multiplier: 3.0,
        }
    }
}

/// Page table configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageTableConfig {
    /// Number of page table levels (typically 3-4 for x86-64)
    pub levels: usize,
    /// Latency per page table walk in cycles
    pub walk_latency: u32,
    /// TLB size for this level
    pub tlb_size: usize,
    /// TLB associativity
    pub tlb_associativity: usize,
}

impl Default for PageTableConfig {
    fn default() -> Self {
        Self {
            levels: 4,        // 4-level page table (x86-64)
            walk_latency: 100, // ~100 cycles for full walk
            tlb_size: 64,      // 64 entries
            tlb_associativity: 8,
        }
    }
}

/// Memory access type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessType {
    /// Instruction fetch
    InstructionFetch,
    /// Data read
    DataRead,
    /// Data write
    DataWrite,
    /// Prefetch
    Prefetch,
}

/// Memory access statistics
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Total memory accesses
    pub total_accesses: u64,
    /// Instruction fetches
    pub instruction_fetches: u64,
    /// Data reads
    pub data_reads: u64,
    /// Data writes
    pub data_writes: u64,
    /// TLB hits
    pub tlb_hits: u64,
    /// TLB misses (require page table walk)
    pub tlb_misses: u64,
    /// Bytes read
    pub bytes_read: u64,
    /// Bytes written
    pub bytes_written: u64,
}

impl MemoryStats {
    /// Calculate TLB hit rate as percentage
    pub fn tlb_hit_rate(&self) -> f64 {
        let total = self.tlb_hits + self.tlb_misses;
        if total == 0 {
            return 100.0;
        }
        (self.tlb_hits as f64 / total as f64) * 100.0
    }
}

/// Memory emulator trait
#[async_trait]
pub trait MemoryEmulator: Send + Sync {
    /// Read from memory
    async fn read(&self, address: u64, size: usize) -> EmulationResult<Vec<u8>>;

    /// Write to memory
    async fn write(&self, address: u64, data: &[u8]) -> EmulationResult<()>;

    /// Reset memory to initial state
    async fn reset(&self) -> EmulationResult<()>;
}

/// NUMA-aware memory controller
#[derive(Debug, Clone)]
pub struct NumaController {
    /// Number of NUMA nodes
    pub num_nodes: usize,
    /// Memory per node
    pub memory_per_node: usize,
    /// Access statistics per node
    pub stats: HashMap<usize, MemoryStats>,
}

impl NumaController {
    /// Create a new NUMA controller
    pub fn new(num_nodes: usize, memory_per_node: usize) -> Self {
        let mut stats = HashMap::new();
        for i in 0..num_nodes {
            stats.insert(i, MemoryStats::default());
        }

        Self {
            num_nodes,
            memory_per_node,
            stats,
        }
    }

    /// Get the NUMA node for an address
    pub fn address_to_node(&self, address: u64) -> usize {
        (address as usize / self.memory_per_node) % self.num_nodes
    }

    /// Calculate latency for accessing a remote node
    pub fn remote_access_latency(&self, from_node: usize, to_node: usize) -> f64 {
        if from_node == to_node {
            1.0 // Local access multiplier
        } else {
            2.5 // Remote access multiplier
        }
    }

    /// Record memory access
    pub fn record_access(&mut self, address: u64, access_type: AccessType, size: usize) {
        let node = self.address_to_node(address);
        if let Some(stats) = self.stats.get_mut(&node) {
            stats.total_accesses += 1;
            match access_type {
                AccessType::InstructionFetch => {
                    stats.instruction_fetches += 1;
                    stats.bytes_read += size as u64;
                }
                AccessType::DataRead => {
                    stats.data_reads += 1;
                    stats.bytes_read += size as u64;
                }
                AccessType::DataWrite => {
                    stats.data_writes += 1;
                    stats.bytes_written += size as u64;
                }
                AccessType::Prefetch => {
                    stats.bytes_read += size as u64;
                }
            }
        }
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        for stats in self.stats.values_mut() {
            *stats = MemoryStats::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_model_default() {
        let model = MemoryModel::default();
        assert_eq!(model.total_size_bytes, 16 * 1024 * 1024 * 1024);
        assert_eq!(model.page_size, 4096);
        assert!(model.numa_enabled);
    }

    #[test]
    fn test_memory_model_small() {
        let model = MemoryModel::small();
        assert_eq!(model.total_size_bytes, 2 * 1024 * 1024 * 1024);
        assert!(!model.numa_enabled);
    }

    #[test]
    fn test_memory_model_large() {
        let model = MemoryModel::large();
        assert_eq!(model.total_size_bytes, 256 * 1024 * 1024 * 1024);
        assert_eq!(model.numa_nodes, 8);
    }

    #[test]
    fn test_page_table_config_default() {
        let config = PageTableConfig::default();
        assert_eq!(config.levels, 4);
        assert_eq!(config.walk_latency, 100);
    }

    #[test]
    fn test_memory_stats_tlb_hit_rate() {
        let mut stats = MemoryStats::default();
        stats.tlb_hits = 95;
        stats.tlb_misses = 5;
        assert_eq!(stats.tlb_hit_rate(), 95.0);
    }

    #[test]
    fn test_numa_controller_creation() {
        let controller =
            NumaController::new(4, 4 * 1024 * 1024 * 1024);
        assert_eq!(controller.num_nodes, 4);
    }

    #[test]
    fn test_numa_address_to_node() {
        let mem_per_node = 4 * 1024 * 1024 * 1024; // 4GB per node
        let controller = NumaController::new(4, mem_per_node);

        let addr0 = 0x0;
        let addr1 = mem_per_node as u64;
        let addr2 = (mem_per_node * 2) as u64;

        assert_eq!(controller.address_to_node(addr0), 0);
        assert_eq!(controller.address_to_node(addr1), 1);
        assert_eq!(controller.address_to_node(addr2), 2);
    }

    #[test]
    fn test_numa_latency() {
        let controller = NumaController::new(4, 4 * 1024 * 1024 * 1024);

        assert_eq!(controller.remote_access_latency(0, 0), 1.0);
        assert_eq!(controller.remote_access_latency(0, 1), 2.5);
        assert_eq!(controller.remote_access_latency(2, 3), 2.5);
    }

    #[test]
    fn test_numa_record_access() {
        let mut controller = NumaController::new(2, 4 * 1024 * 1024 * 1024);

        controller.record_access(0, AccessType::DataRead, 64);
        if let Some(stats) = controller.stats.get(&0) {
            assert_eq!(stats.data_reads, 1);
            assert_eq!(stats.bytes_read, 64);
        }
    }
}
