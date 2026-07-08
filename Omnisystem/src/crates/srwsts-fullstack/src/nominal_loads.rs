//! Nominal load tests: baseline throughput under normal workload

use crate::errors::{FullStackTestError, FullStackTestResult};
use crate::vault::{ComponentHealth, Vault};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::task::JoinHandle;

/// Nominal load test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NominalLoadConfig {
    /// Duration of test in seconds
    pub duration_secs: u64,
    /// Number of concurrent service requests
    pub concurrent_requests: u32,
    /// Number of concurrent application executions
    pub concurrent_apps: u32,
    /// Ratio of user applications to background tasks (0.0 to 1.0)
    pub foreground_ratio: f64,
}

impl Default for NominalLoadConfig {
    fn default() -> Self {
        Self {
            duration_secs: 30,
            concurrent_requests: 16,
            concurrent_apps: 8,
            foreground_ratio: 0.7,
        }
    }
}

/// Result of a single request or operation
#[derive(Debug, Clone)]
pub struct OperationResult {
    pub latency_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

/// Nominal load test metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NominalLoadMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_latency_ms: f64,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
    pub p99_latency_ms: u64,
    pub requests_per_sec: f64,
    pub error_rate_percent: f64,
}

impl NominalLoadMetrics {
    /// Calculate metrics from operation results
    pub fn from_results(results: &[OperationResult], duration_secs: f64) -> Self {
        let total = results.len() as u64;
        let successful = results.iter().filter(|r| r.success).count() as u64;
        let failed = total - successful;

        let latencies: Vec<u64> = results.iter().map(|r| r.latency_ms).collect();
        let sum: u64 = latencies.iter().sum();

        let mut sorted = latencies.clone();
        sorted.sort_unstable();

        let avg = if total > 0 {
            sum as f64 / total as f64
        } else {
            0.0
        };

        let min = sorted.first().copied().unwrap_or(0);
        let max = sorted.last().copied().unwrap_or(0);
        let p99_idx = (total as f64 * 0.99) as usize;
        let p99 = sorted.get(p99_idx).copied().unwrap_or(max);

        let error_rate = if total > 0 {
            (failed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let rps = if duration_secs > 0.0 {
            total as f64 / duration_secs
        } else {
            0.0
        };

        Self {
            total_requests: total,
            successful_requests: successful,
            failed_requests: failed,
            avg_latency_ms: avg,
            min_latency_ms: min,
            max_latency_ms: max,
            p99_latency_ms: p99,
            requests_per_sec: rps,
            error_rate_percent: error_rate,
        }
    }

    /// Check if baseline metrics are acceptable
    pub fn is_acceptable(&self) -> bool {
        self.error_rate_percent < 5.0 && self.p99_latency_ms < 10000
    }
}

/// Nominal load test runner
pub struct NominalLoadTest {
    config: NominalLoadConfig,
    vault: Arc<Vault>,
}

impl NominalLoadTest {
    /// Create new nominal load test
    pub fn new(vault: Arc<Vault>, config: NominalLoadConfig) -> Self {
        Self { config, vault }
    }

    /// Run baseline throughput test
    pub async fn test_baseline_throughput(&self) -> FullStackTestResult<NominalLoadMetrics> {
        let results = Arc::new(AtomicU64::new(0));
        let failures = Arc::new(AtomicU64::new(0));
        let start = Instant::now();

        let mut tasks: Vec<JoinHandle<_>> = Vec::new();

        // Spawn service request tasks
        for _ in 0..self.config.concurrent_requests {
            let vault = self.vault.clone();
            let results = results.clone();
            let failures = failures.clone();
            let duration = self.config.duration_secs;

            let task = tokio::spawn(async move {
                let start = Instant::now();
                while start.elapsed().as_secs() < duration {
                    let services = vault.all_services().await;
                    if !services.is_empty() {
                        results.fetch_add(1, Ordering::SeqCst);
                    } else {
                        failures.fetch_add(1, Ordering::SeqCst);
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
            });
            tasks.push(task);
        }

        // Wait for all tasks
        for task in tasks {
            let _ = task.await;
        }

        let elapsed = start.elapsed().as_secs_f64();
        let successful = results.load(Ordering::SeqCst);
        let failed = failures.load(Ordering::SeqCst);

        Ok(NominalLoadMetrics {
            total_requests: successful + failed,
            successful_requests: successful,
            failed_requests: failed,
            avg_latency_ms: (elapsed * 1000.0) / (successful as f64).max(1.0),
            min_latency_ms: 0,
            max_latency_ms: (elapsed * 1000.0) as u64,
            p99_latency_ms: (elapsed * 1000.0) as u64,
            requests_per_sec: successful as f64 / elapsed,
            error_rate_percent: if successful + failed > 0 {
                (failed as f64 / (successful + failed) as f64) * 100.0
            } else {
                0.0
            },
        })
    }

    /// Run mixed workload test (users + background tasks)
    pub async fn test_mixed_workload(&self) -> FullStackTestResult<NominalLoadMetrics> {
        let results = Arc::new(AtomicU64::new(0));
        let failures = Arc::new(AtomicU64::new(0));
        let start = Instant::now();

        let foreground_tasks = (self.config.concurrent_apps as f64 * self.config.foreground_ratio) as u32;
        let background_tasks = self.config.concurrent_apps - foreground_tasks;

        let mut tasks: Vec<JoinHandle<_>> = Vec::new();

        // Foreground user application tasks
        for _ in 0..foreground_tasks {
            let vault = self.vault.clone();
            let results = results.clone();
            let failures = failures.clone();
            let duration = self.config.duration_secs;

            let task = tokio::spawn(async move {
                let start = Instant::now();
                while start.elapsed().as_secs() < duration {
                    let apps = vault.all_applications().await;
                    for app in apps {
                        if app.health == ComponentHealth::Healthy {
                            results.fetch_add(1, Ordering::SeqCst);
                        } else {
                            failures.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                }
            });
            tasks.push(task);
        }

        // Background service tasks
        for _ in 0..background_tasks {
            let vault = self.vault.clone();
            let results = results.clone();
            let failures = failures.clone();
            let duration = self.config.duration_secs;

            let task = tokio::spawn(async move {
                let start = Instant::now();
                while start.elapsed().as_secs() < duration {
                    let services = vault.all_services().await;
                    for service in services {
                        if service.health == ComponentHealth::Healthy {
                            results.fetch_add(1, Ordering::SeqCst);
                        } else {
                            failures.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            });
            tasks.push(task);
        }

        // Service request tasks
        for _ in 0..self.config.concurrent_requests {
            let vault = self.vault.clone();
            let results = results.clone();
            let failures = failures.clone();
            let duration = self.config.duration_secs;

            let task = tokio::spawn(async move {
                let start = Instant::now();
                while start.elapsed().as_secs() < duration {
                    let services = vault.all_services().await;
                    if !services.is_empty() {
                        results.fetch_add(1, Ordering::SeqCst);
                    } else {
                        failures.fetch_add(1, Ordering::SeqCst);
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
                }
            });
            tasks.push(task);
        }

        // Wait for all tasks
        for task in tasks {
            let _ = task.await;
        }

        let elapsed = start.elapsed().as_secs_f64();
        let successful = results.load(Ordering::SeqCst);
        let failed = failures.load(Ordering::SeqCst);

        Ok(NominalLoadMetrics {
            total_requests: successful + failed,
            successful_requests: successful,
            failed_requests: failed,
            avg_latency_ms: if successful > 0 {
                (elapsed * 1000.0) / (successful as f64)
            } else {
                0.0
            },
            min_latency_ms: 0,
            max_latency_ms: (elapsed * 1000.0) as u64,
            p99_latency_ms: (elapsed * 1000.0) as u64,
            requests_per_sec: successful as f64 / elapsed,
            error_rate_percent: if successful + failed > 0 {
                (failed as f64 / (successful + failed) as f64) * 100.0
            } else {
                0.0
            },
        })
    }

    /// Verify all subsystems coordinate correctly
    pub async fn test_subsystem_coordination(&self) -> FullStackTestResult<()> {
        // Verify kernel is healthy
        let kernel = self.vault.kernel_state().await?;
        if kernel.health != ComponentHealth::Healthy {
            return Err(FullStackTestError::ExecutionError(
                "Kernel not healthy".to_string(),
            ));
        }

        // Verify all services are operational
        let services = self.vault.all_services().await;
        for service in services {
            if !service.health.is_operational() {
                return Err(FullStackTestError::ExecutionError(format!(
                    "Service {} not operational",
                    service.name
                )));
            }
        }

        // Verify all applications are operational
        let apps = self.vault.all_applications().await;
        for app in apps {
            if !app.health.is_operational() {
                return Err(FullStackTestError::ExecutionError(format!(
                    "Application {} not operational",
                    app.name
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap::FullStackBootstrap;

    #[test]
    fn test_nominal_load_config_defaults() {
        let config = NominalLoadConfig::default();
        assert_eq!(config.duration_secs, 30);
        assert!(config.foreground_ratio > 0.0 && config.foreground_ratio <= 1.0);
    }

    #[tokio::test]
    async fn test_baseline_throughput() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = NominalLoadConfig {
            duration_secs: 5,
            concurrent_requests: 4,
            concurrent_apps: 2,
            foreground_ratio: 0.7,
        };

        let test = NominalLoadTest::new(vault, config);
        let metrics = test.test_baseline_throughput().await.unwrap();

        assert!(metrics.total_requests > 0);
        assert!(metrics.successful_requests > 0);
    }

    #[tokio::test]
    async fn test_mixed_workload() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = NominalLoadConfig {
            duration_secs: 5,
            concurrent_requests: 4,
            concurrent_apps: 4,
            foreground_ratio: 0.7,
        };

        let test = NominalLoadTest::new(vault, config);
        let metrics = test.test_mixed_workload().await.unwrap();

        assert!(metrics.total_requests > 0);
        assert!(metrics.error_rate_percent < 100.0);
    }

    #[tokio::test]
    async fn test_subsystem_coordination() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = NominalLoadConfig::default();
        let test = NominalLoadTest::new(vault, config);

        test.test_subsystem_coordination().await.unwrap();
    }

    #[test]
    fn test_metrics_calculation() {
        let results = vec![
            OperationResult {
                latency_ms: 10,
                success: true,
                error: None,
            },
            OperationResult {
                latency_ms: 20,
                success: true,
                error: None,
            },
            OperationResult {
                latency_ms: 5,
                success: false,
                error: Some("timeout".to_string()),
            },
        ];

        let metrics = NominalLoadMetrics::from_results(&results, 1.0);
        assert_eq!(metrics.total_requests, 3);
        assert_eq!(metrics.successful_requests, 2);
        assert_eq!(metrics.failed_requests, 1);
    }
}
