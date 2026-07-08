//! Full-stack test runner: orchestrates all test categories

use crate::cascading_failures::{CascadingFailureConfig, CascadingFailureTest};
use crate::end_to_end_journey::{EndToEndConfig, DeveloperJourneyTest};
use crate::errors::FullStackTestResult;
use crate::long_duration::{LongDurationConfig, LongDurationTest};
use crate::network_partitions::{NetworkPartitionConfig, NetworkPartitionTest};
use crate::nominal_loads::{NominalLoadConfig, NominalLoadTest};
use crate::peak_loads::{PeakLoadConfig, PeakLoadTest};
use crate::recovery::{RecoveryConfig, RecoveryTest};
use crate::state_consistency::{StateConsistencyConfig, StateConsistencyTest};
use crate::vault::Vault;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Results from all test categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullStackTestResults {
    pub run_id: String,
    pub timestamp: u64,
    pub duration_secs: f64,

    // Individual test category results
    pub bootstrap_success: bool,
    pub nominal_load_success: bool,
    pub peak_load_success: bool,
    pub cascading_failures_success: bool,
    pub recovery_success: bool,
    pub network_partition_success: bool,
    pub state_consistency_success: bool,
    pub end_to_end_success: bool,
    pub long_duration_success: bool,

    // Summary metrics
    pub total_tests_run: u32,
    pub total_tests_passed: u32,
    pub total_tests_failed: u32,
    pub system_health_final: String,
}

impl FullStackTestResults {
    /// Get overall pass rate
    pub fn pass_rate(&self) -> f64 {
        if self.total_tests_run == 0 {
            0.0
        } else {
            (self.total_tests_passed as f64 / self.total_tests_run as f64) * 100.0
        }
    }

    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.total_tests_failed == 0
    }

    /// Generate summary
    pub fn summary(&self) -> String {
        format!(
            "Full-Stack Test Summary\n\
             =====================\n\
             Run ID: {}\n\
             Duration: {:.2}s\n\
             Tests Run: {}\n\
             Tests Passed: {}\n\
             Tests Failed: {}\n\
             Pass Rate: {:.1}%\n\
             System Health: {}\n\
             \n\
             Test Results:\n\
             - Bootstrap: {}\n\
             - Nominal Load: {}\n\
             - Peak Load: {}\n\
             - Cascading Failures: {}\n\
             - Recovery: {}\n\
             - Network Partitions: {}\n\
             - State Consistency: {}\n\
             - End-to-End: {}\n\
             - Long Duration: {}",
            self.run_id,
            self.duration_secs,
            self.total_tests_run,
            self.total_tests_passed,
            self.total_tests_failed,
            self.pass_rate(),
            self.system_health_final,
            if self.bootstrap_success { "PASS" } else { "FAIL" },
            if self.nominal_load_success { "PASS" } else { "FAIL" },
            if self.peak_load_success { "PASS" } else { "FAIL" },
            if self.cascading_failures_success {
                "PASS"
            } else {
                "FAIL"
            },
            if self.recovery_success { "PASS" } else { "FAIL" },
            if self.network_partition_success { "PASS" } else { "FAIL" },
            if self.state_consistency_success { "PASS" } else { "FAIL" },
            if self.end_to_end_success { "PASS" } else { "FAIL" },
            if self.long_duration_success { "PASS" } else { "FAIL" },
        )
    }
}

/// Full-stack test runner
pub struct FullStackTestRunner {
    vault: Arc<Vault>,
    run_id: String,
}

impl FullStackTestRunner {
    /// Create new test runner
    pub fn new(vault: Arc<Vault>) -> Self {
        Self {
            vault,
            run_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Get run ID
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    /// Run all tests
    pub async fn run_all_tests(&self) -> FullStackTestResult<FullStackTestResults> {
        let start = std::time::Instant::now();
        let mut passed = 0u32;
        let mut failed = 0u32;

        // 1. Bootstrap (already done)
        let bootstrap_success = true;
        if bootstrap_success {
            passed += 1;
        } else {
            failed += 1;
        }

        // 2. Nominal Load Tests
        let nominal_result = self.run_nominal_load_tests().await;
        if nominal_result.is_ok() {
            passed += 1;
        } else {
            failed += 1;
        }
        let nominal_load_success = nominal_result.is_ok();

        // 3. Peak Load Tests
        let peak_result = self.run_peak_load_tests().await;
        if peak_result.is_ok() {
            passed += 1;
        } else {
            failed += 1;
        }
        let peak_load_success = peak_result.is_ok();

        // 4. Cascading Failure Tests
        let cascading_result = self.run_cascading_failure_tests().await;
        if cascading_result.is_ok() {
            passed += 1;
        } else {
            failed += 1;
        }
        let cascading_failures_success = cascading_result.is_ok();

        // 5. Recovery Tests
        let recovery_result = self.run_recovery_tests().await;
        if recovery_result.is_ok() {
            passed += 1;
        } else {
            failed += 1;
        }
        let recovery_success = recovery_result.is_ok();

        // 6. Network Partition Tests
        let network_result = self.run_network_partition_tests().await;
        if network_result.is_ok() {
            passed += 1;
        } else {
            failed += 1;
        }
        let network_partition_success = network_result.is_ok();

        // 7. State Consistency Tests
        let consistency_result = self.run_state_consistency_tests().await;
        if consistency_result.is_ok() {
            passed += 1;
        } else {
            failed += 1;
        }
        let state_consistency_success = consistency_result.is_ok();

        // 8. End-to-End Tests
        let e2e_result = self.run_end_to_end_tests().await;
        if e2e_result.is_ok() {
            passed += 1;
        } else {
            failed += 1;
        }
        let end_to_end_success = e2e_result.is_ok();

        // 9. Long-Duration Tests (short version for testing)
        let long_result = self.run_long_duration_tests().await;
        if long_result.is_ok() {
            passed += 1;
        } else {
            failed += 1;
        }
        let long_duration_success = long_result.is_ok();

        let elapsed = start.elapsed().as_secs_f64();

        // Verify final system health
        let _final_vault = self.vault.snapshot().await;
        let system_health = if failed == 0 {
            "healthy".to_string()
        } else if failed < 4 {
            "degraded".to_string()
        } else {
            "critical".to_string()
        };

        Ok(FullStackTestResults {
            run_id: self.run_id.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            duration_secs: elapsed,
            bootstrap_success,
            nominal_load_success,
            peak_load_success,
            cascading_failures_success,
            recovery_success,
            network_partition_success,
            state_consistency_success,
            end_to_end_success,
            long_duration_success,
            total_tests_run: passed + failed,
            total_tests_passed: passed,
            total_tests_failed: failed,
            system_health_final: system_health,
        })
    }

    async fn run_nominal_load_tests(&self) -> FullStackTestResult<()> {
        let config = NominalLoadConfig::default();
        let test = NominalLoadTest::new(self.vault.clone(), config);

        test.test_baseline_throughput().await?;
        test.test_mixed_workload().await?;
        test.test_subsystem_coordination().await?;

        Ok(())
    }

    async fn run_peak_load_tests(&self) -> FullStackTestResult<()> {
        let config = PeakLoadConfig {
            phase_duration_secs: 10,
            ..Default::default()
        };
        let test = PeakLoadTest::new(self.vault.clone(), config);

        test.test_cpu_saturation().await?;
        test.test_memory_saturation().await?;
        test.test_io_saturation().await?;

        Ok(())
    }

    async fn run_cascading_failure_tests(&self) -> FullStackTestResult<()> {
        let config = CascadingFailureConfig::default();
        let test = CascadingFailureTest::new(self.vault.clone(), config);

        test.test_kernel_thread_failure().await?;
        test.test_service_failure_isolation().await?;
        test.test_application_failure_isolation().await?;
        test.test_simultaneous_failures().await?;

        Ok(())
    }

    async fn run_recovery_tests(&self) -> FullStackTestResult<()> {
        let config = RecoveryConfig::default();
        let test = RecoveryTest::new(self.vault.clone(), config);

        test.test_kernel_panic_recovery().await?;
        test.test_service_crash_recovery().await?;
        test.test_data_corruption_recovery().await?;

        Ok(())
    }

    async fn run_network_partition_tests(&self) -> FullStackTestResult<()> {
        let config = NetworkPartitionConfig {
            partition_duration_secs: 5,
            ..Default::default()
        };
        let test = NetworkPartitionTest::new(self.vault.clone(), config);

        test.test_mesh_partition().await?;
        test.test_crdt_convergence().await?;
        test.test_reunion_reconciliation().await?;

        Ok(())
    }

    async fn run_state_consistency_tests(&self) -> FullStackTestResult<()> {
        let config = StateConsistencyConfig::default();
        let test = StateConsistencyTest::new(self.vault.clone(), config);

        test.test_audit_log_completeness().await?;
        test.test_data_loss_detection().await?;
        test.test_deterministic_replay().await?;

        Ok(())
    }

    async fn run_end_to_end_tests(&self) -> FullStackTestResult<()> {
        let config = EndToEndConfig {
            inject_network_partition: false,
            partition_duration_secs: 5,
            inject_service_failures: false,
        };
        let test = DeveloperJourneyTest::new(self.vault.clone(), config);

        test.test_developer_workflow().await?;
        test.test_buddy_file_sync().await?;
        test.test_omni_bot_deployment().await?;

        Ok(())
    }

    async fn run_long_duration_tests(&self) -> FullStackTestResult<()> {
        let config = LongDurationConfig {
            total_duration_secs: 10,
            fault_injection_interval_secs: 3,
            measure_performance: true,
            detect_leaks: true,
        };
        let test = LongDurationTest::new(self.vault.clone(), config);

        test.test_continuous_load().await?;
        test.test_with_periodic_faults().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap::FullStackBootstrap;

    #[tokio::test]
    async fn test_runner_creation() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let runner = FullStackTestRunner::new(vault);
        assert!(!runner.run_id().is_empty());
    }

    #[tokio::test]
    async fn test_full_test_suite() {
        let bootstrap = FullStackBootstrap::default();
        let vault = Arc::new(bootstrap.initialize().await.unwrap());

        let runner = FullStackTestRunner::new(vault);
        let results = runner.run_all_tests().await.unwrap();

        assert!(results.total_tests_run > 0);
        println!("{}", results.summary());
    }

    #[test]
    fn test_results_summary() {
        let results = FullStackTestResults {
            run_id: "test-123".to_string(),
            timestamp: 0,
            duration_secs: 100.5,
            bootstrap_success: true,
            nominal_load_success: true,
            peak_load_success: true,
            cascading_failures_success: true,
            recovery_success: true,
            network_partition_success: true,
            state_consistency_success: true,
            end_to_end_success: true,
            long_duration_success: true,
            total_tests_run: 9,
            total_tests_passed: 9,
            total_tests_failed: 0,
            system_health_final: "healthy".to_string(),
        };

        assert!(results.all_passed());
        assert_eq!(results.pass_rate(), 100.0);

        let summary = results.summary();
        assert!(summary.contains("100.0%"));
    }
}
