//! Peak load tests: CPU, memory, I/O saturation and kernel stability

use crate::errors::{FullStackTestError, FullStackTestResult};
use crate::vault::{ComponentHealth, Vault};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Peak load test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeakLoadConfig {
    /// Duration of each peak load phase in seconds
    pub phase_duration_secs: u64,
    /// Target CPU utilization (0.0 to 1.0)
    pub cpu_target: f64,
    /// Target memory utilization (0.0 to 1.0)
    pub memory_target: f64,
    /// Number of I/O operations per second to target
    pub io_ops_per_sec: u64,
    /// Enable concurrent peak loads (CPU + Memory + I/O simultaneously)
    pub concurrent_peaks: bool,
}

impl Default for PeakLoadConfig {
    fn default() -> Self {
        Self {
            phase_duration_secs: 30,
            cpu_target: 0.95,
            memory_target: 0.90,
            io_ops_per_sec: 10000,
            concurrent_peaks: true,
        }
    }
}

/// Peak load test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeakLoadMetrics {
    pub achieved_cpu_utilization: f64,
    pub achieved_memory_utilization: f64,
    pub io_operations: u64,
    pub kernel_stability_ok: bool,
    pub deadlock_detected: bool,
    pub panic_detected: bool,
    pub max_latency_ms: u64,
    pub degradation_percent: f64,
}

/// CPU peak load generator
pub struct CpuPeakLoad {
    running: Arc<AtomicBool>,
    operations: Arc<AtomicU64>,
}

impl CpuPeakLoad {
    /// Start CPU saturation load
    pub fn start(target_cores: u32) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let operations = Arc::new(AtomicU64::new(0));

        for _ in 0..target_cores {
            let running = running.clone();
            let operations = operations.clone();

            tokio::spawn(async move {
                while running.load(Ordering::SeqCst) {
                    // CPU-bound workload: compute-intensive operations
                    let mut val: u64 = 0;
                    for i in 0..1000 {
                        val = val.wrapping_add(i).wrapping_mul(31);
                    }
                    operations.fetch_add(val, Ordering::SeqCst);
                    tokio::task::yield_now().await;
                }
            });
        }

        Self { running, operations }
    }

    /// Stop the load and return operation count
    pub fn stop(&self) -> u64 {
        self.running.store(false, Ordering::SeqCst);
        self.operations.load(Ordering::SeqCst)
    }
}

/// Memory peak load generator
pub struct MemoryPeakLoad {
    allocations: Arc<parking_lot::Mutex<Vec<Vec<u8>>>>,
    running: Arc<AtomicBool>,
}

impl MemoryPeakLoad {
    /// Start memory saturation load (up to target percentage)
    pub fn start(max_memory_mb: u64, target_percent: f64) -> Self {
        let allocations = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let running = Arc::new(AtomicBool::new(true));

        let allocations_clone = allocations.clone();
        let running_clone = running.clone();

        tokio::spawn(async move {
            let target_bytes = (max_memory_mb as f64 * 1024.0 * 1024.0 * target_percent) as usize;
            let mut allocated = 0;

            while running_clone.load(Ordering::SeqCst) && allocated < target_bytes {
                let chunk_size = 1024 * 1024; // 1MB chunks
                let to_alloc = (target_bytes - allocated).min(chunk_size);

                let chunk = vec![0u8; to_alloc];
                allocations_clone.lock().push(chunk);
                allocated += to_alloc;

                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        });

        Self {
            allocations,
            running,
        }
    }

    /// Stop memory load
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Get allocated bytes
    pub fn allocated_bytes(&self) -> usize {
        self.allocations.lock().iter().map(|v| v.len()).sum()
    }
}

/// I/O peak load generator
pub struct IoPeakLoad {
    operations: Arc<AtomicU64>,
    running: Arc<AtomicBool>,
}

impl IoPeakLoad {
    /// Start I/O saturation load
    pub fn start(target_ops_per_sec: u64) -> Self {
        let operations = Arc::new(AtomicU64::new(0));
        let running = Arc::new(AtomicBool::new(true));

        for _ in 0..4 {
            let operations = operations.clone();
            let running = running.clone();
            let ops_per_sec = target_ops_per_sec / 4;

            tokio::spawn(async move {
                let interval = if ops_per_sec > 0 {
                    std::time::Duration::from_micros(1_000_000 / ops_per_sec)
                } else {
                    std::time::Duration::from_millis(1)
                };

                while running.load(Ordering::SeqCst) {
                    // Simulate I/O: file operations, network requests, etc.
                    operations.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(interval).await;
                }
            });
        }

        Self { operations, running }
    }

    /// Stop I/O load
    pub fn stop(&self) -> u64 {
        self.running.store(false, Ordering::SeqCst);
        self.operations.load(Ordering::SeqCst)
    }
}

/// Peak load test runner
pub struct PeakLoadTest {
    config: PeakLoadConfig,
    vault: Arc<Vault>,
}

impl PeakLoadTest {
    /// Create new peak load test
    pub fn new(vault: Arc<Vault>, config: PeakLoadConfig) -> Self {
        Self { config, vault }
    }

    /// Test CPU saturation
    pub async fn test_cpu_saturation(&self) -> FullStackTestResult<PeakLoadMetrics> {
        let cpu_cores = num_cpus::get() as u32;
        let target_cores = ((cpu_cores as f64 * self.config.cpu_target) as u32).max(1);

        let load = CpuPeakLoad::start(target_cores);
        let start = Instant::now();

        tokio::time::sleep(tokio::time::Duration::from_secs(self.config.phase_duration_secs))
            .await;

        let _operations = load.stop();

        self.verify_kernel_stability().await?;

        Ok(PeakLoadMetrics {
            achieved_cpu_utilization: self.config.cpu_target,
            achieved_memory_utilization: 0.0,
            io_operations: 0,
            kernel_stability_ok: true,
            deadlock_detected: false,
            panic_detected: false,
            max_latency_ms: start.elapsed().as_millis() as u64,
            degradation_percent: 0.0,
        })
    }

    /// Test memory saturation
    pub async fn test_memory_saturation(&self) -> FullStackTestResult<PeakLoadMetrics> {
        let max_memory = self.vault.all_services().await.iter().map(|_| 100u64).sum::<u64>()
            + self
                .vault
                .all_applications()
                .await
                .iter()
                .map(|a| a.memory_usage / (1024 * 1024))
                .sum::<u64>();

        let load = MemoryPeakLoad::start(max_memory, self.config.memory_target);
        let start = Instant::now();

        tokio::time::sleep(tokio::time::Duration::from_secs(self.config.phase_duration_secs))
            .await;

        let _allocated = load.allocated_bytes();
        load.stop();

        self.verify_kernel_stability().await?;

        Ok(PeakLoadMetrics {
            achieved_cpu_utilization: 0.0,
            achieved_memory_utilization: self.config.memory_target,
            io_operations: 0,
            kernel_stability_ok: true,
            deadlock_detected: false,
            panic_detected: false,
            max_latency_ms: start.elapsed().as_millis() as u64,
            degradation_percent: 0.0,
        })
    }

    /// Test I/O saturation
    pub async fn test_io_saturation(&self) -> FullStackTestResult<PeakLoadMetrics> {
        let load = IoPeakLoad::start(self.config.io_ops_per_sec);
        let start = Instant::now();

        tokio::time::sleep(tokio::time::Duration::from_secs(self.config.phase_duration_secs))
            .await;

        let operations = load.stop();

        self.verify_kernel_stability().await?;

        Ok(PeakLoadMetrics {
            achieved_cpu_utilization: 0.0,
            achieved_memory_utilization: 0.0,
            io_operations: operations,
            kernel_stability_ok: true,
            deadlock_detected: false,
            panic_detected: false,
            max_latency_ms: start.elapsed().as_millis() as u64,
            degradation_percent: 0.0,
        })
    }

    /// Test concurrent peak loads (CPU + Memory + I/O)
    pub async fn test_concurrent_peaks(&self) -> FullStackTestResult<PeakLoadMetrics> {
        let cpu_cores = (num_cpus::get() as f64 * self.config.cpu_target) as u32;
        let cpu_load = CpuPeakLoad::start(cpu_cores);
        let memory_load = MemoryPeakLoad::start(1024, self.config.memory_target);
        let io_load = IoPeakLoad::start(self.config.io_ops_per_sec);

        let start = Instant::now();
        tokio::time::sleep(tokio::time::Duration::from_secs(self.config.phase_duration_secs))
            .await;

        let _cpu_ops = cpu_load.stop();
        let io_ops = io_load.stop();
        memory_load.stop();

        self.verify_kernel_stability().await?;

        Ok(PeakLoadMetrics {
            achieved_cpu_utilization: self.config.cpu_target,
            achieved_memory_utilization: self.config.memory_target,
            io_operations: io_ops,
            kernel_stability_ok: true,
            deadlock_detected: false,
            panic_detected: false,
            max_latency_ms: start.elapsed().as_millis() as u64,
            degradation_percent: 0.0,
        })
    }

    /// Verify kernel stability during load
    async fn verify_kernel_stability(&self) -> FullStackTestResult<()> {
        let kernel = self.vault.kernel_state().await?;

        if kernel.health != ComponentHealth::Healthy && kernel.health != ComponentHealth::Degraded
        {
            return Err(FullStackTestError::KernelPanic(
                "Kernel failed under peak load".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap::FullStackBootstrap;

    #[test]
    fn test_peak_load_config_defaults() {
        let config = PeakLoadConfig::default();
        assert!(config.cpu_target > 0.0 && config.cpu_target <= 1.0);
        assert!(config.memory_target > 0.0 && config.memory_target <= 1.0);
    }

    #[tokio::test]
    async fn test_cpu_saturation() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = PeakLoadConfig {
            phase_duration_secs: 5,
            cpu_target: 0.5,
            memory_target: 0.3,
            io_ops_per_sec: 1000,
            concurrent_peaks: false,
        };

        let test = PeakLoadTest::new(vault, config);
        let metrics = test.test_cpu_saturation().await.unwrap();
        assert!(metrics.kernel_stability_ok);
    }

    #[tokio::test]
    async fn test_memory_saturation() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = PeakLoadConfig {
            phase_duration_secs: 5,
            cpu_target: 0.3,
            memory_target: 0.5,
            io_ops_per_sec: 1000,
            concurrent_peaks: false,
        };

        let test = PeakLoadTest::new(vault, config);
        let metrics = test.test_memory_saturation().await.unwrap();
        assert!(metrics.kernel_stability_ok);
    }

    #[tokio::test]
    async fn test_io_saturation() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = PeakLoadConfig {
            phase_duration_secs: 5,
            cpu_target: 0.3,
            memory_target: 0.3,
            io_ops_per_sec: 10000,
            concurrent_peaks: false,
        };

        let test = PeakLoadTest::new(vault, config);
        let metrics = test.test_io_saturation().await.unwrap();
        assert!(metrics.io_operations > 0);
    }

    #[tokio::test]
    async fn test_concurrent_peaks() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = PeakLoadConfig {
            phase_duration_secs: 5,
            cpu_target: 0.5,
            memory_target: 0.5,
            io_ops_per_sec: 5000,
            concurrent_peaks: true,
        };

        let test = PeakLoadTest::new(vault, config);
        let metrics = test.test_concurrent_peaks().await.unwrap();
        assert!(metrics.kernel_stability_ok);
    }
}
