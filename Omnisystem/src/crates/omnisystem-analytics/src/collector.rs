/// Metrics Collector

use crate::metrics::{MetricPoint, MetricType, Metrics};
use dashmap::DashMap;
use std::sync::Arc;

/// Metrics Collector
pub struct MetricsCollector {
    metrics: Arc<DashMap<MetricType, Vec<MetricPoint>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        MetricsCollector {
            metrics: Arc::new(DashMap::new()),
        }
    }

    pub fn record(&self, point: MetricPoint) {
        self.metrics
            .entry(point.metric_type)
            .or_insert_with(Vec::new)
            .push(point);
    }

    pub fn get_metrics(&self, metric_type: MetricType) -> Vec<MetricPoint> {
        self.metrics
            .get(&metric_type)
            .map(|points| points.clone())
            .unwrap_or_default()
    }

    pub fn get_aggregated(&self, metric_type: MetricType) -> Option<Metrics> {
        let points = self.get_metrics(metric_type);
        Metrics::calculate(&points)
    }

    pub fn count(&self) -> usize {
        self.metrics.iter().map(|entry| entry.value().len()).sum()
    }

    pub fn clear(&self) {
        self.metrics.clear();
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collector() {
        let collector = MetricsCollector::new();
        let point = MetricPoint::new(MetricType::CpuUsage, 50.0);
        collector.record(point);

        let metrics = collector.get_metrics(MetricType::CpuUsage);
        assert_eq!(metrics.len(), 1);
    }
}
