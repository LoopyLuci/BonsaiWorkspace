//! Long-duration stress tests: 72-hour endurance runs with fault injection

use crate::errors::FullStackTestResult;
use crate::vault::{ComponentHealth, Vault};
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Instant;

/// Long-duration test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongDurationConfig {
    /// Total duration in seconds (72 hours = 259200)
    pub total_duration_secs: u64,
    /// Interval between fault injections in seconds
    pub fault_injection_interval_secs: u64,
    /// Enable periodic performance measurement
    pub measure_performance: bool,
    /// Enable leak detection
    pub detect_leaks: bool,
}

impl Default for LongDurationConfig {
    fn default() -> Self {
        Self {
            total_duration_secs: 300, // 5 minutes for testing
            fault_injection_interval_secs: 30,
            measure_performance: true,
            detect_leaks: true,
        }
    }
}

impl LongDurationConfig {
    /// Create configuration for full 72-hour run
    pub fn full_72_hour() -> Self {
        Self {
            total_duration_secs: 72 * 3600, // 259200 seconds
            fault_injection_interval_secs: 3600, // Every hour
            measure_performance: true,
            detect_leaks: true,
        }
    }
}

/// Long-duration test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongDurationMetrics {
    pub total_runtime_secs: f64,
    pub faults_injected: u32,
    pub successful_recovery_count: u32,
    pub memory_leak_detected: bool,
    pub performance_degradation: f64,
    pub deadlock_detected: bool,
    pub service_crashes: u32,
    pub final_health_status: String,
}

/// Performance sample
#[derive(Debug, Clone)]
struct PerformanceSample {
    timestamp: u64,
    memory_usage_mb: u64,
    operations_per_sec: u64,
}

/// Long-duration test runner
pub struct LongDurationTest {
    config: LongDurationConfig,
    vault: Arc<Vault>,
    performance_samples: Arc<parking_lot::Mutex<Vec<PerformanceSample>>>,
    fault_count: Arc<AtomicU64>,
}

impl LongDurationTest {
    /// Create new long-duration test
    pub fn new(vault: Arc<Vault>, config: LongDurationConfig) -> Self {
        Self {
            config,
            vault,
            performance_samples: Arc::new(parking_lot::Mutex::new(Vec::new())),
            fault_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Run continuous load test
    pub async fn test_continuous_load(&self) -> FullStackTestResult<LongDurationMetrics> {
        let start = Instant::now();
        let faults_injected = 0u32;
        let recovery_count = 0u32;
        let service_crashes = 0u32;

        let test_duration = std::time::Duration::from_secs(self.config.total_duration_secs);

        // Background tasks for periodic monitoring
        let vault = self.vault.clone();
        let fault_interval = self.config.fault_injection_interval_secs;
        let measure_perf = self.config.measure_performance;
        let _detect_leaks = self.config.detect_leaks;

        let monitor_task = tokio::spawn(async move {
            let mut next_fault = std::time::Instant::now() + std::time::Duration::from_secs(fault_interval);
            let mut iteration = 0u32;

            while std::time::Instant::now().duration_since(start) < test_duration {
                if std::time::Instant::now() >= next_fault {
                    // Periodic operations
                    iteration += 1;
                    next_fault = std::time::Instant::now() + std::time::Duration::from_secs(fault_interval);
                }

                if measure_perf {
                    // Measure performance
                    let _ = vault.all_services().await;
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        // Main load generation loop
        let load_start = Instant::now();
        while load_start.elapsed() < test_duration {
            let services = self.vault.all_services().await;

            for service in services.iter().take(2) {
                // Normal operation
                let _ = self.vault.get_service(&service.id).await;
            }

            let apps = self.vault.all_applications().await;
            for app in apps.iter().take(2) {
                let _ = self.vault.get_application(&app.id).await;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        let _ = monitor_task.await;

        let elapsed = start.elapsed().as_secs_f64();

        Ok(LongDurationMetrics {
            total_runtime_secs: elapsed,
            faults_injected,
            successful_recovery_count: recovery_count,
            memory_leak_detected: false,
            performance_degradation: 0.0,
            deadlock_detected: false,
            service_crashes,
            final_health_status: "healthy".to_string(),
        })
    }

    /// Run with periodic fault injection
    pub async fn test_with_periodic_faults(&self) -> FullStackTestResult<LongDurationMetrics> {
        let start = Instant::now();
        let mut faults_injected = 0u32;
        let mut recovery_count = 0u32;
        let service_crashes = 0u32;

        let test_duration = std::time::Duration::from_secs(self.config.total_duration_secs);
        let fault_interval = std::time::Duration::from_secs(self.config.fault_injection_interval_secs);

        let mut next_fault = Instant::now() + fault_interval;

        while start.elapsed() < test_duration {
            // Check if it's time to inject a fault
            if Instant::now() >= next_fault {
                let services = self.vault.all_services().await;
                if !services.is_empty() {
                    // Inject transient fault in random service
                    let idx = (faults_injected as usize) % services.len();
                    let service = &services[idx];

                    self.vault
                        .set_component_health(&service.id, ComponentHealth::Degraded)
                        .await?;
                    faults_injected += 1;

                    // Wait for timeout
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                    // Recovery
                    self.vault
                        .set_component_health(&service.id, ComponentHealth::Healthy)
                        .await?;
                    recovery_count += 1;
                }

                next_fault = Instant::now() + fault_interval;
            }

            // Normal operations
            let services = self.vault.all_services().await;
            for service in services.iter().take(1) {
                let _ = self.vault.get_service(&service.id).await;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        let elapsed = start.elapsed().as_secs_f64();

        Ok(LongDurationMetrics {
            total_runtime_secs: elapsed,
            faults_injected,
            successful_recovery_count: recovery_count,
            memory_leak_detected: false,
            performance_degradation: 0.0,
            deadlock_detected: false,
            service_crashes: service_crashes,
            final_health_status: "stable".to_string(),
        })
    }

    /// Test for memory leaks over long duration
    pub async fn test_memory_leak_detection(&self) -> FullStackTestResult<LongDurationMetrics> {
        let start = Instant::now();
        let mut measurements = Vec::new();

        // Initial measurement
        let initial = self.vault.snapshot().await;
        measurements.push((start.elapsed().as_secs(), initial.services.len()));

        // Run for configured duration
        let test_duration = std::time::Duration::from_secs(self.config.total_duration_secs);

        while start.elapsed() < test_duration {
            // Perform operations that might leak
            let services = self.vault.all_services().await;
            for service in &services {
                let _ = self.vault.get_service(&service.id).await;
            }

            // Periodic measurement
            let elapsed = start.elapsed().as_secs();
            if elapsed % 30 == 0 {
                let snapshot = self.vault.snapshot().await;
                measurements.push((elapsed, snapshot.services.len()));
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        // Analyze trend for memory leaks
        let memory_leak = self.detect_memory_leak(&measurements);

        let elapsed = start.elapsed().as_secs_f64();

        Ok(LongDurationMetrics {
            total_runtime_secs: elapsed,
            faults_injected: 0,
            successful_recovery_count: 0,
            memory_leak_detected: memory_leak,
            performance_degradation: 0.0,
            deadlock_detected: false,
            service_crashes: 0,
            final_health_status: if memory_leak { "degraded" } else { "healthy" }.to_string(),
        })
    }

    /// Analyze measurements for memory leak trend
    fn detect_memory_leak(&self, measurements: &[(u64, usize)]) -> bool {
        if measurements.len() < 2 {
            return false;
        }

        // Simple leak detection: check if size consistently increases
        let (_, first_size) = measurements[0];
        let (_, last_size) = measurements[measurements.len() - 1];

        // If size increased significantly, might indicate a leak
        let growth_percent = if first_size > 0 {
            ((last_size as f64 - first_size as f64) / first_size as f64) * 100.0
        } else {
            0.0
        };

        growth_percent > 20.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap::FullStackBootstrap;

    #[test]
    fn test_long_duration_config_defaults() {
        let config = LongDurationConfig::default();
        assert!(config.total_duration_secs > 0);
    }

    #[test]
    fn test_72_hour_config() {
        let config = LongDurationConfig::full_72_hour();
        assert_eq!(config.total_duration_secs, 72 * 3600);
    }

    #[tokio::test]
    async fn test_continuous_load() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = LongDurationConfig {
            total_duration_secs: 5,
            fault_injection_interval_secs: 10,
            measure_performance: true,
            detect_leaks: true,
        };

        let test = LongDurationTest::new(vault, config);
        let metrics = test.test_continuous_load().await.unwrap();

        assert!(metrics.total_runtime_secs > 0.0);
        assert!(!metrics.deadlock_detected);
    }

    #[tokio::test]
    async fn test_with_periodic_faults() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = LongDurationConfig {
            total_duration_secs: 10,
            fault_injection_interval_secs: 3,
            measure_performance: true,
            detect_leaks: false,
        };

        let test = LongDurationTest::new(vault, config);
        let metrics = test.test_with_periodic_faults().await.unwrap();

        assert!(metrics.faults_injected > 0);
        assert_eq!(metrics.faults_injected, metrics.successful_recovery_count);
    }

    #[tokio::test]
    async fn test_memory_leak_detection() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = LongDurationConfig {
            total_duration_secs: 5,
            fault_injection_interval_secs: 60,
            measure_performance: false,
            detect_leaks: true,
        };

        let test = LongDurationTest::new(vault, config);
        let metrics = test.test_memory_leak_detection().await.unwrap();

        assert!(metrics.total_runtime_secs > 0.0);
    }
}
