//! Cache hierarchy emulation with latency modeling

use serde::{Deserialize, Serialize};

/// Cache level definition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheLevel {
    /// L1 cache (instruction)
    L1I,
    /// L1 cache (data)
    L1D,
    /// L2 cache (unified)
    L2,
    /// L3 cache (shared)
    L3,
    /// Memory
    Memory,
}

impl std::fmt::Display for CacheLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::L1I => write!(f, "L1I"),
            Self::L1D => write!(f, "L1D"),
            Self::L2 => write!(f, "L2"),
            Self::L3 => write!(f, "L3"),
            Self::Memory => write!(f, "Memory"),
        }
    }
}

/// Cache configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// L1 instruction cache size in bytes
    pub l1i_size: usize,
    /// L1 data cache size in bytes
    pub l1d_size: usize,
    /// L2 cache size in bytes
    pub l2_size: usize,
    /// L3 cache size in bytes (shared)
    pub l3_size: usize,
    /// Cache line size in bytes
    pub cache_line_size: usize,
    /// L1 access latency in cycles
    pub l1_latency: u32,
    /// L2 access latency in cycles
    pub l2_latency: u32,
    /// L3 access latency in cycles
    pub l3_latency: u32,
    /// Memory access latency in cycles
    pub memory_latency: u32,
    /// L1 associativity
    pub l1_associativity: usize,
    /// L2 associativity
    pub l2_associativity: usize,
    /// L3 associativity
    pub l3_associativity: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1i_size: 32 * 1024,             // 32 KB
            l1d_size: 32 * 1024,             // 32 KB
            l2_size: 512 * 1024,             // 512 KB
            l3_size: 8 * 1024 * 1024,        // 8 MB
            cache_line_size: 64,
            l1_latency: 4,                   // 4 cycles
            l2_latency: 12,                  // 12 cycles
            l3_latency: 42,                  // 42 cycles
            memory_latency: 200,             // 200 cycles (100ns @ 2GHz)
            l1_associativity: 8,
            l2_associativity: 8,
            l3_associativity: 12,
        }
    }
}

impl CacheConfig {
    /// Create a high-performance cache configuration
    pub fn high_performance() -> Self {
        Self {
            l1i_size: 64 * 1024,             // 64 KB
            l1d_size: 64 * 1024,             // 64 KB
            l2_size: 1024 * 1024,            // 1 MB
            l3_size: 32 * 1024 * 1024,       // 32 MB
            cache_line_size: 64,
            l1_latency: 3,                   // 3 cycles
            l2_latency: 10,                  // 10 cycles
            l3_latency: 35,                  // 35 cycles
            memory_latency: 150,             // 150 cycles
            l1_associativity: 8,
            l2_associativity: 8,
            l3_associativity: 20,
        }
    }

    /// Create an efficiency-optimized cache configuration
    pub fn efficiency() -> Self {
        Self {
            l1i_size: 16 * 1024,             // 16 KB
            l1d_size: 16 * 1024,             // 16 KB
            l2_size: 128 * 1024,             // 128 KB
            l3_size: 2 * 1024 * 1024,        // 2 MB
            cache_line_size: 64,
            l1_latency: 4,                   // 4 cycles
            l2_latency: 14,                  // 14 cycles
            l3_latency: 45,                  // 45 cycles
            memory_latency: 250,             // 250 cycles
            l1_associativity: 4,
            l2_associativity: 4,
            l3_associativity: 8,
        }
    }
}

/// Cache hit/miss statistics
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total hits at this level
    pub hits: u64,
    /// Total misses at this level
    pub misses: u64,
    /// Evictions from this level
    pub evictions: u64,
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }
}

impl CacheStats {
    /// Calculate hit rate as percentage (0-100)
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            return 100.0;
        }
        (self.hits as f64 / total as f64) * 100.0
    }

    /// Calculate miss rate as percentage (0-100)
    pub fn miss_rate(&self) -> f64 {
        100.0 - self.hit_rate()
    }
}

/// Cache hierarchy manager
#[derive(Debug, Clone)]
pub struct CacheHierarchy {
    /// Configuration
    pub config: CacheConfig,
    /// Per-level statistics
    pub stats: Vec<CacheStats>,
}

impl CacheHierarchy {
    /// Create a new cache hierarchy
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            stats: vec![CacheStats::default(); 4], // L1I, L1D, L2, L3
        }
    }

    /// Record a cache hit at a specific level
    pub fn record_hit(&mut self, level: CacheLevel) {
        let idx = match level {
            CacheLevel::L1I | CacheLevel::L1D => 0,
            CacheLevel::L2 => 1,
            CacheLevel::L3 => 2,
            CacheLevel::Memory => return,
        };

        if idx < self.stats.len() {
            self.stats[idx].hits += 1;
        }
    }

    /// Record a cache miss at a specific level
    pub fn record_miss(&mut self, level: CacheLevel) {
        let idx = match level {
            CacheLevel::L1I | CacheLevel::L1D => 0,
            CacheLevel::L2 => 1,
            CacheLevel::L3 => 2,
            CacheLevel::Memory => return,
        };

        if idx < self.stats.len() {
            self.stats[idx].misses += 1;
        }
    }

    /// Get latency for a specific cache level in cycles
    pub fn get_latency(&self, level: CacheLevel) -> u32 {
        match level {
            CacheLevel::L1I | CacheLevel::L1D => self.config.l1_latency,
            CacheLevel::L2 => self.config.l2_latency,
            CacheLevel::L3 => self.config.l3_latency,
            CacheLevel::Memory => self.config.memory_latency,
        }
    }

    /// Calculate total cycles for access pattern (all misses to memory)
    pub fn access_latency_worst_case(&self) -> u32 {
        self.config.memory_latency
    }

    /// Calculate total cycles for best case (L1 hit)
    pub fn access_latency_best_case(&self) -> u32 {
        self.config.l1_latency
    }

    /// Get average access latency based on statistics
    pub fn average_access_latency(&self) -> f64 {
        let total_accesses: u64 = self.stats.iter().map(|s| s.hits + s.misses).sum();
        if total_accesses == 0 {
            return self.config.l1_latency as f64;
        }

        let weighted_latency = (self.stats[0].hits as f64 * self.config.l1_latency as f64)
            + (self.stats[0].misses as f64
                * (self.config.l1_latency as f64 + self.config.l2_latency as f64))
            + (self.stats[1].hits as f64 * self.config.l2_latency as f64)
            + (self.stats[1].misses as f64
                * (self.config.l2_latency as f64 + self.config.l3_latency as f64))
            + (self.stats[2].hits as f64 * self.config.l3_latency as f64)
            + (self.stats[2].misses as f64
                * (self.config.l3_latency as f64 + self.config.memory_latency as f64));

        weighted_latency / total_accesses as f64
    }

    /// Reset all statistics
    pub fn reset_stats(&mut self) {
        for stat in self.stats.iter_mut() {
            *stat = CacheStats::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.l1i_size, 32 * 1024);
        assert_eq!(config.cache_line_size, 64);
        assert_eq!(config.l1_latency, 4);
    }

    #[test]
    fn test_cache_config_high_performance() {
        let config = CacheConfig::high_performance();
        assert_eq!(config.l1i_size, 64 * 1024);
        assert_eq!(config.l3_size, 32 * 1024 * 1024);
    }

    #[test]
    fn test_cache_config_efficiency() {
        let config = CacheConfig::efficiency();
        assert_eq!(config.l1i_size, 16 * 1024);
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let mut stats = CacheStats::default();
        stats.hits = 95;
        stats.misses = 5;
        assert_eq!(stats.hit_rate(), 95.0);
    }

    #[test]
    fn test_cache_stats_miss_rate() {
        let mut stats = CacheStats::default();
        stats.hits = 90;
        stats.misses = 10;
        assert_eq!(stats.miss_rate(), 10.0);
    }

    #[test]
    fn test_cache_hierarchy_creation() {
        let config = CacheConfig::default();
        let hierarchy = CacheHierarchy::new(config);
        assert_eq!(hierarchy.stats.len(), 4);
    }

    #[test]
    fn test_cache_hierarchy_record_hit() {
        let config = CacheConfig::default();
        let mut hierarchy = CacheHierarchy::new(config);
        hierarchy.record_hit(CacheLevel::L1I);
        assert_eq!(hierarchy.stats[0].hits, 1);
    }

    #[test]
    fn test_cache_hierarchy_latencies() {
        let config = CacheConfig::default();
        let hierarchy = CacheHierarchy::new(config);
        assert_eq!(hierarchy.get_latency(CacheLevel::L1I), 4);
        assert_eq!(hierarchy.get_latency(CacheLevel::L2), 12);
        assert_eq!(hierarchy.get_latency(CacheLevel::L3), 42);
        assert_eq!(hierarchy.get_latency(CacheLevel::Memory), 200);
    }
}
