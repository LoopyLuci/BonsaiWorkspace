use crate::{AggregatedMetrics, MetricValue, ObservabilityError, ObservabilityResult};
use chrono::Utc;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;

pub struct MetricsAggregator {
    metrics: Arc<DashMap<String, Vec<MetricValue>>>,
}

impl MetricsAggregator {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(DashMap::new()),
        }
    }

    pub async fn record_metric(
        &self,
        name: &str,
        value: f64,
        labels: HashMap<String, String>,
    ) -> ObservabilityResult<()> {
        if !value.is_finite() {
            return Err(ObservabilityError::InvalidMetric("Value must be finite".to_string()));
        }

        let metric = MetricValue {
            name: name.to_string(),
            value,
            timestamp: Utc::now(),
            labels,
            unit: "1".to_string(),
        };

        self.metrics
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(metric);

        Ok(())
    }

    pub async fn get_metrics(&self, name: &str) -> ObservabilityResult<Vec<MetricValue>> {
        self.metrics
            .get(name)
            .map(|entry| entry.clone())
            .ok_or_else(|| ObservabilityError::InvalidMetric(format!("Metric not found: {}", name)))
    }

    pub async fn aggregate_metrics(&self, name: &str) -> ObservabilityResult<AggregatedMetrics> {
        let metrics = self.get_metrics(name).await?;

        if metrics.is_empty() {
            return Err(ObservabilityError::MetricsAggregationFailed("No metrics to aggregate".to_string()));
        }

        let mut values: Vec<f64> = metrics.iter().map(|m| m.value).collect();
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let count = values.len() as u64;
        let sum: f64 = values.iter().sum();
        let min = values[0];
        let max = values[values.len() - 1];

        let p50 = Self::percentile(&values, 0.5);
        let p95 = Self::percentile(&values, 0.95);
        let p99 = Self::percentile(&values, 0.99);

        Ok(AggregatedMetrics {
            timestamp: Utc::now(),
            count,
            sum,
            min,
            max,
            p50,
            p95,
            p99,
        })
    }

    pub async fn flush_metrics(&self) -> ObservabilityResult<()> {
        self.metrics.clear();
        Ok(())
    }

    pub fn metric_count(&self) -> usize {
        self.metrics.iter().map(|entry| entry.value().len()).sum()
    }

    fn percentile(values: &[f64], p: f64) -> f64 {
        if values.is_empty() {
            return 0.0;
        }

        let index = (p * (values.len() - 1) as f64).ceil() as usize;
        values[index.min(values.len() - 1)]
    }
}

impl Default for MetricsAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_metric() {
        let aggregator = MetricsAggregator::new();
        aggregator.record_metric("latency_ms", 42.5, HashMap::new()).await.unwrap();

        assert_eq!(aggregator.metric_count(), 1);
    }

    #[tokio::test]
    async fn test_invalid_metric_value() {
        let aggregator = MetricsAggregator::new();
        let result = aggregator.record_metric("metric", f64::NAN, HashMap::new()).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_metrics() {
        let aggregator = MetricsAggregator::new();
        aggregator.record_metric("response_time", 100.0, HashMap::new()).await.unwrap();
        aggregator.record_metric("response_time", 150.0, HashMap::new()).await.unwrap();

        let metrics = aggregator.get_metrics("response_time").await.unwrap();
        assert_eq!(metrics.len(), 2);
    }

    #[tokio::test]
    async fn test_aggregate_metrics() {
        let aggregator = MetricsAggregator::new();

        for i in 1..=100 {
            aggregator
                .record_metric("requests", i as f64, HashMap::new())
                .await
                .unwrap();
        }

        let agg = aggregator.aggregate_metrics("requests").await.unwrap();
        assert_eq!(agg.count, 100);
        assert_eq!(agg.min, 1.0);
        assert_eq!(agg.max, 100.0);
        assert!(agg.p50 > 49.0 && agg.p50 <= 51.0);
    }

    #[tokio::test]
    async fn test_percentile_calculation() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];

        let p50 = MetricsAggregator::percentile(&values, 0.5);
        let p95 = MetricsAggregator::percentile(&values, 0.95);
        let p99 = MetricsAggregator::percentile(&values, 0.99);

        assert!(p50 >= 5.0);
        assert!(p95 >= 9.0);
        assert_eq!(p99, 10.0);
    }

    #[tokio::test]
    async fn test_flush_metrics() {
        let aggregator = MetricsAggregator::new();
        aggregator.record_metric("metric", 42.0, HashMap::new()).await.unwrap();

        assert!(aggregator.metric_count() > 0);

        aggregator.flush_metrics().await.unwrap();
        assert_eq!(aggregator.metric_count(), 0);
    }

    #[tokio::test]
    async fn test_get_nonexistent_metric() {
        let aggregator = MetricsAggregator::new();
        let result = aggregator.get_metrics("nonexistent").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_aggregate_empty_metrics() {
        let aggregator = MetricsAggregator::new();
        let result = aggregator.aggregate_metrics("nonexistent").await;

        assert!(result.is_err());
    }
}
