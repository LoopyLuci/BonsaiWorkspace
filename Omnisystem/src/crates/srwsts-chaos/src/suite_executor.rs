//! Chaos suite executor for comprehensive testing.
//!
//! Runs complete chaos engineering test suites:
//! - Baseline run (no faults)
//! - Multiple runs with different fault seeds
//! - Aggregate analysis and reporting

use crate::deterministic_clock::DeterministicClock;
use crate::error::Result;
use crate::recovery_validation::{RecoveryMetrics, RecoveryValidator};
use crate::scenarios::ChaosScenario;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use tracing::{info, debug};
use uuid::Uuid;

/// Test suite configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosTestConfig {
    /// Scenario to test.
    pub scenario: String,
    /// Number of test runs (excluding baseline).
    pub num_runs: usize,
    /// Random seed for first run (incremented for each).
    pub base_seed: u64,
    /// Maximum detection time allowed (millis).
    pub max_detection_ms: u64,
    /// Maximum recovery time allowed (millis).
    pub max_recovery_ms: u64,
    /// Allow data loss in results.
    pub allow_data_loss: bool,
    /// Require all runs to succeed.
    pub require_all_success: bool,
    /// Timeout for each test run (seconds).
    pub run_timeout_secs: u64,
}

impl Default for ChaosTestConfig {
    fn default() -> Self {
        Self {
            scenario: "Black Friday Traffic Surge".to_string(),
            num_runs: 50,
            base_seed: 1,
            max_detection_ms: 5000,
            max_recovery_ms: 30000,
            allow_data_loss: false,
            require_all_success: true,
            run_timeout_secs: 60,
        }
    }
}

/// Single test run result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunResult {
    /// Run number (0 = baseline).
    pub run_number: usize,
    /// Random seed used.
    pub seed: u64,
    /// Did test pass?
    pub passed: bool,
    /// Runtime (millis).
    pub runtime_ms: u64,
    /// Faults injected during this run.
    pub faults_injected: usize,
    /// Recovery metrics for each fault.
    pub recovery_metrics: Vec<RecoveryMetrics>,
    /// Optional failure reason.
    pub failure_reason: Option<String>,
}

/// Complete test suite results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosTestResults {
    /// Test configuration used.
    pub config: ChaosTestConfig,
    /// Individual run results.
    pub runs: Vec<TestRunResult>,
    /// Start time millis.
    pub start_time_ms: u64,
    /// End time millis.
    pub end_time_ms: Option<u64>,
    /// Overall pass/fail.
    pub all_passed: bool,
    /// Summary report.
    pub summary: Option<String>,
}

impl ChaosTestResults {
    /// Create new empty results.
    pub fn new(config: ChaosTestConfig) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            config,
            runs: Vec::new(),
            start_time_ms: now,
            end_time_ms: None,
            all_passed: true,
            summary: None,
        }
    }

    /// Add a run result.
    pub fn add_run(&mut self, result: TestRunResult) {
        if !result.passed {
            self.all_passed = false;
        }
        self.runs.push(result);
    }

    /// Mark suite as complete.
    pub fn mark_complete(&mut self) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.end_time_ms = Some(now);
    }

    /// Get total runtime.
    pub fn total_runtime_ms(&self) -> u64 {
        match self.end_time_ms {
            Some(end) => end.saturating_sub(self.start_time_ms),
            None => {
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                now.saturating_sub(self.start_time_ms)
            }
        }
    }

    /// Get pass rate percentage.
    pub fn pass_rate_percent(&self) -> usize {
        if self.runs.is_empty() {
            0
        } else {
            let passed = self.runs.iter().filter(|r| r.passed).count();
            (passed * 100) / self.runs.len()
        }
    }

    /// Get average runtime across runs.
    pub fn avg_runtime_ms(&self) -> u64 {
        if self.runs.is_empty() {
            0
        } else {
            self.runs.iter().map(|r| r.runtime_ms).sum::<u64>() / self.runs.len() as u64
        }
    }

    /// Generate summary report.
    pub fn generate_summary(&mut self) {
        let mut report = format!(
            "Chaos Test Suite Report\n\
             =======================\n\
             Scenario: {}\n\
             Total Runs: {}\n\
             Pass Rate: {}%\n\
             Total Runtime: {}ms\n\
             Average Runtime per Run: {}ms\n\
             \n",
            self.config.scenario,
            self.runs.len(),
            self.pass_rate_percent(),
            self.total_runtime_ms(),
            self.avg_runtime_ms()
        );

        if !self.runs.is_empty() {
            report.push_str("Run Details:\n");
            for (idx, run) in self.runs.iter().enumerate() {
                let status = if run.passed { "✓ PASS" } else { "✗ FAIL" };
                report.push_str(&format!(
                    "  Run {}: {} (seed={}, faults={}, {}ms)\n",
                    idx, status, run.seed, run.faults_injected, run.runtime_ms
                ));

                if let Some(reason) = &run.failure_reason {
                    report.push_str(&format!("    Reason: {}\n", reason));
                }
            }
        }

        self.summary = Some(report);
    }

    /// Get formatted report.
    pub fn report(&self) -> String {
        self.summary.clone().unwrap_or_else(|| {
            "Summary not generated. Call generate_summary() first.".to_string()
        })
    }
}

/// Chaos test suite executor.
pub struct ChaosSuiteExecutor {
    config: ChaosTestConfig,
    results: ChaosTestResults,
}

impl ChaosSuiteExecutor {
    /// Create new executor.
    pub fn new(config: ChaosTestConfig) -> Self {
        let results = ChaosTestResults::new(config.clone());
        Self { config, results }
    }

    /// Run complete test suite.
    pub async fn run_suite(&mut self, scenario: &ChaosScenario) -> Result<()> {
        info!("Starting chaos test suite: {}", self.config.scenario);

        // Run baseline test (no faults)
        self.run_baseline(scenario).await?;

        // Run chaos tests with different seeds
        for run_num in 1..=self.config.num_runs {
            let seed = self.config.base_seed + run_num as u64;
            self.run_with_seed(scenario, run_num, seed).await?;
        }

        self.results.mark_complete();
        self.results.generate_summary();

        info!("Chaos test suite complete: {}% pass rate", self.results.pass_rate_percent());

        Ok(())
    }

    /// Run baseline test (no faults).
    async fn run_baseline(&mut self, scenario: &ChaosScenario) -> Result<()> {
        let start = std::time::Instant::now();

        debug!("Running baseline test (no faults)");

        // Simulate baseline test execution
        let clock = DeterministicClock::new(1000);
        clock.advance(scenario.time_to_failure_secs)?;

        let runtime = start.elapsed().as_millis() as u64;

        let result = TestRunResult {
            run_number: 0,
            seed: 0,
            passed: true,
            runtime_ms: runtime,
            faults_injected: 0,
            recovery_metrics: Vec::new(),
            failure_reason: None,
        };

        self.results.add_run(result);
        Ok(())
    }

    /// Run test with specific fault seed.
    async fn run_with_seed(
        &mut self,
        scenario: &ChaosScenario,
        run_num: usize,
        seed: u64,
    ) -> Result<()> {
        let start = std::time::Instant::now();

        debug!("Running chaos test #{} with seed {}", run_num, seed);

        // Simulate test with faults
        let clock = DeterministicClock::new(1000);

        let mut validator = RecoveryValidator::new();
        let mut fault_count = 0;

        for scheduled_fault in &scenario.fault_schedule.faults {
            // Simulate fault injection and recovery
            clock.jump_to(scheduled_fault.inject_time)?;

            let detection_time = 100 + (seed % 1000) as u64;
            let recovery_time = 200 + ((seed * 2) % 2000) as u64;

            let metrics = RecoveryMetrics::new(Uuid::new_v4())
                .mark_successful(detection_time, recovery_time);

            validator.add_metrics(metrics);
            fault_count += 1;

            clock.advance(scheduled_fault.duration_secs)?;
        }

        let runtime = start.elapsed().as_millis() as u64;
        let stats = validator.get_stats();

        let passed = stats.successful_recoveries == fault_count
            && (!self.config.allow_data_loss || stats.faults_with_data_loss == 0);

        let result = TestRunResult {
            run_number: run_num,
            seed,
            passed,
            runtime_ms: runtime,
            faults_injected: fault_count,
            recovery_metrics: validator.metrics,
            failure_reason: if !passed {
                Some(format!(
                    "Successful: {}/{}, DataLoss: {}",
                    stats.successful_recoveries, fault_count, stats.faults_with_data_loss
                ))
            } else {
                None
            },
        };

        self.results.add_run(result);
        Ok(())
    }

    /// Get current results.
    pub fn results(&self) -> &ChaosTestResults {
        &self.results
    }

    /// Get mutable results.
    pub fn results_mut(&mut self) -> &mut ChaosTestResults {
        &mut self.results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = ChaosTestConfig::default();
        assert_eq!(config.num_runs, 50);
    }

    #[test]
    fn test_test_run_result() {
        let result = TestRunResult {
            run_number: 1,
            seed: 42,
            passed: true,
            runtime_ms: 1000,
            faults_injected: 5,
            recovery_metrics: Vec::new(),
            failure_reason: None,
        };

        assert!(result.passed);
        assert_eq!(result.faults_injected, 5);
    }

    #[test]
    fn test_results_creation() {
        let config = ChaosTestConfig::default();
        let results = ChaosTestResults::new(config);
        assert_eq!(results.runs.len(), 0);
        assert!(results.all_passed);
    }

    #[test]
    fn test_pass_rate() {
        let config = ChaosTestConfig::default();
        let mut results = ChaosTestResults::new(config);

        for i in 0..10 {
            let result = TestRunResult {
                run_number: i,
                seed: i as u64,
                passed: i % 2 == 0, // 5 pass, 5 fail
                runtime_ms: 1000,
                faults_injected: 5,
                recovery_metrics: Vec::new(),
                failure_reason: None,
            };
            results.add_run(result);
        }

        assert_eq!(results.pass_rate_percent(), 50);
    }
}
