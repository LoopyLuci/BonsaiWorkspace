//! Metrics collection and reporting for test execution
//!
//! Provides types for tracking resource usage, performance metrics,
//! and system behavior during test execution.

use serde::{Deserialize, Serialize};

/// Resource metrics captured during test execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceMetrics {
    /// CPU usage in percentage (0.0-100.0)
    pub cpu_usage_percent: f64,
    /// Memory usage in bytes
    pub memory_used_bytes: u64,
    /// Maximum memory used in bytes
    pub memory_peak_bytes: u64,
    /// Number of threads spawned
    pub thread_count: u32,
    /// Number of context switches
    pub context_switches: u64,
    /// I/O operations performed
    pub io_operations: u64,
    /// Bytes written to disk
    pub io_bytes_written: u64,
    /// Bytes read from disk
    pub io_bytes_read: u64,
}

impl ResourceMetrics {
    /// Create a new empty resource metrics snapshot
    pub fn new() -> Self {
        Self {
            cpu_usage_percent: 0.0,
            memory_used_bytes: 0,
            memory_peak_bytes: 0,
            thread_count: 0,
            context_switches: 0,
            io_operations: 0,
            io_bytes_written: 0,
            io_bytes_read: 0,
        }
    }

    /// Check if memory usage exceeds a limit
    pub fn memory_exceeds(&self, limit_bytes: u64) -> bool {
        self.memory_used_bytes > limit_bytes
    }

    /// Get memory usage as percentage of peak
    pub fn memory_usage_percent(&self) -> f64 {
        if self.memory_peak_bytes == 0 {
            return 0.0;
        }
        (self.memory_used_bytes as f64 / self.memory_peak_bytes as f64) * 100.0
    }

    /// Get total I/O bytes
    pub fn total_io_bytes(&self) -> u64 {
        self.io_bytes_written + self.io_bytes_read
    }
}

impl Default for ResourceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Timing metrics for test execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimingMetrics {
    /// Milliseconds spent in test setup
    pub setup_millis: u64,
    /// Milliseconds spent in actual test execution
    pub execution_millis: u64,
    /// Milliseconds spent in teardown/cleanup
    pub teardown_millis: u64,
    /// Milliseconds spent in fault injection
    pub fault_injection_millis: u64,
    /// Milliseconds spent in result collection
    pub result_collection_millis: u64,
}

impl TimingMetrics {
    /// Create new timing metrics
    pub fn new() -> Self {
        Self {
            setup_millis: 0,
            execution_millis: 0,
            teardown_millis: 0,
            fault_injection_millis: 0,
            result_collection_millis: 0,
        }
    }

    /// Get total execution time in milliseconds
    pub fn total_millis(&self) -> u64 {
        self.setup_millis
            + self.execution_millis
            + self.teardown_millis
            + self.fault_injection_millis
            + self.result_collection_millis
    }

    /// Get execution time as percentage of total
    pub fn execution_percent(&self) -> f64 {
        let total = self.total_millis();
        if total == 0 {
            return 0.0;
        }
        (self.execution_millis as f64 / total as f64) * 100.0
    }
}

impl Default for TimingMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// System behavior metrics during test execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorMetrics {
    /// Number of faults that were successfully injected
    pub faults_injected: u32,
    /// Number of faults from which the system recovered
    pub faults_recovered: u32,
    /// Number of faults that caused system failure
    pub faults_not_recovered: u32,
    /// Number of assertions that passed
    pub assertions_passed: u32,
    /// Number of assertions that failed
    pub assertions_failed: u32,
    /// Number of kernel calls made
    pub kernel_calls: u64,
    /// Number of times the system entered a specific state
    pub state_transitions: u32,
}

impl BehaviorMetrics {
    /// Create new behavior metrics
    pub fn new() -> Self {
        Self {
            faults_injected: 0,
            faults_recovered: 0,
            faults_not_recovered: 0,
            assertions_passed: 0,
            assertions_failed: 0,
            kernel_calls: 0,
            state_transitions: 0,
        }
    }

    /// Get total assertions
    pub fn total_assertions(&self) -> u32 {
        self.assertions_passed + self.assertions_failed
    }

    /// Get assertion pass rate as percentage
    pub fn assertion_pass_rate(&self) -> f64 {
        let total = self.total_assertions();
        if total == 0 {
            return 100.0;
        }
        (self.assertions_passed as f64 / total as f64) * 100.0
    }

    /// Get fault recovery rate as percentage
    pub fn fault_recovery_rate(&self) -> f64 {
        let total = self.faults_recovered + self.faults_not_recovered;
        if total == 0 {
            return 100.0;
        }
        (self.faults_recovered as f64 / total as f64) * 100.0
    }
}

impl Default for BehaviorMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive metrics for a single test execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestMetrics {
    /// Resource metrics
    pub resources: ResourceMetrics,
    /// Timing metrics
    pub timing: TimingMetrics,
    /// Behavior metrics
    pub behavior: BehaviorMetrics,
}

impl TestMetrics {
    /// Create new test metrics
    pub fn new() -> Self {
        Self {
            resources: ResourceMetrics::new(),
            timing: TimingMetrics::new(),
            behavior: BehaviorMetrics::new(),
        }
    }

    /// Validate metrics for reasonable values
    pub fn validate(&self) -> crate::errors::SrwstsResult<()> {
        use crate::errors::SrwstsError;

        // Validate CPU usage
        if self.resources.cpu_usage_percent < 0.0 || self.resources.cpu_usage_percent > 100.0 {
            return Err(SrwstsError::InvalidMetricValue {
                metric_name: "cpu_usage_percent".to_string(),
                value: self.resources.cpu_usage_percent.to_string(),
            });
        }

        // Validate memory
        if self.resources.memory_peak_bytes > 0 && self.resources.memory_used_bytes > self.resources.memory_peak_bytes {
            return Err(SrwstsError::MetricsValidationFailed {
                reason: "memory_used exceeds memory_peak".to_string(),
            });
        }

        // Validate timing
        if self.timing.total_millis() > 0 && self.timing.execution_percent() > 100.0 {
            return Err(SrwstsError::MetricsValidationFailed {
                reason: "execution_percent exceeds 100%".to_string(),
            });
        }

        // Validate fault metrics consistency
        if self.behavior.faults_recovered + self.behavior.faults_not_recovered > self.behavior.faults_injected {
            return Err(SrwstsError::MetricsValidationFailed {
                reason: "faults_recovered + faults_not_recovered exceeds faults_injected".to_string(),
            });
        }

        Ok(())
    }

    /// Get summary as JSON
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }
}

impl Default for TestMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_metrics_memory_exceeds() {
        let mut metrics = ResourceMetrics::new();
        metrics.memory_used_bytes = 2_000_000;
        assert!(metrics.memory_exceeds(1_000_000));
        assert!(!metrics.memory_exceeds(3_000_000));
    }

    #[test]
    fn test_resource_metrics_memory_percent() {
        let mut metrics = ResourceMetrics::new();
        metrics.memory_used_bytes = 500_000;
        metrics.memory_peak_bytes = 1_000_000;
        assert_eq!(metrics.memory_usage_percent(), 50.0);
    }

    #[test]
    fn test_timing_metrics_total() {
        let mut metrics = TimingMetrics::new();
        metrics.setup_millis = 100;
        metrics.execution_millis = 200;
        metrics.teardown_millis = 50;
        assert_eq!(metrics.total_millis(), 350);
    }

    #[test]
    fn test_timing_metrics_execution_percent() {
        let mut metrics = TimingMetrics::new();
        metrics.setup_millis = 100;
        metrics.execution_millis = 300;
        metrics.teardown_millis = 100;
        let percent = metrics.execution_percent();
        assert!(percent > 59.9 && percent < 60.1);
    }

    #[test]
    fn test_behavior_metrics_assertion_pass_rate() {
        let mut metrics = BehaviorMetrics::new();
        metrics.assertions_passed = 80;
        metrics.assertions_failed = 20;
        let rate = metrics.assertion_pass_rate();
        assert!(rate > 79.9 && rate < 80.1);
    }

    #[test]
    fn test_behavior_metrics_fault_recovery_rate() {
        let mut metrics = BehaviorMetrics::new();
        metrics.faults_recovered = 75;
        metrics.faults_not_recovered = 25;
        let rate = metrics.fault_recovery_rate();
        assert!(rate > 74.9 && rate < 75.1);
    }

    #[test]
    fn test_test_metrics_validate_cpu() {
        let mut metrics = TestMetrics::new();
        metrics.resources.cpu_usage_percent = 150.0;
        assert!(metrics.validate().is_err());
    }

    #[test]
    fn test_test_metrics_validate_memory() {
        let mut metrics = TestMetrics::new();
        metrics.resources.memory_peak_bytes = 100;
        metrics.resources.memory_used_bytes = 200;
        assert!(metrics.validate().is_err());
    }

    #[test]
    fn test_test_metrics_validate_success() {
        let mut metrics = TestMetrics::new();
        metrics.resources.cpu_usage_percent = 50.0;
        metrics.resources.memory_peak_bytes = 1_000_000;
        metrics.resources.memory_used_bytes = 500_000;
        assert!(metrics.validate().is_ok());
    }
}
