use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Metrics for a single profiled operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMetrics {
    pub operation: String,
    pub latency_ms: f64,
    pub memory_mb: f64,
    pub timestamp: DateTime<Utc>,
}

/// Latency percentiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyPercentiles {
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub p999: f64,
    pub max: f64,
}

/// Aggregate metrics for an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateMetrics {
    pub operation: String,
    pub count: usize,
    pub total_time_ms: f64,
    pub avg_time_ms: f64,
    pub min_time_ms: f64,
    pub max_time_ms: f64,
    pub percentiles: LatencyPercentiles,
    pub total_memory_mb: f64,
    pub avg_memory_mb: f64,
    pub max_memory_mb: f64,
}

impl AggregateMetrics {
    pub fn from_operations(operation: &str, metrics: &[OperationMetrics]) -> Self {
        let times: Vec<f64> = metrics.iter().map(|m| m.latency_ms).collect();
        let memories: Vec<f64> = metrics.iter().map(|m| m.memory_mb).collect();

        let mut sorted_times = times.clone();
        sorted_times.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let percentiles = LatencyPercentiles {
            p50: percentile(&sorted_times, 0.50),
            p95: percentile(&sorted_times, 0.95),
            p99: percentile(&sorted_times, 0.99),
            p999: percentile(&sorted_times, 0.999),
            max: *sorted_times.last().unwrap_or(&0.0),
        };

        Self {
            operation: operation.to_string(),
            count: metrics.len(),
            total_time_ms: times.iter().sum(),
            avg_time_ms: times.iter().sum::<f64>() / times.len().max(1) as f64,
            min_time_ms: times.iter().cloned().fold(f64::INFINITY, f64::min),
            max_time_ms: times.iter().cloned().fold(0.0, f64::max),
            percentiles,
            total_memory_mb: memories.iter().sum(),
            avg_memory_mb: memories.iter().sum::<f64>() / memories.len().max(1) as f64,
            max_memory_mb: memories.iter().cloned().fold(0.0, f64::max),
        }
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).ceil() as usize;
    sorted[idx.min(sorted.len() - 1)]
}
