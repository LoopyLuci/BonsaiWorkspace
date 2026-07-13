//! Metrics collection and analysis

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Performance metrics from test execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// CPU time in milliseconds
    pub cpu_time_ms: u64,
    /// Wall clock time in milliseconds
    pub wall_time_ms: u64,
    /// Peak memory usage in bytes
    pub peak_memory_bytes: u64,
    /// Average memory usage in bytes
    pub avg_memory_bytes: u64,
    /// Number of context switches
    pub context_switches: u64,
    /// Number of page faults
    pub page_faults: u64,
    /// I/O operations count
    pub io_operations: u64,
    /// Bytes read
    pub bytes_read: u64,
    /// Bytes written
    pub bytes_written: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Branch mispredictions
    pub branch_mispredictions: u64,
    /// Custom metrics
    pub custom: HashMap<String, f64>,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            cpu_time_ms: 0,
            wall_time_ms: 0,
            peak_memory_bytes: 0,
            avg_memory_bytes: 0,
            context_switches: 0,
            page_faults: 0,
            io_operations: 0,
            bytes_read: 0,
            bytes_written: 0,
            cache_misses: 0,
            branch_mispredictions: 0,
            custom: HashMap::new(),
        }
    }
}

impl PerformanceMetrics {
    /// Create new metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate CPU utilization as percentage
    pub fn cpu_utilization(&self) -> f64 {
        if self.wall_time_ms == 0 {
            return 0.0;
        }
        (self.cpu_time_ms as f64 / self.wall_time_ms as f64) * 100.0
    }

    /// Calculate I/O throughput in MB/s
    pub fn io_throughput_mbps(&self) -> f64 {
        let total_bytes = self.bytes_read + self.bytes_written;
        if self.wall_time_ms == 0 {
            return 0.0;
        }
        (total_bytes as f64 / 1024.0 / 1024.0) / (self.wall_time_ms as f64 / 1000.0)
    }

    /// Get memory efficiency (avg / peak)
    pub fn memory_efficiency(&self) -> f64 {
        if self.peak_memory_bytes == 0 {
            return 100.0;
        }
        (self.avg_memory_bytes as f64 / self.peak_memory_bytes as f64) * 100.0
    }

    /// Set custom metric
    pub fn set_custom(&mut self, key: String, value: f64) {
        self.custom.insert(key, value);
    }

    /// Get custom metric
    pub fn get_custom(&self, key: &str) -> Option<f64> {
        self.custom.get(key).copied()
    }
}

/// Metric sample for time-series data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSample {
    /// Timestamp
    pub timestamp_us: u64,
    /// Metric name
    pub name: String,
    /// Metric value
    pub value: f64,
    /// Optional tags
    pub tags: HashMap<String, String>,
}

impl MetricSample {
    /// Create a new metric sample
    pub fn new(timestamp_us: u64, name: String, value: f64) -> Self {
        Self {
            timestamp_us,
            name,
            value,
            tags: HashMap::new(),
        }
    }

    /// Add a tag
    pub fn with_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }
}

/// Metrics collector for aggregating test metrics
pub struct MetricsCollector {
    /// Metrics per test
    metrics: HashMap<String, Vec<PerformanceMetrics>>,
    /// Time-series samples
    samples: Vec<MetricSample>,
    /// Total tests collected
    total_tests: u64,
    /// Passed tests
    passed_tests: u64,
    /// Failed tests
    failed_tests: u64,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
            samples: Vec::new(),
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
        }
    }

    /// Collect metrics from a test
    pub fn collect(&mut self, test_name: String, metrics: PerformanceMetrics) {
        self.metrics
            .entry(test_name)
            .or_insert_with(Vec::new)
            .push(metrics);
        self.total_tests += 1;
    }

    /// Record test result
    pub fn record_result(&mut self, passed: bool) {
        if passed {
            self.passed_tests += 1;
        } else {
            self.failed_tests += 1;
        }
    }

    /// Add a metric sample
    pub fn add_sample(&mut self, sample: MetricSample) {
        self.samples.push(sample);
    }

    /// Get metrics for a test
    pub fn get_metrics(&self, test_name: &str) -> Option<&Vec<PerformanceMetrics>> {
        self.metrics.get(test_name)
    }

    /// Get average metrics for a test
    pub fn average_metrics(&self, test_name: &str) -> Option<PerformanceMetrics> {
        let metrics = self.metrics.get(test_name)?;
        if metrics.is_empty() {
            return None;
        }

        let count = metrics.len() as u64;
        let cpu_sum: u64 = metrics.iter().map(|m| m.cpu_time_ms).sum();
        let wall_sum: u64 = metrics.iter().map(|m| m.wall_time_ms).sum();
        let memory_sum: u64 = metrics.iter().map(|m| m.peak_memory_bytes).sum();

        Some(PerformanceMetrics {
            cpu_time_ms: cpu_sum / count,
            wall_time_ms: wall_sum / count,
            peak_memory_bytes: memory_sum / count,
            ..Default::default()
        })
    }

    /// Get total tests
    pub fn total_tests(&self) -> u64 {
        self.total_tests
    }

    /// Get passed tests
    pub fn passed_tests(&self) -> u64 {
        self.passed_tests
    }

    /// Get failed tests
    pub fn failed_tests(&self) -> u64 {
        self.failed_tests
    }

    /// Get test statistics
    pub fn statistics(&self) -> MetricsStatistics {
        let pass_rate = if self.total_tests == 0 {
            100.0
        } else {
            (self.passed_tests as f64 / self.total_tests as f64) * 100.0
        };

        MetricsStatistics {
            total_tests: self.total_tests,
            passed_tests: self.passed_tests,
            failed_tests: self.failed_tests,
            pass_rate,
        }
    }

    /// Reset all metrics
    pub fn reset(&mut self) {
        self.metrics.clear();
        self.samples.clear();
        self.total_tests = 0;
        self.passed_tests = 0;
        self.failed_tests = 0;
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Aggregated metrics statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsStatistics {
    /// Total tests
    pub total_tests: u64,
    /// Passed tests
    pub passed_tests: u64,
    /// Failed tests
    pub failed_tests: u64,
    /// Pass rate percentage
    pub pass_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_metrics_creation() {
        let metrics = PerformanceMetrics::new();
        assert_eq!(metrics.cpu_time_ms, 0);
    }

    #[test]
    fn test_performance_metrics_cpu_utilization() {
        let mut metrics = PerformanceMetrics::new();
        metrics.cpu_time_ms = 50;
        metrics.wall_time_ms = 100;

        assert_eq!(metrics.cpu_utilization(), 50.0);
    }

    #[test]
    fn test_performance_metrics_io_throughput() {
        let mut metrics = PerformanceMetrics::new();
        metrics.bytes_read = 1024 * 1024; // 1 MB
        metrics.wall_time_ms = 1000;      // 1 second

        assert_eq!(metrics.io_throughput_mbps(), 1.0);
    }

    #[test]
    fn test_performance_metrics_custom() {
        let mut metrics = PerformanceMetrics::new();
        metrics.set_custom("latency_ms".to_string(), 42.5);

        assert_eq!(metrics.get_custom("latency_ms"), Some(42.5));
    }

    #[test]
    fn test_metric_sample_creation() {
        let sample = MetricSample::new(1000, "cpu_usage".to_string(), 45.2);
        assert_eq!(sample.name, "cpu_usage");
        assert_eq!(sample.value, 45.2);
    }

    #[test]
    fn test_metrics_collector_collection() {
        let mut collector = MetricsCollector::new();
        let metrics = PerformanceMetrics::new();

        collector.collect("test1".to_string(), metrics);
        assert_eq!(collector.total_tests(), 1);
    }

    #[test]
    fn test_metrics_collector_statistics() {
        let mut collector = MetricsCollector::new();

        for i in 0..10 {
            let metrics = PerformanceMetrics::new();
            collector.collect(format!("test{}", i), metrics);
            collector.record_result(i < 8); // 8 passed, 2 failed
        }

        let stats = collector.statistics();
        assert_eq!(stats.total_tests, 10);
        assert_eq!(stats.passed_tests, 8);
        assert_eq!(stats.pass_rate, 80.0);
    }

    #[test]
    fn test_metrics_collector_average() {
        let mut collector = MetricsCollector::new();
        let mut metrics1 = PerformanceMetrics::new();
        metrics1.cpu_time_ms = 100;
        metrics1.wall_time_ms = 200;

        let mut metrics2 = PerformanceMetrics::new();
        metrics2.cpu_time_ms = 200;
        metrics2.wall_time_ms = 400;

        collector.collect("test".to_string(), metrics1);
        collector.collect("test".to_string(), metrics2);

        let avg = collector.average_metrics("test").unwrap();
        assert_eq!(avg.cpu_time_ms, 150);
        assert_eq!(avg.wall_time_ms, 300);
    }
}
