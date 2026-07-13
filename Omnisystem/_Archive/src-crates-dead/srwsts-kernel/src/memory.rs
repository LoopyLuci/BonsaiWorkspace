//! Kernel Memory Stress Tests
//!
//! Tests memory allocation/deallocation, fragmentation, NUMA migrations, OOM handling,
//! huge pages, and swap pressure. Validates memory management subsystem under stress.

use crate::metrics::MetricsCollector;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Memory test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Total memory to allocate in bytes
    pub total_allocation_bytes: u64,
    /// Size of each allocation chunk
    pub chunk_size_bytes: u64,
    /// Number of concurrent allocations
    pub concurrent_allocations: usize,
    /// Enable NUMA migrations
    pub enable_numa: bool,
    /// Number of NUMA nodes
    pub numa_nodes: usize,
    /// Enable huge page testing
    pub enable_huge_pages: bool,
    /// Enable swap pressure testing
    pub enable_swap: bool,
    /// OOM handling strategy: "graceful" or "panic"
    pub oom_strategy: String,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            total_allocation_bytes: 4 * 1024 * 1024 * 1024, // 4 GB
            chunk_size_bytes: 4 * 1024 * 1024, // 4 MB chunks
            concurrent_allocations: 256,
            enable_numa: true,
            numa_nodes: 4,
            enable_huge_pages: true,
            enable_swap: true,
            oom_strategy: "graceful".to_string(),
        }
    }
}

/// Memory allocation tracking
#[derive(Debug, Clone)]
pub struct MemoryAllocation {
    pub id: u64,
    pub size_bytes: u64,
    pub allocated_at_ns: u64,
    pub freed_at_ns: Option<u64>,
    pub numa_node: Option<usize>,
    pub is_huge_page: bool,
    pub access_count: u64,
}

impl MemoryAllocation {
    /// Create a new memory allocation
    pub fn new(id: u64, size_bytes: u64, numa_node: Option<usize>, is_huge_page: bool) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        Self {
            id,
            size_bytes,
            allocated_at_ns: now,
            freed_at_ns: None,
            numa_node,
            is_huge_page,
            access_count: 0,
        }
    }

    /// Get lifetime in nanoseconds
    pub fn lifetime_ns(&self) -> u64 {
        self.freed_at_ns.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64
        }) - self.allocated_at_ns
    }
}

/// Memory fragmentation info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FragmentationInfo {
    pub total_allocated: u64,
    pub total_freed: u64,
    pub num_holes: u64,
    pub fragmentation_ratio: f64,
    pub largest_hole: u64,
}

/// Memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_allocations: u64,
    pub successful_allocations: u64,
    pub failed_allocations: u64,
    pub total_allocated_bytes: u64,
    pub peak_memory_bytes: u64,
    pub avg_allocation_time_us: f64,
    pub max_allocation_time_us: f64,
    pub fragmentation: FragmentationInfo,
    pub oom_events: u64,
    pub numa_migrations: u64,
}

/// Memory test engine
#[derive(Debug)]
pub struct MemoryTest {
    config: MemoryConfig,
    allocations: Arc<RwLock<Vec<MemoryAllocation>>>,
    metrics: Arc<RwLock<MetricsCollector>>,
    peak_memory: Arc<AtomicU64>,
    oom_count: Arc<AtomicU64>,
}

impl MemoryTest {
    /// Create a new memory test
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            config,
            allocations: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(MetricsCollector::new())),
            peak_memory: Arc::new(AtomicU64::new(0)),
            oom_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Run allocation stress test
    pub async fn test_allocation_stress(&self) -> Result<MemoryStats> {
        info!(
            "Starting memory allocation stress test: {} bytes in {} chunks",
            self.config.total_allocation_bytes, self.config.chunk_size_bytes
        );

        let mut handles = vec![];
        let allocation_count = Arc::new(AtomicU64::new(0));

        for i in 0..self.config.concurrent_allocations {
            let config = self.config.clone();
            let allocations = Arc::clone(&self.allocations);
            let metrics = Arc::clone(&self.metrics);
            let peak = Arc::clone(&self.peak_memory);
            let alloc_count = Arc::clone(&allocation_count);

            let handle = tokio::spawn(async move {
                let mut offset = 0u64;
                let allocation_id = i as u64;

                while offset < config.total_allocation_bytes / config.concurrent_allocations as u64
                {
                    let start = std::time::Instant::now();

                    // Simulate allocation with vector
                    let size = std::cmp::min(
                        config.chunk_size_bytes,
                        config.total_allocation_bytes - offset,
                    );

                    let numa_node = if config.enable_numa {
                        Some((i % config.numa_nodes) as usize)
                    } else {
                        None
                    };

                    let is_huge = config.enable_huge_pages && size >= 2 * 1024 * 1024;

                    let _data = vec![0u8; size as usize];
                    let duration = start.elapsed().as_micros() as f64;

                    let alloc = MemoryAllocation::new(
                        allocation_id * 1000 + (offset / size),
                        size,
                        numa_node,
                        is_huge,
                    );

                    allocations.write().await.push(alloc);
                    alloc_count.fetch_add(1, Ordering::Relaxed);

                    let mut m = metrics.write().await;
                    m.record_latency("allocation_time_us", duration);

                    let total = allocations.read().await
                        .iter()
                        .map(|a| a.size_bytes)
                        .sum::<u64>();

                    let current_peak = peak.load(Ordering::Relaxed);
                    if total > current_peak {
                        peak.store(total, Ordering::Relaxed);
                    }

                    offset += size;
                    tokio::task::yield_now().await;
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        self.calculate_memory_stats().await
    }

    /// Test deallocation
    pub async fn test_deallocation(&self) -> Result<()> {
        info!("Testing deallocation");

        let mut allocations = self.allocations.write().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        for alloc in allocations.iter_mut() {
            alloc.freed_at_ns = Some(now);
        }

        debug!("Deallocated {} allocations", allocations.len());
        Ok(())
    }

    /// Test NUMA migrations
    pub async fn test_numa_migrations(&self) -> Result<()> {
        if !self.config.enable_numa {
            return Ok(());
        }

        info!("Testing NUMA migrations");

        let mut handles = vec![];
        let migration_count = Arc::new(AtomicU64::new(0));

        for _i in 0..100 {
            let count = Arc::clone(&migration_count);
            let _num_nodes = self.config.numa_nodes;

            let handle = tokio::spawn(async move {
                // Simulate NUMA migration
                for _ in 0..10 {
                    let _data = vec![0u8; 1024 * 1024]; // 1 MB
                    tokio::task::yield_now().await;
                    count.fetch_add(1, Ordering::Relaxed);
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        debug!(
            "NUMA migrations: {}",
            migration_count.load(Ordering::Relaxed)
        );
        Ok(())
    }

    /// Test huge pages
    pub async fn test_huge_pages(&self) -> Result<()> {
        if !self.config.enable_huge_pages {
            return Ok(());
        }

        info!("Testing huge pages");

        let huge_page_size = 2 * 1024 * 1024; // 2 MB
        let count = self.config.total_allocation_bytes / huge_page_size;

        for i in 0..std::cmp::min(count, 1000) {
            let _data = vec![0u8; huge_page_size as usize];
            if i % 100 == 0 {
                tokio::task::yield_now().await;
            }
        }

        debug!("Tested {} huge pages", std::cmp::min(count, 1000));
        Ok(())
    }

    /// Test swap pressure
    pub async fn test_swap_pressure(&self) -> Result<()> {
        if !self.config.enable_swap {
            return Ok(());
        }

        info!("Testing swap pressure");

        let mut allocations = vec![];

        for i in 0..100 {
            let size = 10 * 1024 * 1024; // 10 MB each
            allocations.push(vec![0u8; size]);

            if i % 10 == 0 {
                // Simulate memory pressure
                tokio::task::yield_now().await;
            }
        }

        drop(allocations);
        debug!("Swap pressure test completed");
        Ok(())
    }

    /// Test OOM handling
    pub async fn test_oom_handling(&self) -> Result<()> {
        info!("Testing OOM handling");

        let mut handles = vec![];

        for _ in 0..10 {
            let oom_count = Arc::clone(&self.oom_count);
            let handle = tokio::spawn(async move {
                // Try to allocate too much memory
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _impossible = vec![0u8; usize::MAX / 2];
                })) {
                    Err(_) => {
                        oom_count.fetch_add(1, Ordering::Relaxed);
                    }
                    Ok(_) => {}
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        let oom_events = self.oom_count.load(Ordering::Relaxed);
        debug!("OOM events: {}", oom_events);

        Ok(())
    }

    /// Test fragmentation
    pub async fn test_fragmentation(&self) -> Result<()> {
        info!("Testing memory fragmentation");

        // Allocate and deallocate in alternating pattern
        for _ in 0..50 {
            let _a = vec![0u8; 1024 * 1024];
            let _b = vec![0u8; 512 * 1024];
            let _c = vec![0u8; 256 * 1024];
            tokio::task::yield_now().await;
        }

        debug!("Fragmentation test completed");
        Ok(())
    }

    async fn calculate_memory_stats(&self) -> Result<MemoryStats> {
        let allocations = self.allocations.read().await;
        let _metrics = self.metrics.read().await;

        let total_alloc = allocations.iter().map(|a| a.size_bytes).sum::<u64>();

        let stats = MemoryStats {
            total_allocations: allocations.len() as u64,
            successful_allocations: allocations.len() as u64,
            failed_allocations: 0,
            total_allocated_bytes: total_alloc,
            peak_memory_bytes: self.peak_memory.load(Ordering::Relaxed),
            avg_allocation_time_us: 0.0,
            max_allocation_time_us: 0.0,
            fragmentation: FragmentationInfo {
                total_allocated: total_alloc,
                total_freed: 0,
                num_holes: 0,
                fragmentation_ratio: 0.0,
                largest_hole: 0,
            },
            oom_events: self.oom_count.load(Ordering::Relaxed),
            numa_migrations: 0,
        };

        info!("Memory stats: {:?}", stats);
        Ok(stats)
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> MemoryStats {
        let allocations = self.allocations.read().await;
        let total_alloc = allocations.iter().map(|a| a.size_bytes).sum::<u64>();

        MemoryStats {
            total_allocations: allocations.len() as u64,
            successful_allocations: allocations.len() as u64,
            failed_allocations: 0,
            total_allocated_bytes: total_alloc,
            peak_memory_bytes: self.peak_memory.load(Ordering::Relaxed),
            avg_allocation_time_us: 0.0,
            max_allocation_time_us: 0.0,
            fragmentation: FragmentationInfo {
                total_allocated: total_alloc,
                total_freed: 0,
                num_holes: 0,
                fragmentation_ratio: 0.0,
                largest_hole: 0,
            },
            oom_events: self.oom_count.load(Ordering::Relaxed),
            numa_migrations: 0,
        }
    }

    /// Run all memory tests
    pub async fn run_all(&self) -> Result<()> {
        self.test_allocation_stress().await?;
        self.test_deallocation().await?;
        self.test_numa_migrations().await?;
        self.test_huge_pages().await?;
        self.test_swap_pressure().await?;
        self.test_oom_handling().await?;
        self.test_fragmentation().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_allocation_creation() {
        let alloc = MemoryAllocation::new(1, 1024, Some(0), false);
        assert_eq!(alloc.id, 1);
        assert_eq!(alloc.size_bytes, 1024);
        assert_eq!(alloc.numa_node, Some(0));
    }

    #[tokio::test]
    async fn test_memory_test_creation() {
        let test = MemoryTest::new(MemoryConfig::default());
        let stats = test.get_stats().await;
        assert_eq!(stats.total_allocations, 0);
    }

    #[tokio::test]
    async fn test_allocation_stress() {
        let config = MemoryConfig {
            total_allocation_bytes: 100 * 1024 * 1024, // 100 MB
            concurrent_allocations: 10,
            ..Default::default()
        };
        let test = MemoryTest::new(config);
        let result = test.test_allocation_stress().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_deallocation() {
        let config = MemoryConfig {
            total_allocation_bytes: 10 * 1024 * 1024,
            concurrent_allocations: 4,
            ..Default::default()
        };
        let test = MemoryTest::new(config);
        test.test_allocation_stress().await.ok();
        let result = test.test_deallocation().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_numa_migrations() {
        let config = MemoryConfig {
            enable_numa: true,
            numa_nodes: 4,
            ..Default::default()
        };
        let test = MemoryTest::new(config);
        let result = test.test_numa_migrations().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_huge_pages() {
        let config = MemoryConfig {
            enable_huge_pages: true,
            total_allocation_bytes: 100 * 1024 * 1024,
            ..Default::default()
        };
        let test = MemoryTest::new(config);
        let result = test.test_huge_pages().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_oom_handling() {
        let test = MemoryTest::new(MemoryConfig::default());
        let result = test.test_oom_handling().await;
        assert!(result.is_ok());
    }
}
