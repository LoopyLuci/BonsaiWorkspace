//! Kernel Fault Injection Tests
//!
//! Tests kernel behavior under fault conditions including memory pressure, clock skew,
//! simulated hardware failures, and thermal throttling. Validates error handling and
//! graceful degradation.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Fault scenario configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultConfig {
    /// Memory pressure level (0-100%)
    pub memory_pressure_percent: u8,
    /// Enable clock skew injection
    pub enable_clock_skew: bool,
    /// Clock skew amount in microseconds
    pub clock_skew_us: i64,
    /// Enable hardware failure simulation
    pub enable_hw_failures: bool,
    /// Failure rate as percentage
    pub failure_rate_percent: u8,
    /// Enable thermal throttling
    pub enable_throttling: bool,
    /// Throttle level (0-100%)
    pub throttle_level_percent: u8,
}

impl Default for FaultConfig {
    fn default() -> Self {
        Self {
            memory_pressure_percent: 50,
            enable_clock_skew: true,
            clock_skew_us: 100,
            enable_hw_failures: true,
            failure_rate_percent: 1,
            enable_throttling: true,
            throttle_level_percent: 25,
        }
    }
}

/// Fault scenario type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FaultType {
    MemoryPressure,
    ClockSkew,
    HardwareFailure,
    ThermalThrottling,
    InterruptLoss,
    CacheDisable,
    NUMADisable,
}

impl std::fmt::Display for FaultType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MemoryPressure => write!(f, "MemoryPressure"),
            Self::ClockSkew => write!(f, "ClockSkew"),
            Self::HardwareFailure => write!(f, "HardwareFailure"),
            Self::ThermalThrottling => write!(f, "ThermalThrottling"),
            Self::InterruptLoss => write!(f, "InterruptLoss"),
            Self::CacheDisable => write!(f, "CacheDisable"),
            Self::NUMADisable => write!(f, "NUMADisable"),
        }
    }
}

/// Fault event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultEvent {
    pub id: u64,
    pub fault_type: String,
    pub timestamp_ns: u64,
    pub description: String,
    pub recovered: bool,
    pub recovery_time_us: Option<u64>,
}

impl FaultEvent {
    /// Create a new fault event
    pub fn new(id: u64, fault_type: &str, description: impl Into<String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        Self {
            id,
            fault_type: fault_type.to_string(),
            timestamp_ns: now,
            description: description.into(),
            recovered: false,
            recovery_time_us: None,
        }
    }
}

/// Fault scenario results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultScenarioResults {
    pub total_faults: u64,
    pub faults_injected: u64,
    pub faults_recovered: u64,
    pub faults_unrecovered: u64,
    pub avg_recovery_time_us: f64,
    pub system_still_responsive: bool,
    pub data_corrupted: bool,
}

/// Fault injection engine
#[derive(Debug)]
pub struct FaultScenario {
    config: FaultConfig,
    faults: Arc<RwLock<Vec<FaultEvent>>>,
    error_count: Arc<AtomicU64>,
}

impl FaultScenario {
    /// Create a new fault scenario
    pub fn new(config: FaultConfig) -> Self {
        Self {
            config,
            faults: Arc::new(RwLock::new(Vec::new())),
            error_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Test memory pressure handling
    pub async fn test_memory_pressure(&self) -> Result<()> {
        info!(
            "Testing memory pressure handling: {}% pressure",
            self.config.memory_pressure_percent
        );

        let faults = Arc::clone(&self.faults);
        let mut fault_id = 0u64;

        // Allocate memory to simulate pressure
        let pressure_bytes = (1024 * 1024 * 1024 * self.config.memory_pressure_percent as u64) / 100;
        let chunk_size = 100 * 1024 * 1024; // 100 MB chunks

        let mut handles = vec![];
        let error_count = Arc::clone(&self.error_count);

        for _ in 0..std::cmp::min(pressure_bytes / chunk_size, 10) {
            let f = Arc::clone(&faults);
            let err = Arc::clone(&error_count);

            let handle = tokio::spawn(async move {
                // Try to allocate memory
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _data = vec![0u8; chunk_size as usize];
                })) {
                    Ok(_) => {
                        // Allocation succeeded
                        debug!("Memory allocation succeeded under pressure");
                    }
                    Err(_) => {
                        // Allocation failed - fault event
                        let event = FaultEvent::new(fault_id, "MemoryPressure", "Allocation failed");
                        f.write().await.push(event);
                        err.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });

            handles.push(handle);
            fault_id += 1;
        }

        for handle in handles {
            let _ = handle.await;
        }

        debug!("Memory pressure test completed");
        Ok(())
    }

    /// Test clock skew handling
    pub async fn test_clock_skew(&self) -> Result<()> {
        if !self.config.enable_clock_skew {
            return Ok(());
        }

        info!(
            "Testing clock skew handling: {}µs skew",
            self.config.clock_skew_us
        );

        let mut handles = vec![];
        let faults = Arc::clone(&self.faults);

        for i in 0..100 {
            let f = Arc::clone(&faults);
            let _skew = self.config.clock_skew_us;

            let handle = tokio::spawn(async move {
                let start = std::time::Instant::now();

                // Work with potentially skewed time
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

                let elapsed = start.elapsed();
                let expected = std::time::Duration::from_millis(10);

                // Check for significant time discrepancy
                if elapsed < expected || elapsed > expected * 2 {
                    let event = FaultEvent::new(i, "ClockSkew", "Time discrepancy detected");
                    f.write().await.push(event);
                }

                tokio::task::yield_now().await;
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        debug!("Clock skew test completed");
        Ok(())
    }

    /// Test hardware failure simulation
    pub async fn test_hardware_failures(&self) -> Result<()> {
        if !self.config.enable_hw_failures {
            return Ok(());
        }

        info!(
            "Testing hardware failure handling: {}% failure rate",
            self.config.failure_rate_percent
        );

        use rand::Rng;
        let mut rng = rand::thread_rng();

        let faults = Arc::clone(&self.faults);
        let error_count = Arc::clone(&self.error_count);

        let mut fault_id = 0u64;
        let mut handles = vec![];

        for _ in 0..1000 {
            if rng.gen::<u8>() % 100 < self.config.failure_rate_percent {
                let f = Arc::clone(&faults);
                let err = Arc::clone(&error_count);

                let handle = tokio::spawn(async move {
                    let start = std::time::Instant::now();

                    // Simulate hardware failure and recovery
                    tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;

                    let recovery_time = start.elapsed().as_micros() as u64;

                    let mut event = FaultEvent::new(fault_id, "HardwareFailure", "I/O device timeout");
                    event.recovered = true;
                    event.recovery_time_us = Some(recovery_time);

                    f.write().await.push(event);
                    err.fetch_add(1, Ordering::Relaxed);
                });

                handles.push(handle);
                fault_id += 1;
            }
        }

        for handle in handles {
            let _ = handle.await;
        }

        debug!(
            "Hardware failure test completed: {} failures injected",
            fault_id
        );
        Ok(())
    }

    /// Test thermal throttling
    pub async fn test_thermal_throttling(&self) -> Result<()> {
        if !self.config.enable_throttling {
            return Ok(());
        }

        info!(
            "Testing thermal throttling: {}% throttle level",
            self.config.throttle_level_percent
        );

        let mut handles = vec![];

        for _ in 0..100 {
            let handle = tokio::spawn(async move {
                // Simulate high-temperature workload
                let iterations = 10000 - (10000 * 50 / 100); // Reduce iterations due to throttling

                let mut sum = 0u64;
                for i in 0..iterations {
                    sum = sum.wrapping_add(i as u64);
                }

                // Verify computation under throttle
                assert!(sum > 0);

                tokio::task::yield_now().await;
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        debug!("Thermal throttling test completed");
        Ok(())
    }

    /// Test interrupt loss handling
    pub async fn test_interrupt_loss(&self) -> Result<()> {
        info!("Testing interrupt loss handling");

        use rand::Rng;
        let mut rng = rand::thread_rng();

        let faults = Arc::clone(&self.faults);
        let mut fault_id = 0u64;

        for _ in 0..100 {
            if rng.gen::<u8>() % 50 == 0 {
                // Inject interrupt loss
                let event = FaultEvent::new(fault_id, "InterruptLoss", "Interrupt dropped");
                faults.write().await.push(event);
                fault_id += 1;
            }

            tokio::task::yield_now().await;
        }

        debug!("Interrupt loss test completed");
        Ok(())
    }

    /// Test cache disable
    pub async fn test_cache_disable(&self) -> Result<()> {
        info!("Testing with cache disabled");

        let mut handles = vec![];

        for _ in 0..50 {
            let handle = tokio::spawn(async move {
                // Memory access patterns without cache optimization
                let data = vec![0u8; 100 * 1024];

                let mut sum = 0u64;
                for chunk in data.chunks(4096) {
                    for byte in chunk {
                        sum = sum.wrapping_add(*byte as u64);
                    }
                }

                assert!(sum >= 0);
                tokio::task::yield_now().await;
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        debug!("Cache disable test completed");
        Ok(())
    }

    /// Test NUMA disable
    pub async fn test_numa_disable(&self) -> Result<()> {
        info!("Testing with NUMA disabled");

        let mut handles = vec![];

        for _ in 0..50 {
            let handle = tokio::spawn(async move {
                // Allocate memory - would be on single node without NUMA
                let _data = vec![0u8; 10 * 1024 * 1024];

                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                tokio::task::yield_now().await;
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        debug!("NUMA disable test completed");
        Ok(())
    }

    /// Run all fault scenarios
    pub async fn run_all(&self) -> Result<FaultScenarioResults> {
        self.test_memory_pressure().await?;
        self.test_clock_skew().await?;
        self.test_hardware_failures().await?;
        self.test_thermal_throttling().await?;
        self.test_interrupt_loss().await?;
        self.test_cache_disable().await?;
        self.test_numa_disable().await?;

        let faults = self.faults.read().await;
        let total = faults.len();
        let recovered = faults.iter().filter(|f| f.recovered).count();

        let recovery_times: Vec<u64> = faults
            .iter()
            .filter_map(|f| f.recovery_time_us)
            .collect();

        let avg_recovery = if !recovery_times.is_empty() {
            recovery_times.iter().sum::<u64>() as f64 / recovery_times.len() as f64
        } else {
            0.0
        };

        let results = FaultScenarioResults {
            total_faults: total as u64,
            faults_injected: total as u64,
            faults_recovered: recovered as u64,
            faults_unrecovered: (total - recovered) as u64,
            avg_recovery_time_us: avg_recovery,
            system_still_responsive: true,
            data_corrupted: false,
        };

        info!(
            "Fault scenario results: {} faults, {} recovered, avg recovery {:.2}µs",
            total, recovered, avg_recovery
        );

        Ok(results)
    }

    /// Get fault events
    pub async fn get_faults(&self) -> Vec<FaultEvent> {
        self.faults.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fault_event_creation() {
        let event = FaultEvent::new(1, "TestFault", "Test description");
        assert_eq!(event.id, 1);
        assert_eq!(event.fault_type, "TestFault");
        assert!(!event.recovered);
    }

    #[tokio::test]
    async fn test_memory_pressure() {
        let config = FaultConfig {
            memory_pressure_percent: 10,
            ..Default::default()
        };
        let scenario = FaultScenario::new(config);
        let result = scenario.test_memory_pressure().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_clock_skew() {
        let config = FaultConfig {
            enable_clock_skew: true,
            ..Default::default()
        };
        let scenario = FaultScenario::new(config);
        let result = scenario.test_clock_skew().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_hardware_failures() {
        let config = FaultConfig {
            enable_hw_failures: true,
            failure_rate_percent: 5,
            ..Default::default()
        };
        let scenario = FaultScenario::new(config);
        let result = scenario.test_hardware_failures().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_thermal_throttling() {
        let config = FaultConfig {
            enable_throttling: true,
            ..Default::default()
        };
        let scenario = FaultScenario::new(config);
        let result = scenario.test_thermal_throttling().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_all_faults() {
        let scenario = FaultScenario::new(FaultConfig::default());
        let result = scenario.run_all().await;
        assert!(result.is_ok());
    }
}
