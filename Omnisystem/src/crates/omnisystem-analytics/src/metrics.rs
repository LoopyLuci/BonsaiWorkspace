/// Metrics Collection

use serde::{Deserialize, Serialize};

/// Metric Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricType {
    CpuUsage,
    MemoryUsage,
    NetworkLatency,
    RequestsPerSecond,
    ErrorRate,
    CacheHitRate,
    ProcessCount,
    ThreadCount,
}

/// Metric Point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub metric_type: MetricType,
    pub value: f64,
    pub timestamp: u64,
    pub tags: std::collections::HashMap<String, String>,
}

impl MetricPoint {
    pub fn new(metric_type: MetricType, value: f64) -> Self {
        MetricPoint {
            metric_type,
            value,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            tags: std::collections::HashMap::new(),
        }
    }

    pub fn with_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }
}

/// Aggregated Metrics
#[derive(Debug, Clone)]
pub struct Metrics {
    pub metric_type: MetricType,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub count: usize,
}

impl Metrics {
    pub fn calculate(points: &[MetricPoint]) -> Option<Self> {
        if points.is_empty() {
            return None;
        }

        let metric_type = points[0].metric_type;
        let values: Vec<f64> = points.iter().map(|p| p.value).collect();

        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let avg = values.iter().sum::<f64>() / values.len() as f64;

        Some(Metrics {
            metric_type,
            min,
            max,
            avg,
            count: points.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_point_creation() {
        let metric = MetricPoint::new(MetricType::CpuUsage, 45.5);
        assert_eq!(metric.metric_type, MetricType::CpuUsage);
        assert_eq!(metric.value, 45.5);
    }

    #[test]
    fn test_metrics_calculation() {
        let points = vec![
            MetricPoint::new(MetricType::CpuUsage, 10.0),
            MetricPoint::new(MetricType::CpuUsage, 20.0),
            MetricPoint::new(MetricType::CpuUsage, 30.0),
        ];

        let metrics = Metrics::calculate(&points).unwrap();
        assert_eq!(metrics.min, 10.0);
        assert_eq!(metrics.max, 30.0);
        assert_eq!(metrics.avg, 20.0);
        assert_eq!(metrics.count, 3);
    }
}
