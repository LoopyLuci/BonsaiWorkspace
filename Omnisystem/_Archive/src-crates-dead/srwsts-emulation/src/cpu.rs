//! CPU emulator trait and implementations

use crate::errors::EmulationResult; // Used in async_trait impl
use crate::interrupt::InterruptType;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// CPU emulator trait for cycle-level execution
#[async_trait]
pub trait CPUEmulator: Send + Sync {
    /// Execute one cycle of the CPU
    async fn cycle_execute(&self) -> EmulationResult<()>;

    /// Deliver an interrupt to this CPU
    async fn interrupt_deliver(&self, interrupt_type: InterruptType) -> EmulationResult<()>;

    /// Capture the current CPU state
    async fn state_capture(&self) -> EmulationResult<CPUState>;

    /// Reset CPU to initial state
    async fn reset(&self) -> EmulationResult<()>;
}

/// CPU core configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    /// L1 instruction cache size in bytes
    pub l1_icache_size: usize,
    /// L1 data cache size in bytes
    pub l1_dcache_size: usize,
    /// L2 cache size in bytes
    pub l2_cache_size: usize,
    /// Branch prediction buffer size
    pub bpb_size: usize,
    /// Instruction window size (IPC capability)
    pub instruction_window: usize,
    /// Whether SMT (simultaneous multithreading) is enabled
    pub smt_enabled: bool,
    /// Number of threads if SMT enabled
    pub threads_per_core: usize,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            l1_icache_size: 32 * 1024,      // 32 KB
            l1_dcache_size: 32 * 1024,      // 32 KB
            l2_cache_size: 512 * 1024,      // 512 KB
            bpb_size: 4096,
            instruction_window: 128,
            smt_enabled: true,
            threads_per_core: 2,
        }
    }
}

impl CoreConfig {
    /// Create a configuration for a high-performance core
    pub fn high_performance() -> Self {
        Self {
            l1_icache_size: 64 * 1024,      // 64 KB
            l1_dcache_size: 64 * 1024,      // 64 KB
            l2_cache_size: 1024 * 1024,     // 1 MB
            bpb_size: 16384,
            instruction_window: 256,
            smt_enabled: true,
            threads_per_core: 2,
        }
    }

    /// Create a configuration for an efficiency core
    pub fn efficiency() -> Self {
        Self {
            l1_icache_size: 16 * 1024,      // 16 KB
            l1_dcache_size: 16 * 1024,      // 16 KB
            l2_cache_size: 128 * 1024,      // 128 KB
            bpb_size: 1024,
            instruction_window: 64,
            smt_enabled: false,
            threads_per_core: 1,
        }
    }
}

/// CPU state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CPUState {
    /// Total instruction count since last reset
    pub instruction_count: u64,
    /// Total cycle count since last reset
    pub cycle_count: u64,
    /// Program counter
    pub program_counter: u64,
    /// General purpose registers (simulated as array)
    pub registers: Vec<u64>,
    /// Floating point registers
    pub fp_registers: Vec<f64>,
    /// Flags register (EFLAGS-like)
    pub flags: u32,
    /// Pending interrupt
    pub interrupt_pending: Option<InterruptType>,
    /// Whether CPU is in privilege mode
    pub privilege_mode: PrivilegeMode,
    /// CPU utilization (0-100%)
    pub utilization: u8,
    /// Pipeline stalls count
    pub pipeline_stalls: u64,
    /// Cache misses count
    pub cache_misses: u64,
    /// Branch mispredictions
    pub branch_mispredictions: u64,
}

impl Default for CPUState {
    fn default() -> Self {
        Self {
            instruction_count: 0,
            cycle_count: 0,
            program_counter: 0,
            registers: vec![0u64; 16],
            fp_registers: vec![0.0; 16],
            flags: 0x0200, // IF flag set
            interrupt_pending: None,
            privilege_mode: PrivilegeMode::User,
            utilization: 0,
            pipeline_stalls: 0,
            cache_misses: 0,
            branch_mispredictions: 0,
        }
    }
}

/// CPU privilege mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivilegeMode {
    /// User mode (ring 3)
    User,
    /// Kernel mode (ring 0)
    Kernel,
}

impl std::fmt::Display for PrivilegeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "User"),
            Self::Kernel => write!(f, "Kernel"),
        }
    }
}

/// CPU cache configuration and statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// L1 instruction cache hits
    pub l1i_hits: u64,
    /// L1 instruction cache misses
    pub l1i_misses: u64,
    /// L1 data cache hits
    pub l1d_hits: u64,
    /// L1 data cache misses
    pub l1d_misses: u64,
    /// L2 cache hits
    pub l2_hits: u64,
    /// L2 cache misses
    pub l2_misses: u64,
    /// L3 cache hits (if applicable)
    pub l3_hits: u64,
    /// L3 cache misses
    pub l3_misses: u64,
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            l1i_hits: 0,
            l1i_misses: 0,
            l1d_hits: 0,
            l1d_misses: 0,
            l2_hits: 0,
            l2_misses: 0,
            l3_hits: 0,
            l3_misses: 0,
        }
    }
}

impl CacheStats {
    /// Calculate overall cache hit rate (0-100%)
    pub fn hit_rate(&self) -> f64 {
        let total_accesses = self.l1i_hits
            + self.l1i_misses
            + self.l1d_hits
            + self.l1d_misses
            + self.l2_hits
            + self.l2_misses;

        if total_accesses == 0 {
            return 100.0;
        }

        let total_hits = self.l1i_hits + self.l1d_hits + self.l2_hits;
        (total_hits as f64 / total_accesses as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_config_default() {
        let config = CoreConfig::default();
        assert_eq!(config.l1_icache_size, 32 * 1024);
        assert!(config.smt_enabled);
    }

    #[test]
    fn test_core_config_high_performance() {
        let config = CoreConfig::high_performance();
        assert_eq!(config.l1_icache_size, 64 * 1024);
        assert_eq!(config.instruction_window, 256);
    }

    #[test]
    fn test_core_config_efficiency() {
        let config = CoreConfig::efficiency();
        assert_eq!(config.l1_icache_size, 16 * 1024);
        assert!(!config.smt_enabled);
    }

    #[test]
    fn test_cpu_state_default() {
        let state = CPUState::default();
        assert_eq!(state.instruction_count, 0);
        assert_eq!(state.registers.len(), 16);
        assert_eq!(state.fp_registers.len(), 16);
        assert_eq!(state.privilege_mode, PrivilegeMode::User);
    }

    #[test]
    fn test_privilege_mode_display() {
        assert_eq!(PrivilegeMode::User.to_string(), "User");
        assert_eq!(PrivilegeMode::Kernel.to_string(), "Kernel");
    }

    #[test]
    fn test_cache_stats_default() {
        let stats = CacheStats::default();
        assert_eq!(stats.hit_rate(), 100.0);
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let mut stats = CacheStats::default();
        stats.l1i_hits = 90;
        stats.l1i_misses = 10;
        stats.l1d_hits = 80;
        stats.l1d_misses = 20;

        let hit_rate = stats.hit_rate();
        assert!(hit_rate > 0.0 && hit_rate < 100.0);
    }
}
