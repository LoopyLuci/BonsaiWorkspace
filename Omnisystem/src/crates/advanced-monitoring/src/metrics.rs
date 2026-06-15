use crate::{Metric, MonitoringError, MonitoringResult};
use dashmap::DashMap;
use std::sync::Arc;

pub struct MetricsCollector {
    metrics: Arc<DashMap<String, Vec<Metric>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(DashMap::new()),
        }
    }

    pub async fn record(&self, metric: &Metric) -> MonitoringResult<()> {
        self.metrics
            .entry(metric.metric_name.clone())
            .or_insert_with(Vec::new)
            .push(metric.clone());
        Ok(())
    }

    pub async fn get_metrics(&self, name: &str) -> MonitoringResult<Vec<Metric>> {
        if let Some(metrics) = self.metrics.get(name) {
            Ok(metrics.clone())
        } else {
            Err(MonitoringError::MetricNotFound)
        }
    }

    pub fn metric_count(&self) -> usize {
        self.metrics.len()
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
    use chrono::Utc;

    #[tokio::test]
    async fn test_record_metric() {
        let collector = MetricsCollector::new();
        let metric = Metric {
            metric_name: "cpu_usage".to_string(),
            value: 45.5,
            timestamp: Utc::now(),
            labels: std::collections::HashMap::new(),
        };

        collector.record(&metric).await.unwrap();
        assert_eq!(collector.metric_count(), 1);
    }

    #[tokio::test]
    async fn test_get_metrics() {
        let collector = MetricsCollector::new();
        let metric = Metric {
            metric_name: "cpu_usage".to_string(),
            value: 45.5,
            timestamp: Utc::now(),
            labels: std::collections::HashMap::new(),
        };

        collector.record(&metric).await.unwrap();
        let metrics = collector.get_metrics("cpu_usage").await.unwrap();
        assert_eq!(metrics.len(), 1);
    }
}
