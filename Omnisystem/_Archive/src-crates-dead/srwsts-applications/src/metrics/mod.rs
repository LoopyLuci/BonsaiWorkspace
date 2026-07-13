//! Comprehensive metrics collection for application stress testing

mod collectors;
mod memory;
mod performance;
mod ui;

pub use collectors::{MetricsCollector, MetricsSnapshot};
pub use memory::MemoryProfile;
pub use performance::PerformanceMetrics;
pub use ui::UIMetrics;

use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Comprehensive application metrics
#[derive(Debug, Clone)]
pub struct ApplicationMetrics {
    /// Response time metrics
    pub response_times: Arc<ResponseTimeMetrics>,
    /// Memory profiling
    pub memory: Arc<MemoryProfile>,
    /// UI responsiveness
    pub ui: Arc<UIMetrics>,
    /// Performance counters
    pub performance: Arc<PerformanceMetrics>,
    /// Custom metrics
    pub custom: Arc<DashMap<String, u64>>,
    /// Collection start time
    pub start_time: Instant,
}

impl ApplicationMetrics {
    /// Create a new metrics collection
    pub fn new() -> Self {
        Self {
            response_times: Arc::new(ResponseTimeMetrics::new()),
            memory: Arc::new(MemoryProfile::new()),
            ui: Arc::new(UIMetrics::new()),
            performance: Arc::new(PerformanceMetrics::new()),
            custom: Arc::new(DashMap::new()),
            start_time: Instant::now(),
        }
    }

    /// Record a response time measurement
    pub fn record_response_time(&self, operation: impl Into<String>, duration: Duration) {
        self.response_times.record(operation, duration);
    }

    /// Record memory usage
    pub fn record_memory(&self, label: impl Into<String>, bytes: u64) {
        self.memory.record(label, bytes);
    }

    /// Record UI frame time
    pub fn record_frame_time(&self, duration: Duration) {
        self.ui.record_frame_time(duration);
    }

    /// Record input latency
    pub fn record_input_latency(&self, duration: Duration) {
        self.ui.record_input_latency(duration);
    }

    /// Record compilation time
    pub fn record_compilation(&self, duration: Duration) {
        self.performance.record_compilation(duration);
    }

    /// Record task completion
    pub fn record_task_completion(&self, duration: Duration) {
        self.performance.record_task_completion(duration);
    }

    /// Increment a custom metric
    pub fn increment_metric(&self, name: impl Into<String>) {
        let name_str = name.into();
        self.custom
            .entry(name_str)
            .and_modify(|v| *v += 1)
            .or_insert(1);
    }

    /// Get a custom metric value
    pub fn get_metric(&self, name: &str) -> Option<u64> {
        self.custom.get(name).map(|v| *v)
    }

    /// Generate metrics summary
    pub fn summary(&self) -> MetricsSummary {
        MetricsSummary {
            elapsed_secs: self.start_time.elapsed().as_secs_f64(),
            response_times: self.response_times.summary(),
            memory: self.memory.summary(),
            ui: self.ui.summary(),
            performance: self.performance.summary(),
        }
    }
}

impl Default for ApplicationMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Response time metrics
#[derive(Debug, Clone)]
pub struct ResponseTimeMetrics {
    measurements: Arc<DashMap<String, Vec<Duration>>>,
}

impl ResponseTimeMetrics {
    /// Create a new response time metrics collector
    pub fn new() -> Self {
        Self {
            measurements: Arc::new(DashMap::new()),
        }
    }

    /// Record a response time
    pub fn record(&self, operation: impl Into<String>, duration: Duration) {
        let op_str = operation.into();
        self.measurements
            .entry(op_str)
            .or_insert_with(Vec::new)
            .push(duration);
    }

    /// Get percentile response time
    pub fn percentile(&self, operation: &str, percentile: f64) -> Option<Duration> {
        self.measurements
            .get(operation)
            .and_then(|measurements| {
                if measurements.is_empty() {
                    return None;
                }

                let mut sorted = measurements.clone();
                sorted.sort();

                let index = ((percentile / 100.0) * sorted.len() as f64) as usize;
                Some(sorted[index.min(sorted.len() - 1)])
            })
    }

    /// Generate summary
    pub fn summary(&self) -> ResponseTimeSummary {
        let mut ops = Vec::new();
        for entry in self.measurements.iter() {
            let measurements = entry.value();
            if !measurements.is_empty() {
                let count = measurements.len();
                let total: Duration = measurements.iter().sum();
                let avg = total / count as u32;

                ops.push((
                    entry.key().clone(),
                    count,
                    avg,
                    self.percentile(entry.key(), 95.0).unwrap_or_default(),
                    self.percentile(entry.key(), 99.0).unwrap_or_default(),
                ));
            }
        }
        ResponseTimeSummary { operations: ops }
    }
}

impl Default for ResponseTimeMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Response time summary
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResponseTimeSummary {
    pub operations: Vec<(String, usize, Duration, Duration, Duration)>,
}

/// Metrics summary
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricsSummary {
    pub elapsed_secs: f64,
    pub response_times: ResponseTimeSummary,
    pub memory: MemorySummary,
    pub ui: UISummary,
    pub performance: PerformanceSummary,
}

/// Memory summary
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemorySummary {
    pub peak_mb: u64,
    pub average_mb: u64,
}

/// UI metrics summary
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UISummary {
    pub avg_frame_time_ms: f64,
    pub p99_frame_time_ms: f64,
    pub avg_input_latency_ms: f64,
}

/// Performance summary
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerformanceSummary {
    pub compilations: u64,
    pub avg_compilation_ms: f64,
    pub tasks_completed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = ApplicationMetrics::new();
        assert_eq!(metrics.start_time.elapsed().as_millis(), 0 as u128);
    }

    #[test]
    fn test_response_time_recording() {
        let metrics = ApplicationMetrics::new();
        metrics.record_response_time("query", Duration::from_millis(100));
        metrics.record_response_time("query", Duration::from_millis(150));

        let summary = metrics.response_times.summary();
        assert_eq!(summary.operations.len(), 1);
        assert_eq!(summary.operations[0].1, 2); // 2 measurements
    }

    #[test]
    fn test_memory_recording() {
        let metrics = ApplicationMetrics::new();
        metrics.record_memory("heap", 1024 * 1024);
        metrics.record_memory("stack", 512 * 1024);

        let summary = metrics.memory.summary();
        assert!(summary.peak_mb > 0);
    }

    #[test]
    fn test_custom_metrics() {
        let metrics = ApplicationMetrics::new();
        metrics.increment_metric("requests");
        metrics.increment_metric("requests");
        metrics.increment_metric("errors");

        assert_eq!(metrics.get_metric("requests"), Some(2));
        assert_eq!(metrics.get_metric("errors"), Some(1));
    }

    #[test]
    fn test_metrics_summary() {
        let metrics = ApplicationMetrics::new();
        metrics.record_response_time("op", Duration::from_millis(100));
        metrics.record_memory("mem", 1024);

        let summary = metrics.summary();
        assert!(summary.elapsed_secs >= 0.0);
    }
}
