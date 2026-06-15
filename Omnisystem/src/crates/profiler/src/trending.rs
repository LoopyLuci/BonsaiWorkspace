use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Performance trend over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrend {
    pub operation: String,
    pub date: DateTime<Utc>,
    pub avg_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub avg_memory_mb: f64,
}

impl PerformanceTrend {
    pub fn new(operation: &str, avg_latency_ms: f64, p99_latency_ms: f64, avg_memory_mb: f64) -> Self {
        Self {
            operation: operation.to_string(),
            date: Utc::now(),
            avg_latency_ms,
            p99_latency_ms,
            avg_memory_mb,
        }
    }

    /// Calculate trend direction (positive = degradation)
    pub fn trend_vs(&self, other: &PerformanceTrend) -> f64 {
        if self.operation != other.operation {
            return 0.0;
        }
        (self.avg_latency_ms - other.avg_latency_ms) / other.avg_latency_ms.max(0.001) * 100.0
    }

    /// Check if performance degraded significantly
    pub fn is_degraded(&self, other: &PerformanceTrend, threshold_percent: f64) -> bool {
        self.trend_vs(other) > threshold_percent
    }
}
