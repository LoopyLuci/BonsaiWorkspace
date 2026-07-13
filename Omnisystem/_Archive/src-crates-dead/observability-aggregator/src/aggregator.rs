use crate::{
    AggregationPeriod, AggregationStats, AggregationWindow, AggregatorError, AggregatorResult, MetricTimeSeries,
    TimeSeriesPoint,
};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct MetricsAggregator {
    time_series: Arc<DashMap<String, MetricTimeSeries>>,
    config: crate::AggregatorConfig,
    stats: Arc<parking_lot::Mutex<AggregationStats>>,
}

impl MetricsAggregator {
    pub fn new(config: crate::AggregatorConfig) -> Self {
        let now = Utc::now();
        Self {
            time_series: Arc::new(DashMap::new()),
            config,
            stats: Arc::new(parking_lot::Mutex::new(AggregationStats {
                total_metrics: 0,
                total_windows: 0,
                total_data_points: 0,
                last_rotation: now,
                uptime_secs: 0,
            })),
        }
    }

    pub async fn record_metric(
        &self,
        metric_name: &str,
        value: f64,
        labels: std::collections::HashMap<String, String>,
    ) -> AggregatorResult<()> {
        if !value.is_finite() {
            return Err(AggregatorError::AggregationFailed("Value must be finite".to_string()));
        }

        let point = TimeSeriesPoint {
            timestamp: Utc::now(),
            value,
            labels,
        };

        let is_new = !self.time_series.contains_key(metric_name);

        let mut series = self
            .time_series
            .entry(metric_name.to_string())
            .or_insert_with(|| MetricTimeSeries {
                metric_name: metric_name.to_string(),
                windows: vec![],
                retention_hours: self.config.retention_hours,
            });

        if is_new {
            let mut stats = self.stats.lock();
            stats.total_metrics += 1;
        }

        if series.windows.is_empty() {
            let now = Utc::now();
            series.windows.push(AggregationWindow {
                period: AggregationPeriod::OneMinute,
                window_start: now,
                window_end: now,
                data_points: vec![],
                count: 0,
                sum: 0.0,
                min: f64::INFINITY,
                max: f64::NEG_INFINITY,
                mean: 0.0,
            });
        }

        if let Some(window) = series.windows.last_mut() {
            window.data_points.push(point.clone());
            window.count += 1;
            window.sum += value;
            window.min = window.min.min(value);
            window.max = window.max.max(value);
            window.mean = window.sum / window.count as f64;

            let mut stats = self.stats.lock();
            stats.total_data_points += 1;
        }

        Ok(())
    }

    pub async fn get_time_series(&self, metric_name: &str) -> AggregatorResult<MetricTimeSeries> {
        self.time_series
            .get(metric_name)
            .map(|entry| entry.clone())
            .ok_or_else(|| AggregatorError::MetricNotFound(metric_name.to_string()))
    }

    pub async fn rotate_window(&self, metric_name: &str) -> AggregatorResult<()> {
        if let Some(mut series) = self.time_series.get_mut(metric_name) {
            let now = Utc::now();

            if !series.windows.is_empty() {
                if let Some(last_window) = series.windows.last_mut() {
                    last_window.window_end = now;
                }
            }

            let new_window = AggregationWindow {
                period: AggregationPeriod::OneMinute,
                window_start: now,
                window_end: now,
                data_points: vec![],
                count: 0,
                sum: 0.0,
                min: f64::INFINITY,
                max: f64::NEG_INFINITY,
                mean: 0.0,
            };

            series.windows.push(new_window);

            if series.windows.len() > self.config.max_windows_per_metric {
                series.windows.remove(0);
            }

            let mut stats = self.stats.lock();
            stats.last_rotation = now;
            stats.total_windows += 1;
        }

        Ok(())
    }

    pub async fn aggregate_all(&self) -> AggregatorResult<std::collections::HashMap<String, crate::AggregationWindow>> {
        let mut result = std::collections::HashMap::new();

        for entry in self.time_series.iter() {
            if let Some(series) = entry.value().windows.last() {
                result.insert(entry.key().clone(), series.clone());
            }
        }

        Ok(result)
    }

    pub async fn get_stats(&self) -> AggregatorResult<AggregationStats> {
        let stats = self.stats.lock();
        Ok(stats.clone())
    }

    pub fn metric_count(&self) -> usize {
        self.time_series.len()
    }

    pub fn window_count(&self) -> usize {
        self.time_series.iter().map(|entry| entry.windows.len()).sum()
    }
}

impl Default for MetricsAggregator {
    fn default() -> Self {
        Self::new(crate::AggregatorConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_record_metric() {
        let aggregator = MetricsAggregator::default();
        let labels = HashMap::new();

        aggregator.record_metric("request_latency", 42.5, labels).await.unwrap();
        assert_eq!(aggregator.metric_count(), 1);
    }

    #[tokio::test]
    async fn test_get_time_series() {
        let aggregator = MetricsAggregator::default();
        let labels = HashMap::new();

        aggregator.record_metric("response_time", 100.0, labels).await.unwrap();

        let series = aggregator.get_time_series("response_time").await.unwrap();
        assert_eq!(series.metric_name, "response_time");
    }

    #[tokio::test]
    async fn test_record_multiple_values() {
        let aggregator = MetricsAggregator::default();

        for i in 1..=10 {
            aggregator
                .record_metric("metric", i as f64 * 10.0, HashMap::new())
                .await
                .unwrap();
        }

        let series = aggregator.get_time_series("metric").await.unwrap();
        let window = &series.windows[0];

        assert_eq!(window.count, 10);
        assert_eq!(window.sum, 550.0);
        assert_eq!(window.min, 10.0);
        assert_eq!(window.max, 100.0);
    }

    #[tokio::test]
    async fn test_window_rotation() {
        let aggregator = MetricsAggregator::default();

        aggregator.record_metric("metric", 42.0, HashMap::new()).await.unwrap();
        aggregator.rotate_window("metric").await.unwrap();
        aggregator.record_metric("metric", 50.0, HashMap::new()).await.unwrap();

        let series = aggregator.get_time_series("metric").await.unwrap();
        assert_eq!(series.windows.len(), 2);
    }

    #[tokio::test]
    async fn test_aggregate_all() {
        let aggregator = MetricsAggregator::default();

        aggregator.record_metric("metric1", 10.0, HashMap::new()).await.unwrap();
        aggregator.record_metric("metric2", 20.0, HashMap::new()).await.unwrap();

        let aggregated = aggregator.aggregate_all().await.unwrap();
        assert_eq!(aggregated.len(), 2);
    }

    #[tokio::test]
    async fn test_invalid_value() {
        let aggregator = MetricsAggregator::default();

        let result = aggregator.record_metric("metric", f64::NAN, HashMap::new()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_stats() {
        let aggregator = MetricsAggregator::default();

        aggregator.record_metric("metric1", 10.0, HashMap::new()).await.unwrap();
        aggregator.record_metric("metric2", 20.0, HashMap::new()).await.unwrap();

        let stats = aggregator.get_stats().await.unwrap();
        assert_eq!(stats.total_metrics, 2);
        assert_eq!(stats.total_data_points, 2);
    }
}
