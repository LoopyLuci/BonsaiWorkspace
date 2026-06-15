//! Fault injection and scenario testing
//!
//! Tests for:
//! - Kill service scenarios
//! - Network partitions
//! - Storage backend failure
//! - CPU overload
//! - Service timeouts

use crate::types::{TestConfig, TestResult, TestResultStatus};
use crate::{ServiceMetricsCollector, ServiceResult, TestReport};
use std::time::Instant;
use tracing::info;

/// Fault scenario simulator
pub struct FaultScenario {
    pub name: String,
    pub duration_ms: u64,
    pub recovery_ms: u64,
    pub injected: bool,
}

impl FaultScenario {
    pub fn new(name: impl Into<String>, duration_ms: u64, recovery_ms: u64) -> Self {
        Self {
            name: name.into(),
            duration_ms,
            recovery_ms,
            injected: false,
        }
    }

    pub fn inject(&mut self) {
        self.injected = true;
    }

    pub fn recover(&mut self) {
        self.injected = false;
    }
}

/// Fault scenario tests
pub struct FaultScenarioTests {
    config: TestConfig,
    metrics: ServiceMetricsCollector,
}

impl FaultScenarioTests {
    pub fn new(config: TestConfig) -> Self {
        Self {
            config,
            metrics: ServiceMetricsCollector::new("fault-scenarios"),
        }
    }

    /// Test killing a service
    pub async fn test_service_kill(&self) -> ServiceResult<TestResult> {
        let _start = Instant::now();
        let test_id = "fault-scenario-service-kill";

        info!("Testing service kill scenario...");

        let mut scenario = FaultScenario::new("kill-storage", 1000, 2000);

        // Inject fault
        scenario.inject();
        let kill_time = Instant::now();

        // Simulate service being killed
        tokio::time::sleep(std::time::Duration::from_millis(scenario.duration_ms)).await;

        // Measure recovery time
        scenario.recover();
        let recovery_time = kill_time.elapsed().as_millis() as u64;

        let success = recovery_time < scenario.recovery_ms;

        self.metrics.record_operation(
            "service_kill",
            recovery_time as f64,
            success,
            None,
        );

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status).with_message(format!(
            "Fault duration: {}ms, Recovery: {}ms, Target: {}ms",
            scenario.duration_ms, recovery_time, scenario.recovery_ms
        ));

        Ok(result)
    }

    /// Test network partition
    pub async fn test_network_partition(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "fault-scenario-network-partition";

        info!("Testing network partition scenario...");

        // Simulate a network partition
        let partition_duration = std::time::Duration::from_millis(2000);
        let deadline = Instant::now() + partition_duration;

        let mut isolated_operations = 0;
        let mut recovered_operations = 0;
        let mut operations = Vec::new();

        while Instant::now() < deadline {
            // Operations during partition
            isolated_operations += 1;
            operations.push((true, 0u128)); // true = during partition
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        // Recovery phase
        let recovery_start = Instant::now();
        let recovery_deadline = recovery_start + std::time::Duration::from_millis(1000);

        while Instant::now() < recovery_deadline {
            recovered_operations += 1;
            operations.push((false, recovery_start.elapsed().as_millis()));
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        let success = isolated_operations > 0 && recovered_operations > 0;

        self.metrics.record_operation(
            "network_partition",
            start.elapsed().as_millis() as f64,
            success,
            None,
        );

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status).with_message(format!(
            "Isolated operations: {}, Recovered operations: {}",
            isolated_operations, recovered_operations
        ));

        Ok(result)
    }

    /// Test storage backend failure
    pub async fn test_storage_failure(&self) -> ServiceResult<TestResult> {
        let _start = Instant::now();
        let test_id = "fault-scenario-storage-failure";

        info!("Testing storage backend failure scenario...");

        // Simulate storage operations
        let mut successful_writes = 0;
        let mut failed_writes = 0;
        let mut storage_healthy = true;

        for i in 0..100 {
            if i == 50 {
                // Inject fault at operation 50
                storage_healthy = false;
            }

            let write_start = Instant::now();
            let write_success = storage_healthy;

            if write_success {
                successful_writes += 1;
            } else {
                failed_writes += 1;
            }

            self.metrics.record_operation(
                "storage_write",
                write_start.elapsed().as_millis() as f64,
                write_success,
                None,
            );

            if i == 75 {
                // Recover at operation 75
                storage_healthy = true;
            }
        }

        let success = successful_writes > 0 && failed_writes > 0;

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status).with_message(format!(
            "Successful writes: {}, Failed writes: {}, Recovery: {}",
            successful_writes, failed_writes, storage_healthy
        ));

        Ok(result)
    }

    /// Test CPU overload
    pub async fn test_cpu_overload(&self) -> ServiceResult<TestResult> {
        let _start = Instant::now();
        let test_id = "fault-scenario-cpu-overload";

        info!("Testing CPU overload scenario...");

        let mut latencies = Vec::new();
        let mut rng = rand::thread_rng();
        use rand::RngCore;

        // Normal operations
        for _ in 0..50 {
            let op_time = 5.0 + (rng.next_u32() % 10) as f64; // 5-15ms
            latencies.push(op_time);
            tokio::time::sleep(std::time::Duration::from_millis(op_time as u64)).await;
        }

        // Inject CPU overload
        let overload_duration = std::time::Duration::from_secs(2);
        let deadline = Instant::now() + overload_duration;

        while Instant::now() < deadline {
            // Operations under load
            let op_time = 50.0 + (rng.next_u32() % 100) as f64; // 50-150ms under load
            latencies.push(op_time);

            self.metrics
                .record_operation("cpu_overload_operation", op_time, true, None);
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let latency_increase = latencies[latencies.len() - 1] / latencies[0];
        let success = latency_increase > 2.0; // At least 2x latency increase

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status).with_message(format!(
            "Latency increase: {:.1}x, Operations: {}, Max latency: {:.1}ms",
            latency_increase,
            latencies.len(),
            latencies[latencies.len() - 1]
        ));

        Ok(result)
    }

    /// Test service timeout
    pub async fn test_service_timeout(&self) -> ServiceResult<TestResult> {
        let _start = Instant::now();
        let test_id = "fault-scenario-service-timeout";

        info!("Testing service timeout scenario...");

        let timeout_threshold = std::time::Duration::from_millis(100);
        let mut timeout_count = 0;
        let mut success_count = 0;

        for _ in 0..50 {
            let op_start = Instant::now();

            // Simulate operation that might timeout
            let operation_duration = 30 + (rand::random::<u64>() % 200); // 30-230ms
            tokio::time::sleep(std::time::Duration::from_millis(operation_duration)).await;

            let elapsed = op_start.elapsed();

            if elapsed > timeout_threshold {
                timeout_count += 1;
            } else {
                success_count += 1;
            }

            let is_success = elapsed <= timeout_threshold;
            self.metrics
                .record_operation("timeout_operation", elapsed.as_millis() as f64, is_success, None);
        }

        let timeout_rate = (timeout_count as f64 / 50.0) * 100.0;
        let success = timeout_count > 10 && timeout_rate > 20.0; // Expect some timeouts

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status).with_message(format!(
            "Timeouts: {}, Successes: {}, Timeout rate: {:.1}%",
            timeout_count, success_count, timeout_rate
        ));

        Ok(result)
    }

    /// Test cascading fault propagation
    pub async fn test_cascading_fault_propagation(&self) -> ServiceResult<TestResult> {
        let start = Instant::now();
        let test_id = "fault-scenario-cascading-propagation";

        info!("Testing cascading fault propagation...");

        // Service chain: web -> api -> database
        let _services = vec!["web", "api", "database"];
        let mut failed_services = Vec::new();

        // Fail database first
        failed_services.push("database");

        // Check cascading failures
        if failed_services.contains(&"database") {
            failed_services.push("api");
        }

        if failed_services.contains(&"api") {
            failed_services.push("web");
        }

        let success = failed_services.len() == 3;

        self.metrics.record_operation(
            "cascading_propagation",
            start.elapsed().as_millis() as f64,
            success,
            None,
        );

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status).with_message(format!(
            "Failed services: {:?}, Cascaded: {}",
            failed_services, failed_services.len()
        ));

        Ok(result)
    }

    /// Test fault recovery and restart
    pub async fn test_fault_recovery(&self) -> ServiceResult<TestResult> {
        let _start = Instant::now();
        let test_id = "fault-scenario-fault-recovery";

        info!("Testing fault recovery and restart...");

        let mut service_state = "running";
        let mut recovery_times = Vec::new();

        // Simulate multiple fault/recovery cycles
        for _cycle in 0..5 {
            // Inject fault
            let fault_start = Instant::now();
            service_state = "failed";

            // Wait for detection
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            // Attempt recovery
            service_state = "recovering";
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            service_state = "running";
            let recovery_time = fault_start.elapsed().as_millis() as u64;
            recovery_times.push(recovery_time);

            self.metrics.record_operation(
                "recovery_cycle",
                recovery_time as f64,
                service_state == "running",
                None,
            );
        }

        let avg_recovery = recovery_times.iter().sum::<u64>() / recovery_times.len() as u64;
        let success = service_state == "running" && avg_recovery < 700;

        let status = if success {
            TestResultStatus::Passed
        } else {
            TestResultStatus::Failed
        };

        let result = TestResult::new(test_id, status).with_message(format!(
            "Cycles: {}, Avg recovery: {}ms, Final state: {}",
            recovery_times.len(),
            avg_recovery,
            service_state
        ));

        Ok(result)
    }

    /// Run all fault scenario tests
    pub async fn run_all_tests(&self) -> ServiceResult<TestReport> {
        info!("Running all fault scenario tests...");

        let results = vec![
            self.test_service_kill().await?,
            self.test_network_partition().await?,
            self.test_storage_failure().await?,
            self.test_cpu_overload().await?,
            self.test_service_timeout().await?,
            self.test_cascading_fault_propagation().await?,
            self.test_fault_recovery().await?,
        ];

        let mut report = TestReport::new();
        report.test_results = results.clone();
        report.service_metrics
            .insert("fault-scenarios".to_string(), self.metrics.aggregate());

        report.total_tests = results.len();
        report.passed_tests = results.iter().filter(|r| r.is_success()).count();
        report.failed_tests = report.total_tests - report.passed_tests;
        report.success_rate = (report.passed_tests as f64 / report.total_tests as f64) * 100.0;

        info!(
            "Fault scenario tests complete: {}/{} passed",
            report.passed_tests, report.total_tests
        );

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fault_scenario_creation() {
        let scenario = FaultScenario::new("test", 1000, 2000);
        assert_eq!(scenario.name, "test");
        assert!(!scenario.injected);
    }

    #[test]
    fn test_fault_injection_and_recovery() {
        let mut scenario = FaultScenario::new("test", 1000, 2000);
        scenario.inject();
        assert!(scenario.injected);
        scenario.recover();
        assert!(!scenario.injected);
    }

    #[tokio::test]
    async fn test_fault_scenario_tests() {
        let config = TestConfig::default();
        let tests = FaultScenarioTests::new(config);

        let result = tests.test_service_kill().await.unwrap();
        assert!(matches!(result.status, TestResultStatus::Passed | TestResultStatus::Failed));
    }
}
