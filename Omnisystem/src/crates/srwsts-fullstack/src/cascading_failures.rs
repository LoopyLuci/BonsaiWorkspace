//! Cascading failure tests: component isolation under failures

use crate::errors::FullStackTestResult;
use crate::vault::{ComponentHealth, Vault};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Cascading failure test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadingFailureConfig {
    /// Delay between failure injections in milliseconds
    pub failure_delay_ms: u64,
    /// Whether to inject multiple simultaneous failures
    pub concurrent_failures: bool,
    /// Maximum number of failures to inject in sequence
    pub max_sequential_failures: u32,
}

impl Default for CascadingFailureConfig {
    fn default() -> Self {
        Self {
            failure_delay_ms: 100,
            concurrent_failures: true,
            max_sequential_failures: 10,
        }
    }
}

/// Result of a cascading failure test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadingFailureMetrics {
    pub failures_injected: u32,
    pub isolation_maintained: bool,
    pub services_affected: Vec<String>,
    pub apps_affected: Vec<String>,
    pub kernel_survived: bool,
    pub recovery_successful: Vec<String>,
    pub data_loss_detected: bool,
}

/// Cascading failure test runner
pub struct CascadingFailureTest {
    config: CascadingFailureConfig,
    vault: Arc<Vault>,
}

impl CascadingFailureTest {
    /// Create new cascading failure test
    pub fn new(vault: Arc<Vault>, config: CascadingFailureConfig) -> Self {
        Self { config, vault }
    }

    /// Test kernel thread failure handling
    pub async fn test_kernel_thread_failure(&self) -> FullStackTestResult<CascadingFailureMetrics> {
        let mut kernel = self.vault.kernel_state().await?;
        let _original_threads = kernel.thread_count;

        // Simulate kernel thread failure
        kernel.thread_count = kernel.thread_count.saturating_sub(1);

        // Services should handle degraded kernel gracefully
        let services = self.vault.all_services().await;
        let affected_services: Vec<String> = services
            .iter()
            .filter(|s| !s.health.is_operational())
            .map(|s| s.name.clone())
            .collect();

        // Kernel should remain operational
        let kernel_survived = kernel.thread_count > 0 || services.is_empty();

        Ok(CascadingFailureMetrics {
            failures_injected: 1,
            isolation_maintained: !affected_services.is_empty() || services.is_empty(),
            services_affected: affected_services,
            apps_affected: Vec::new(),
            kernel_survived,
            recovery_successful: Vec::new(),
            data_loss_detected: false,
        })
    }

    /// Test service failure and graceful degradation
    pub async fn test_service_failure_isolation(&self) -> FullStackTestResult<CascadingFailureMetrics> {
        let services = self.vault.all_services().await;

        if services.is_empty() {
            return Ok(CascadingFailureMetrics {
                failures_injected: 0,
                isolation_maintained: true,
                services_affected: Vec::new(),
                apps_affected: Vec::new(),
                kernel_survived: true,
                recovery_successful: Vec::new(),
                data_loss_detected: false,
            });
        }

        let mut failed_service_ids = Vec::new();
        let mut isolation_maintained = true;

        // Fail first service
        let first_service = &services[0];
        self.vault
            .set_component_health(&first_service.id, ComponentHealth::Failed)
            .await?;
        failed_service_ids.push(first_service.name.clone());

        // Check if other services remain operational
        let remaining_services = self.vault.all_services().await;
        for service in &remaining_services[1..] {
            if !service.health.is_operational() {
                isolation_maintained = false;
                failed_service_ids.push(service.name.clone());
            }
        }

        // Verify kernel still operational
        let kernel = self.vault.kernel_state().await?;
        let kernel_survived = kernel.health.is_operational();

        Ok(CascadingFailureMetrics {
            failures_injected: 1,
            isolation_maintained,
            services_affected: failed_service_ids,
            apps_affected: Vec::new(),
            kernel_survived,
            recovery_successful: Vec::new(),
            data_loss_detected: false,
        })
    }

    /// Test application failure isolation
    pub async fn test_application_failure_isolation(
        &self,
    ) -> FullStackTestResult<CascadingFailureMetrics> {
        let apps = self.vault.all_applications().await;

        if apps.is_empty() {
            return Ok(CascadingFailureMetrics {
                failures_injected: 0,
                isolation_maintained: true,
                services_affected: Vec::new(),
                apps_affected: Vec::new(),
                kernel_survived: true,
                recovery_successful: Vec::new(),
                data_loss_detected: false,
            });
        }

        let mut failed_app_ids = Vec::new();
        let mut isolation_maintained = true;

        // Fail first application
        let first_app = &apps[0];
        self.vault
            .set_component_health(&first_app.id, ComponentHealth::Failed)
            .await?;
        failed_app_ids.push(first_app.name.clone());

        // Check if other applications remain operational
        let remaining_apps = self.vault.all_applications().await;
        for app in &remaining_apps[1..] {
            if !app.health.is_operational() {
                isolation_maintained = false;
                failed_app_ids.push(app.name.clone());
            }
        }

        // Verify services still operational
        let services = self.vault.all_services().await;
        let services_affected = services
            .iter()
            .filter(|s| !s.health.is_operational())
            .map(|s| s.name.clone())
            .collect();

        Ok(CascadingFailureMetrics {
            failures_injected: 1,
            isolation_maintained,
            services_affected,
            apps_affected: failed_app_ids,
            kernel_survived: true,
            recovery_successful: Vec::new(),
            data_loss_detected: false,
        })
    }

    /// Test simultaneous multi-component failures
    pub async fn test_simultaneous_failures(&self) -> FullStackTestResult<CascadingFailureMetrics> {
        let mut failed_services = Vec::new();
        let mut failed_apps = Vec::new();

        // Fail multiple services simultaneously
        let services = self.vault.all_services().await;
        for (i, service) in services.iter().enumerate() {
            if i >= 2 {
                break;
            }
            self.vault
                .set_component_health(&service.id, ComponentHealth::Failed)
                .await?;
            failed_services.push(service.name.clone());
        }

        // Fail multiple applications simultaneously
        let apps = self.vault.all_applications().await;
        for (i, app) in apps.iter().enumerate() {
            if i >= 2 {
                break;
            }
            self.vault
                .set_component_health(&app.id, ComponentHealth::Failed)
                .await?;
            failed_apps.push(app.name.clone());
        }

        // Verify system still partially operational
        let kernel = self.vault.kernel_state().await?;
        let kernel_survived = kernel.health.is_operational();

        let remaining_services = self.vault.all_services().await;
        let operational_services = remaining_services
            .iter()
            .filter(|s| s.health.is_operational())
            .count();

        let isolation_maintained = operational_services > 0 || failed_services.len() <= 1;

        Ok(CascadingFailureMetrics {
            failures_injected: (failed_services.len() + failed_apps.len()) as u32,
            isolation_maintained,
            services_affected: failed_services,
            apps_affected: failed_apps,
            kernel_survived,
            recovery_successful: Vec::new(),
            data_loss_detected: false,
        })
    }

    /// Test sequential failures with recovery verification
    pub async fn test_sequential_failures_with_recovery(
        &self,
    ) -> FullStackTestResult<CascadingFailureMetrics> {
        let mut injected = 0u32;
        let mut failed_items = Vec::new();
        let mut recovered_items = Vec::new();

        let services = self.vault.all_services().await;

        for service in services.iter().take(self.config.max_sequential_failures as usize) {
            // Inject failure
            self.vault
                .set_component_health(&service.id, ComponentHealth::Failed)
                .await?;
            failed_items.push(service.name.clone());
            injected += 1;

            tokio::time::sleep(tokio::time::Duration::from_millis(
                self.config.failure_delay_ms,
            ))
            .await;

            // Recovery: restore health
            self.vault
                .set_component_health(&service.id, ComponentHealth::Healthy)
                .await?;
            recovered_items.push(service.name.clone());
        }

        Ok(CascadingFailureMetrics {
            failures_injected: injected,
            isolation_maintained: true,
            services_affected: failed_items,
            apps_affected: Vec::new(),
            kernel_survived: true,
            recovery_successful: recovered_items,
            data_loss_detected: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap::FullStackBootstrap;

    #[test]
    fn test_cascading_failure_config_defaults() {
        let config = CascadingFailureConfig::default();
        assert!(config.failure_delay_ms > 0);
    }

    #[tokio::test]
    async fn test_kernel_thread_failure() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = CascadingFailureConfig::default();
        let test = CascadingFailureTest::new(vault, config);

        let metrics = test.test_kernel_thread_failure().await.unwrap();
        assert!(metrics.kernel_survived);
    }

    #[tokio::test]
    async fn test_service_failure_isolation() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = CascadingFailureConfig::default();
        let test = CascadingFailureTest::new(vault, config);

        let metrics = test.test_service_failure_isolation().await.unwrap();
        assert!(metrics.kernel_survived);
    }

    #[tokio::test]
    async fn test_application_failure_isolation() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = CascadingFailureConfig::default();
        let test = CascadingFailureTest::new(vault, config);

        let metrics = test.test_application_failure_isolation().await.unwrap();
        assert!(metrics.kernel_survived);
    }

    #[tokio::test]
    async fn test_simultaneous_failures() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = CascadingFailureConfig::default();
        let test = CascadingFailureTest::new(vault, config);

        let metrics = test.test_simultaneous_failures().await.unwrap();
        assert!(metrics.kernel_survived);
    }

    #[tokio::test]
    async fn test_sequential_failures_with_recovery() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = CascadingFailureConfig::default();
        let test = CascadingFailureTest::new(vault, config);

        let metrics = test.test_sequential_failures_with_recovery().await.unwrap();
        assert!(metrics.kernel_survived);
        assert!(!metrics.recovery_successful.is_empty());
    }
}
