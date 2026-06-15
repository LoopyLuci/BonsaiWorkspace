//! Performance metrics data structures

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub timestamp: String,
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
    pub disk_io_ops: u64,
    pub network_bandwidth_mbps: f64,
    pub cache_hit_ratio: f64,
    pub latency_ms: f64,
}

impl PerformanceMetrics {
    /// Calculate overall performance score (0-100)
    pub fn performance_score(&self) -> f64 {
        let cpu_score = (100.0 - self.cpu_usage_percent.min(100.0)) * 0.3;
        let memory_score = (100.0 - (self.memory_usage_mb / 8192.0 * 100.0).min(100.0)) * 0.2;
        let cache_score = self.cache_hit_ratio * 100.0 * 0.3;
        let latency_score = (100.0 - (self.latency_ms / 100.0 * 100.0).min(100.0)) * 0.2;

        cpu_score + memory_score + cache_score + latency_score
    }

    /// Identify performance bottlenecks
    pub fn identify_bottlenecks(&self) -> Vec<String> {
        let mut bottlenecks = Vec::new();

        if self.cpu_usage_percent > 80.0 {
            bottlenecks.push("High CPU usage".to_string());
        }
        if self.memory_usage_mb > 6000.0 {
            bottlenecks.push("High memory usage".to_string());
        }
        if self.cache_hit_ratio < 0.6 {
            bottlenecks.push("Low cache hit ratio".to_string());
        }
        if self.latency_ms > 50.0 {
            bottlenecks.push("High latency".to_string());
        }

        bottlenecks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_score() {
        let metrics = PerformanceMetrics {
            timestamp: "2026-06-10T00:00:00Z".to_string(),
            cpu_usage_percent: 50.0,
            memory_usage_mb: 2048.0,
            disk_io_ops: 1000,
            network_bandwidth_mbps: 100.0,
            cache_hit_ratio: 0.8,
            latency_ms: 10.0,
        };

        let score = metrics.performance_score();
        assert!(score > 0.0 && score <= 100.0);
    }

    #[test]
    fn test_identify_bottlenecks() {
        let metrics = PerformanceMetrics {
            timestamp: "2026-06-10T00:00:00Z".to_string(),
            cpu_usage_percent: 90.0,
            memory_usage_mb: 7000.0,
            disk_io_ops: 1000,
            network_bandwidth_mbps: 100.0,
            cache_hit_ratio: 0.5,
            latency_ms: 60.0,
        };

        let bottlenecks = metrics.identify_bottlenecks();
        assert!(!bottlenecks.is_empty());
    }
}
