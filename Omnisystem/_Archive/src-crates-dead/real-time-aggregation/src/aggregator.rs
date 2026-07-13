use crate::{TimeSeriesMetric, AggregatedValue, PercentileValue, Rollup, DownsampledData, MetricBucket, AggregationError, AggregationResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct RealTimeAggregator {
    metrics: Arc<DashMap<Uuid, TimeSeriesMetric>>,
    aggregated: Arc<DashMap<Uuid, AggregatedValue>>,
    percentiles: Arc<DashMap<Uuid, PercentileValue>>,
    rollups: Arc<DashMap<Uuid, Rollup>>,
    downsampled: Arc<DashMap<Uuid, DownsampledData>>,
    buckets: Arc<DashMap<Uuid, MetricBucket>>,
}

impl RealTimeAggregator {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(DashMap::new()),
            aggregated: Arc::new(DashMap::new()),
            percentiles: Arc::new(DashMap::new()),
            rollups: Arc::new(DashMap::new()),
            downsampled: Arc::new(DashMap::new()),
            buckets: Arc::new(DashMap::new()),
        }
    }

    pub async fn record_metric(&self, name: &str, value: f64) -> AggregationResult<TimeSeriesMetric> {
        let metric = TimeSeriesMetric {
            metric_id: Uuid::new_v4(),
            name: name.to_string(),
            timestamp: Utc::now(),
            value,
            tags: vec![],
        };

        self.metrics.insert(metric.metric_id, metric.clone());
        Ok(metric)
    }

    pub async fn aggregate_window(&self, metric_name: &str, window_size_ms: u64) -> AggregationResult<AggregatedValue> {
        let mut values = Vec::new();

        for entry in self.metrics.iter() {
            if entry.value().name == metric_name {
                values.push(entry.value().value);
            }
        }

        if values.is_empty() {
            return Err(AggregationError::MetricNotFound);
        }

        values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let sum: f64 = values.iter().sum();
        let count = values.len() as u32;
        let avg = sum / count as f64;

        let agg = AggregatedValue {
            agg_id: Uuid::new_v4(),
            metric_name: metric_name.to_string(),
            window_start: Utc::now(),
            window_end: Utc::now() + chrono::Duration::milliseconds(window_size_ms as i64),
            count,
            sum,
            min: values[0],
            max: values[values.len() - 1],
            avg,
        };

        self.aggregated.insert(agg.agg_id, agg.clone());
        Ok(agg)
    }

    pub async fn compute_percentiles(&self, metric_name: &str) -> AggregationResult<PercentileValue> {
        let mut values = Vec::new();

        for entry in self.metrics.iter() {
            if entry.value().name == metric_name {
                values.push(entry.value().value);
            }
        }

        if values.is_empty() {
            return Err(AggregationError::MetricNotFound);
        }

        values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p50_idx = (values.len() as f64 * 0.50) as usize;
        let p95_idx = (values.len() as f64 * 0.95) as usize;
        let p99_idx = (values.len() as f64 * 0.99) as usize;
        let p999_idx = (values.len() as f64 * 0.999) as usize;

        let percentile = PercentileValue {
            percentile_id: Uuid::new_v4(),
            metric_name: metric_name.to_string(),
            p50: values[p50_idx.min(values.len() - 1)],
            p95: values[p95_idx.min(values.len() - 1)],
            p99: values[p99_idx.min(values.len() - 1)],
            p999: values[p999_idx.min(values.len() - 1)],
            computed_at: Utc::now(),
        };

        self.percentiles.insert(percentile.percentile_id, percentile.clone());
        Ok(percentile)
    }

    pub async fn create_rollup(&self, source: &str, target_bucket: &str, retention: u32) -> AggregationResult<Rollup> {
        let rollup = Rollup {
            rollup_id: Uuid::new_v4(),
            source_metric: source.to_string(),
            target_bucket: target_bucket.to_string(),
            retention_days: retention,
            schedule: "hourly".to_string(),
        };

        self.rollups.insert(rollup.rollup_id, rollup.clone());
        Ok(rollup)
    }

    pub async fn downsample(&self, metric_name: &str, ratio: f32) -> AggregationResult<DownsampledData> {
        let count = self.metrics.len() as u32;
        let target_points = (count as f32 * ratio) as u32;

        let downsampled = DownsampledData {
            downsample_id: Uuid::new_v4(),
            metric_name: metric_name.to_string(),
            source_points: count,
            target_points,
            compression_ratio: ratio,
            created_at: Utc::now(),
        };

        self.downsampled.insert(downsampled.downsample_id, downsampled.clone());
        Ok(downsampled)
    }

    pub fn metric_count(&self) -> usize {
        self.metrics.len()
    }
}

impl Default for RealTimeAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_metric() {
        let aggregator = RealTimeAggregator::new();
        let metric = aggregator.record_metric("cpu_usage", 75.5).await.unwrap();

        assert_eq!(metric.name, "cpu_usage");
        assert_eq!(metric.value, 75.5);
        assert_eq!(aggregator.metric_count(), 1);
    }

    #[tokio::test]
    async fn test_aggregate_window() {
        let aggregator = RealTimeAggregator::new();
        aggregator.record_metric("latency", 100.0).await.unwrap();
        aggregator.record_metric("latency", 200.0).await.unwrap();
        aggregator.record_metric("latency", 300.0).await.unwrap();

        let agg = aggregator.aggregate_window("latency", 60000).await.unwrap();
        assert_eq!(agg.count, 3);
        assert_eq!(agg.sum, 600.0);
    }

    #[tokio::test]
    async fn test_compute_percentiles() {
        let aggregator = RealTimeAggregator::new();
        for i in 1..=100 {
            aggregator.record_metric("response_time", i as f64).await.unwrap();
        }

        let percentiles = aggregator.compute_percentiles("response_time").await.unwrap();
        assert!(percentiles.p95 > percentiles.p50);
        assert!(percentiles.p99 > percentiles.p95);
    }

    #[tokio::test]
    async fn test_create_rollup() {
        let aggregator = RealTimeAggregator::new();
        let rollup = aggregator.create_rollup("cpu_1m", "cpu_1h", 30).await.unwrap();

        assert_eq!(rollup.retention_days, 30);
    }
}
