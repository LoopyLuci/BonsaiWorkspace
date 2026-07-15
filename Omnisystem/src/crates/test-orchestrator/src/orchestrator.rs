//! Top-level orchestration: wires the scheduler, runner, comparer, and
//! storage modules together into a single `run_spec` entry point.

use crate::comparer;
use crate::runner;
use crate::scheduler::Scheduler;
use crate::spec::TestSpec;
use crate::storage::{SpecStats, TestResult, TestStatus, TestStorage};
use std::path::PathBuf;
use std::time::Duration;

/// Configuration for an orchestrator run: currently just the working
/// directory used for result storage and test artifacts.
pub struct UtofConfig {
    pub work_dir: PathBuf,
}

impl UtofConfig {
    /// Create a new config, creating the working directory if it doesn't
    /// already exist.
    pub fn new(work_dir: PathBuf) -> anyhow::Result<Self> {
        if !work_dir.exists() {
            std::fs::create_dir_all(&work_dir)?;
        }
        Ok(Self { work_dir })
    }
}

/// Ties the scheduler, runner, comparer, and storage together to actually
/// execute a `TestSpec` end to end.
pub struct Orchestrator {
    config: UtofConfig,
    storage: TestStorage,
}

impl Orchestrator {
    pub fn new(config: UtofConfig) -> anyhow::Result<Self> {
        let storage_dir = config.work_dir.join("results");
        let storage = TestStorage::new(storage_dir.to_string_lossy().to_string());
        Ok(Self { config, storage })
    }

    /// Access the underlying result storage (e.g. to export/inspect results
    /// after a run).
    pub fn results(&self) -> &TestStorage {
        &self.storage
    }

    /// The working directory this orchestrator was configured with.
    pub fn work_dir(&self) -> &PathBuf {
        &self.config.work_dir
    }

    /// Run every job the scheduler generates for `spec`, comparing actual
    /// output against the expected output for each test case, storing a
    /// real `TestResult` per job, and returning aggregate statistics.
    pub async fn run_spec(&mut self, spec: &TestSpec) -> anyhow::Result<SpecStats> {
        let mut scheduler = Scheduler::new(spec);
        let jobs = scheduler.remaining_jobs();

        // Fidelity isn't part of the persisted TestResult schema (storage.rs
        // is kept as-is), so we track it locally from the real comparisons
        // performed during this run and fold it into the final stats.
        let mut fidelities = Vec::with_capacity(jobs.len());

        for job in jobs {
            let test_case = &spec.test_cases[job.test_case_index];
            let runner_template = spec.runners.get(&job.lang).map(String::as_str);
            let timeout = Duration::from_secs(spec.timeout_secs());

            let start = std::time::Instant::now();
            let run_result = runner::run_test(
                &job.lang,
                &spec.canonical_source,
                &test_case.input,
                job.seed,
                runner_template,
                timeout,
            )
            .await;
            let duration_ms = start.elapsed().as_millis();

            let (status, output, fidelity) = match run_result {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let fidelity = comparer::compare_outputs(&stdout, &test_case.expected);
                    let status = if fidelity >= spec.fidelity_threshold() {
                        TestStatus::Passed
                    } else {
                        TestStatus::Failed
                    };
                    (status, stdout, fidelity)
                }
                Err(e) => (TestStatus::Error, e.to_string(), 0.0),
            };

            fidelities.push(fidelity);

            let result = TestResult {
                test_id: format!("{}-{}-{}", spec.name, job.lang, job.test_case_index),
                test_name: test_case.name.clone(),
                result: status,
                duration_ms,
                output,
                timestamp: chrono::Utc::now().timestamp(),
            };

            self.storage.store_result(result).await?;
        }

        let results = self.storage.get_all_results().await?;
        Ok(Self::compute_stats(&spec.name, &results, &fidelities))
    }

    /// Pure stats computation, deliberately separated from any I/O or
    /// subprocess execution so it can be unit-tested hermetically (no
    /// dependency on which language runtimes happen to be installed).
    fn compute_stats(spec_name: &str, results: &[TestResult], fidelities: &[f64]) -> SpecStats {
        let total_tests = results.len();
        let passed = results
            .iter()
            .filter(|r| r.result == TestStatus::Passed)
            .count();
        let failed = total_tests.saturating_sub(passed);
        let success_rate = if total_tests == 0 {
            0.0
        } else {
            (passed as f64 / total_tests as f64) * 100.0
        };
        let avg_fidelity = if fidelities.is_empty() {
            0.0
        } else {
            fidelities.iter().sum::<f64>() / fidelities.len() as f64
        };
        let total_execution_time_ms: u64 = results.iter().map(|r| r.duration_ms as u64).sum();

        SpecStats {
            spec_name: spec_name.to_string(),
            total_tests,
            passed,
            failed,
            success_rate,
            avg_fidelity,
            total_execution_time_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{TestResult, TestStatus};

    fn make_result(name: &str, status: TestStatus, duration_ms: u128) -> TestResult {
        TestResult {
            test_id: format!("id-{name}"),
            test_name: name.to_string(),
            result: status,
            duration_ms,
            output: "output".to_string(),
            timestamp: 0,
        }
    }

    #[test]
    fn test_utof_config_creates_work_dir() {
        let dir = std::env::temp_dir().join(format!(
            "utof_test_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!dir.exists());

        let config = UtofConfig::new(dir.clone()).unwrap();
        assert!(dir.exists());
        assert_eq!(config.work_dir, dir);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_orchestrator_new_creates_storage() {
        let dir = std::env::temp_dir().join(format!(
            "utof_orchestrator_test_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let config = UtofConfig::new(dir.clone()).unwrap();
        let orchestrator = Orchestrator::new(config).unwrap();

        assert!(orchestrator.results().get_all_results().await.unwrap().is_empty());
        assert_eq!(orchestrator.work_dir(), &dir);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Hermetic test of the stats-computation math: no subprocesses, no
    /// dependency on which language toolchains are installed in this
    /// environment. Verifies the aggregate numbers are genuinely computed
    /// from the (hand-built) results rather than hardcoded.
    #[test]
    fn test_compute_stats_all_passed() {
        let results = vec![
            make_result("a", TestStatus::Passed, 10),
            make_result("b", TestStatus::Passed, 20),
        ];
        let fidelities = vec![1.0, 0.98];

        let stats = Orchestrator::compute_stats("MySpec", &results, &fidelities);

        assert_eq!(stats.spec_name, "MySpec");
        assert_eq!(stats.total_tests, 2);
        assert_eq!(stats.passed, 2);
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.success_rate, 100.0);
        assert!((stats.avg_fidelity - 0.99).abs() < 1e-9);
        assert_eq!(stats.total_execution_time_ms, 30);
    }

    #[test]
    fn test_compute_stats_mixed_results() {
        let results = vec![
            make_result("a", TestStatus::Passed, 5),
            make_result("b", TestStatus::Failed, 15),
            make_result("c", TestStatus::Error, 0),
        ];
        let fidelities = vec![1.0, 0.5, 0.0];

        let stats = Orchestrator::compute_stats("MixedSpec", &results, &fidelities);

        assert_eq!(stats.total_tests, 3);
        assert_eq!(stats.passed, 1);
        assert_eq!(stats.failed, 2);
        assert!((stats.success_rate - (100.0 / 3.0)).abs() < 1e-9);
        assert!((stats.avg_fidelity - 0.5).abs() < 1e-9);
        assert_eq!(stats.total_execution_time_ms, 20);
    }

    #[test]
    fn test_compute_stats_empty() {
        let stats = Orchestrator::compute_stats("EmptySpec", &[], &[]);
        assert_eq!(stats.total_tests, 0);
        assert_eq!(stats.success_rate, 0.0);
        assert_eq!(stats.avg_fidelity, 0.0);
    }
}
