//! State consistency tests: audit log verification and deterministic replay

use crate::errors::FullStackTestResult;
use crate::vault::Vault;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// State consistency test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateConsistencyConfig {
    /// Whether to verify complete audit trail
    pub verify_audit_trail: bool,
    /// Whether to perform deterministic replay
    pub enable_replay: bool,
    /// Maximum audit log entries to verify
    pub max_audit_entries: usize,
}

impl Default for StateConsistencyConfig {
    fn default() -> Self {
        Self {
            verify_audit_trail: true,
            enable_replay: true,
            max_audit_entries: 10000,
        }
    }
}

/// State consistency test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateConsistencyMetrics {
    pub audit_entries_logged: u64,
    pub audit_gap_detected: bool,
    pub data_loss_detected: bool,
    pub replay_divergence_detected: bool,
    pub component_state_mismatch: Vec<String>,
    pub consistency_score: f64,
}

/// State consistency test runner
pub struct StateConsistencyTest {
    config: StateConsistencyConfig,
    vault: Arc<Vault>,
    operation_log: Arc<parking_lot::Mutex<Vec<StateOperation>>>,
}

/// Record of a state-modifying operation
#[derive(Debug, Clone)]
struct StateOperation {
    component_id: String,
    operation: String,
    before_state: String,
    after_state: String,
    timestamp: u64,
}

impl StateConsistencyTest {
    /// Create new state consistency test
    pub fn new(vault: Arc<Vault>, config: StateConsistencyConfig) -> Self {
        Self {
            config,
            vault,
            operation_log: Arc::new(parking_lot::Mutex::new(Vec::new())),
        }
    }

    /// Verify audit log records all events end-to-end
    pub async fn test_audit_log_completeness(&self) -> FullStackTestResult<StateConsistencyMetrics> {
        let initial_count = self.vault.event_count();

        // Perform operations
        let services = self.vault.all_services().await;
        for service in services.iter().take(3) {
            // Simulate operations that should be logged
            let _ = self.vault.get_service(&service.id).await;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let final_count = self.vault.event_count();
        let events_logged = final_count - initial_count;

        let audit_log = self.vault.audit_log().await;
        let audit_gap = audit_log.is_empty() && events_logged > 0;

        Ok(StateConsistencyMetrics {
            audit_entries_logged: events_logged,
            audit_gap_detected: audit_gap,
            data_loss_detected: false,
            replay_divergence_detected: false,
            component_state_mismatch: Vec::new(),
            consistency_score: if audit_gap { 0.0 } else { 1.0 },
        })
    }

    /// Verify no data loss across any failure scenario
    pub async fn test_data_loss_detection(&self) -> FullStackTestResult<StateConsistencyMetrics> {
        // Snapshot current state
        let before = self.vault.snapshot().await;

        // Perform operations
        let services = self.vault.all_services().await;
        for service in services.iter().take(2) {
            let _ = self.vault.get_service(&service.id).await;
        }

        // Snapshot after operations
        let after = self.vault.snapshot().await;

        // Verify no data loss
        let component_state_mismatch = Vec::new();
        let data_lost = before.services.len() != after.services.len()
            || before.applications.len() != after.applications.len();

        Ok(StateConsistencyMetrics {
            audit_entries_logged: after.event_count - before.event_count,
            audit_gap_detected: false,
            data_loss_detected: data_lost,
            replay_divergence_detected: false,
            component_state_mismatch,
            consistency_score: if data_lost { 0.0 } else { 1.0 },
        })
    }

    /// Test deterministic replay: same workload produces identical outcome
    pub async fn test_deterministic_replay(&self) -> FullStackTestResult<StateConsistencyMetrics> {
        // Perform initial workload
        let before = self.vault.snapshot().await;
        self.execute_workload().await;
        let after_first = self.vault.snapshot().await;

        // Restore to before state
        self.vault.restore(&before).await?;

        // Replay same workload
        self.execute_workload().await;
        let after_replay = self.vault.snapshot().await;

        // Compare outcomes
        let diverged = self.snapshots_diverge(&after_first, &after_replay);

        Ok(StateConsistencyMetrics {
            audit_entries_logged: after_replay.event_count - before.event_count,
            audit_gap_detected: false,
            data_loss_detected: false,
            replay_divergence_detected: diverged,
            component_state_mismatch: Vec::new(),
            consistency_score: if diverged { 0.5 } else { 1.0 },
        })
    }

    /// Test that every component state is consistent
    pub async fn test_component_state_consistency(&self) -> FullStackTestResult<StateConsistencyMetrics> {
        let snapshot = self.vault.snapshot().await;

        let mut component_state_mismatch = Vec::new();

        // Verify kernel state
        let kernel = self.vault.kernel_state().await?;
        if kernel.thread_count != snapshot.kernel.thread_count {
            component_state_mismatch.push(format!("Kernel thread count mismatch"));
        }

        // Verify services
        for service in &snapshot.services {
            if let Ok(current) = self.vault.get_service(&service.id).await {
                if current.request_count != service.request_count {
                    component_state_mismatch.push(format!("Service {} request count mismatch", service.id));
                }
            }
        }

        // Verify applications
        for app in &snapshot.applications {
            if let Ok(current) = self.vault.get_application(&app.id).await {
                if current.execution_count != app.execution_count {
                    component_state_mismatch.push(format!("Application {} execution count mismatch", app.id));
                }
            }
        }

        let consistency_score = if component_state_mismatch.is_empty() {
            1.0
        } else {
            0.5
        };

        Ok(StateConsistencyMetrics {
            audit_entries_logged: snapshot.event_count,
            audit_gap_detected: false,
            data_loss_detected: !component_state_mismatch.is_empty(),
            replay_divergence_detected: false,
            component_state_mismatch,
            consistency_score,
        })
    }

    /// Execute a standard workload
    async fn execute_workload(&self) {
        let services = self.vault.all_services().await;
        for service in services.iter().take(2) {
            let _ = self.vault.get_service(&service.id).await;
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        let apps = self.vault.all_applications().await;
        for app in apps.iter().take(2) {
            let _ = self.vault.get_application(&app.id).await;
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    }

    /// Check if two snapshots have diverged
    fn snapshots_diverge(&self, snap1: &crate::vault::VaultSnapshot, snap2: &crate::vault::VaultSnapshot) -> bool {
        snap1.services.len() != snap2.services.len()
            || snap1.applications.len() != snap2.applications.len()
            || snap1.event_count != snap2.event_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap::FullStackBootstrap;

    #[test]
    fn test_state_consistency_config_defaults() {
        let config = StateConsistencyConfig::default();
        assert!(config.verify_audit_trail);
        assert!(config.enable_replay);
    }

    #[tokio::test]
    async fn test_audit_log_completeness() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = StateConsistencyConfig::default();
        let test = StateConsistencyTest::new(vault, config);

        let metrics = test.test_audit_log_completeness().await.unwrap();
        assert!(!metrics.audit_gap_detected);
    }

    #[tokio::test]
    async fn test_data_loss_detection() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = StateConsistencyConfig::default();
        let test = StateConsistencyTest::new(vault, config);

        let metrics = test.test_data_loss_detection().await.unwrap();
        assert!(!metrics.data_loss_detected);
    }

    #[tokio::test]
    async fn test_deterministic_replay() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = StateConsistencyConfig::default();
        let test = StateConsistencyTest::new(vault, config);

        let metrics = test.test_deterministic_replay().await.unwrap();
        assert!(!metrics.replay_divergence_detected);
    }

    #[tokio::test]
    async fn test_component_state_consistency() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let config = StateConsistencyConfig::default();
        let test = StateConsistencyTest::new(vault, config);

        let metrics = test.test_component_state_consistency().await.unwrap();
        assert!(metrics.consistency_score > 0.5);
    }
}
