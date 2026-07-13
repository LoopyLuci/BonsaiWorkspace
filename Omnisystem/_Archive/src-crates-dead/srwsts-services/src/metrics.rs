//! Metrics collection and analysis

use crate::types::ServiceId;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Per-operation metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMetric {
    pub latency_ms: f64,
    pub timestamp_ms: u64,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Service resource metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceMetrics {
    pub cpu_percent: f64,
    pub memory_mb: u64,
    pub memory_percent: f64,
    pub threads: usize,
    pub file_descriptors: usize,
    pub gpu_memory_mb: Option<u64>,
}

/// Service latency statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LatencyStats {
    pub min_ms: f64,
    pub max_ms: f64,
    pub mean_ms: f64,
    pub median_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

impl LatencyStats {
    pub fn from_latencies(mut latencies: Vec<f64>) -> Self {
        if latencies.is_empty() {
            return Self::default();
        }

        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let min = *latencies.first().unwrap_or(&0.0);
        let max = *latencies.last().unwrap_or(&0.0);
        let mean = latencies.iter().sum::<f64>() / latencies.len() as f64;

        let median_idx = latencies.len() / 2;
        let median = if latencies.len() % 2 == 0 {
            (latencies[median_idx - 1] + latencies[median_idx]) / 2.0
        } else {
            latencies[median_idx]
        };

        let p95_idx = (latencies.len() as f64 * 0.95) as usize;
        let p95 = latencies.get(p95_idx).copied().unwrap_or(max);

        let p99_idx = (latencies.len() as f64 * 0.99) as usize;
        let p99 = latencies.get(p99_idx).copied().unwrap_or(max);

        Self {
            min_ms: min,
            max_ms: max,
            mean_ms: mean,
            median_ms: median,
            p95_ms: p95,
            p99_ms: p99,
        }
    }
}

/// Throughput statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThroughputStats {
    pub ops_per_sec: f64,
    pub bytes_per_sec: u64,
    pub total_ops: u64,
    pub total_bytes: u64,
}

/// Error statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorStats {
    pub total_errors: u64,
    pub error_rate: f64,
    pub last_error: Option<String>,
    pub error_types: HashMap<String, u64>,
}

/// Aggregated service metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceMetrics {
    pub service_id: String,
    pub latency: LatencyStats,
    pub throughput: ThroughputStats,
    pub errors: ErrorStats,
    pub resources: ResourceMetrics,
    pub total_operations: u64,
    pub uptime_ms: u64,
}

impl ServiceMetrics {
    pub fn new(service_id: impl Into<String>) -> Self {
        Self {
            service_id: service_id.into(),
            ..Default::default()
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_operations == 0 {
            return 100.0;
        }
        let successes = self.total_operations - self.errors.total_errors;
        (successes as f64 / self.total_operations as f64) * 100.0
    }
}

/// Collects and aggregates metrics for a service
pub struct ServiceMetricsCollector {
    service_id: ServiceId,
    metrics: Arc<DashMap<String, Vec<OperationMetric>>>,
    resource_samples: Arc<DashMap<u64, ResourceMetrics>>,
    start_time: Instant,
}

impl ServiceMetricsCollector {
    pub fn new(service_id: impl Into<String>) -> Self {
        Self {
            service_id: ServiceId::new(service_id),
            metrics: Arc::new(DashMap::new()),
            resource_samples: Arc::new(DashMap::new()),
            start_time: Instant::now(),
        }
    }

    /// Record an operation metric
    pub fn record_operation(
        &self,
        operation: impl Into<String>,
        latency_ms: f64,
        success: bool,
        error: Option<String>,
    ) {
        let metric = OperationMetric {
            latency_ms,
            timestamp_ms: self.elapsed_ms(),
            success,
            error_message: error,
        };

        self.metrics
            .entry(operation.into())
            .or_insert_with(Vec::new)
            .push(metric);
    }

    /// Record resource usage sample
    pub fn record_resources(&self, resources: ResourceMetrics) {
        self.resource_samples
            .insert(self.elapsed_ms(), resources);
    }

    /// Get aggregated metrics
    pub fn aggregate(&self) -> ServiceMetrics {
        let mut metrics = ServiceMetrics::new(self.service_id.as_str());
        metrics.uptime_ms = self.elapsed_ms();

        let mut all_latencies = Vec::new();
        let mut total_ops = 0u64;
        let mut total_errors = 0u64;
        let mut error_types = HashMap::new();

        for ref_multi in self.metrics.iter() {
            let op_metrics = ref_multi.value();
            for metric in op_metrics {
                total_ops += 1;
                all_latencies.push(metric.latency_ms);

                if !metric.success {
                    total_errors += 1;
                    if let Some(err) = &metric.error_message {
                        *error_types.entry(err.clone()).or_insert(0) += 1;
                    }
                }
            }
        }

        metrics.total_operations = total_ops;
        metrics.latency = LatencyStats::from_latencies(all_latencies);
        metrics.errors.total_errors = total_errors;
        metrics.errors.error_rate = if total_ops > 0 {
            (total_errors as f64 / total_ops as f64) * 100.0
        } else {
            0.0
        };
        metrics.errors.error_types = error_types;

        // Calculate average resource usage
        if !self.resource_samples.is_empty() {
            let samples: Vec<_> = self.resource_samples.iter().map(|r| r.value().clone()).collect();
            if !samples.is_empty() {
                let sample_count = samples.len() as f64;
                metrics.resources.cpu_percent =
                    samples.iter().map(|s| s.cpu_percent).sum::<f64>() / sample_count;
                metrics.resources.memory_mb =
                    (samples.iter().map(|s| s.memory_mb).sum::<u64>() as f64 / sample_count) as u64;
                metrics.resources.memory_percent =
                    samples.iter().map(|s| s.memory_percent).sum::<f64>() / sample_count;
            }
        }

        metrics
    }

    fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    /// Clear all collected metrics
    pub fn clear(&self) {
        self.metrics.clear();
        self.resource_samples.clear();
    }

    /// Get operation count for a specific operation type
    pub fn operation_count(&self, operation: &str) -> usize {
        self.metrics
            .get(operation)
            .map(|metrics| metrics.len())
            .unwrap_or(0)
    }

    /// Get error count for a specific operation type
    pub fn error_count(&self, operation: &str) -> usize {
        self.metrics
            .get(operation)
            .map(|metrics| metrics.iter().filter(|m| !m.success).count())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_stats() {
        let latencies = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let stats = LatencyStats::from_latencies(latencies);
        assert_eq!(stats.min_ms, 10.0);
        assert_eq!(stats.max_ms, 50.0);
        assert_eq!(stats.median_ms, 30.0);
    }

    #[test]
    fn test_metrics_collector() {
        let collector = ServiceMetricsCollector::new("test-svc");
        collector.record_operation("read", 10.0, true, None);
        collector.record_operation("read", 15.0, true, None);
        collector.record_operation("read", 20.0, false, Some("error".to_string()));

        let metrics = collector.aggregate();
        assert_eq!(metrics.total_operations, 3);
        assert_eq!(metrics.errors.total_errors, 1);
        assert!(metrics.success_rate() > 60.0 && metrics.success_rate() < 70.0);
    }

    #[test]
    fn test_service_metrics_success_rate() {
        let mut metrics = ServiceMetrics::new("test");
        metrics.total_operations = 100;
        metrics.errors.total_errors = 10;
        assert_eq!(metrics.success_rate(), 90.0);
    }

    #[test]
    fn test_collector_operation_count() {
        let collector = ServiceMetricsCollector::new("test");
        collector.record_operation("op1", 10.0, true, None);
        collector.record_operation("op1", 20.0, true, None);
        collector.record_operation("op2", 30.0, true, None);

        assert_eq!(collector.operation_count("op1"), 2);
        assert_eq!(collector.operation_count("op2"), 1);
        assert_eq!(collector.error_count("op1"), 0);
    }
}
