use crate::{DataPoint, Aggregation, TimeSeriesTrend, StatisticalSummary, AnalyticsError, AnalyticsResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct AnalyticsEngine {
    data_points: Arc<DashMap<Uuid, DataPoint>>,
    aggregations: Arc<DashMap<Uuid, Aggregation>>,
    trends: Arc<DashMap<Uuid, TimeSeriesTrend>>,
    summaries: Arc<DashMap<Uuid, StatisticalSummary>>,
}

impl AnalyticsEngine {
    pub fn new() -> Self {
        Self {
            data_points: Arc::new(DashMap::new()),
            aggregations: Arc::new(DashMap::new()),
            trends: Arc::new(DashMap::new()),
            summaries: Arc::new(DashMap::new()),
        }
    }

    pub async fn ingest_data_point(&self, point: &DataPoint) -> AnalyticsResult<()> {
        self.data_points.insert(point.point_id, point.clone());
        Ok(())
    }

    pub async fn aggregate_data(&self, dataset_id: Uuid, agg_type: &str) -> AnalyticsResult<Aggregation> {
        let mut sum = 0.0;
        let mut count = 0;

        for entry in self.data_points.iter() {
            if entry.value().dataset_id == dataset_id {
                sum += entry.value().value;
                count += 1;
            }
        }

        if count == 0 {
            return Err(AnalyticsError::DatasetNotFound);
        }

        let result = match agg_type {
            "sum" => sum,
            "avg" => sum / count as f64,
            "count" => count as f64,
            _ => return Err(AnalyticsError::AggregationFailed),
        };

        let agg = Aggregation {
            agg_id: Uuid::new_v4(),
            dataset_id,
            agg_type: agg_type.to_string(),
            time_bucket: "1h".to_string(),
            result,
        };

        self.aggregations.insert(agg.agg_id, agg.clone());
        Ok(agg)
    }

    pub async fn analyze_trend(&self, dataset_id: Uuid) -> AnalyticsResult<TimeSeriesTrend> {
        let mut values = Vec::new();

        for entry in self.data_points.iter() {
            if entry.value().dataset_id == dataset_id {
                values.push(entry.value().value);
            }
        }

        if values.is_empty() {
            return Err(AnalyticsError::DatasetNotFound);
        }

        let trend_direction = if values.len() > 1 && values[values.len() - 1] > values[0] {
            "increasing".to_string()
        } else {
            "stable".to_string()
        };

        let trend = TimeSeriesTrend {
            trend_id: Uuid::new_v4(),
            dataset_id,
            trend_direction,
            magnitude: 0.5,
            confidence: 0.85,
        };

        self.trends.insert(trend.trend_id, trend.clone());
        Ok(trend)
    }

    pub async fn compute_statistics(&self, dataset_id: Uuid) -> AnalyticsResult<StatisticalSummary> {
        let mut values = Vec::new();

        for entry in self.data_points.iter() {
            if entry.value().dataset_id == dataset_id {
                values.push(entry.value().value);
            }
        }

        if values.is_empty() {
            return Err(AnalyticsError::DatasetNotFound);
        }

        values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
        let stddev = variance.sqrt();

        let summary = StatisticalSummary {
            summary_id: Uuid::new_v4(),
            dataset_id,
            mean,
            stddev,
            min: values[0],
            max: values[values.len() - 1],
            percentile_95: values[(values.len() as f64 * 0.95) as usize],
        };

        self.summaries.insert(summary.summary_id, summary.clone());
        Ok(summary)
    }

    pub fn point_count(&self) -> usize {
        self.data_points.len()
    }
}

impl Default for AnalyticsEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ingest_data_point() {
        let engine = AnalyticsEngine::new();
        let point = DataPoint {
            point_id: Uuid::new_v4(),
            dataset_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            value: 42.5,
            dimensions: vec![("region".to_string(), "us-east".to_string())],
        };

        engine.ingest_data_point(&point).await.unwrap();
        assert_eq!(engine.point_count(), 1);
    }

    #[tokio::test]
    async fn test_aggregate_data() {
        let engine = AnalyticsEngine::new();
        let dataset_id = Uuid::new_v4();

        for i in 1..=5 {
            let point = DataPoint {
                point_id: Uuid::new_v4(),
                dataset_id,
                timestamp: Utc::now(),
                value: i as f64,
                dimensions: vec![],
            };
            engine.ingest_data_point(&point).await.unwrap();
        }

        let agg = engine.aggregate_data(dataset_id, "sum").await.unwrap();
        assert_eq!(agg.result, 15.0);
    }

    #[tokio::test]
    async fn test_analyze_trend() {
        let engine = AnalyticsEngine::new();
        let dataset_id = Uuid::new_v4();

        for i in 1..=3 {
            let point = DataPoint {
                point_id: Uuid::new_v4(),
                dataset_id,
                timestamp: Utc::now(),
                value: (i * 10) as f64,
                dimensions: vec![],
            };
            engine.ingest_data_point(&point).await.unwrap();
        }

        let trend = engine.analyze_trend(dataset_id).await.unwrap();
        assert_eq!(trend.trend_direction, "increasing");
    }

    #[tokio::test]
    async fn test_compute_statistics() {
        let engine = AnalyticsEngine::new();
        let dataset_id = Uuid::new_v4();

        for i in 1..=5 {
            let point = DataPoint {
                point_id: Uuid::new_v4(),
                dataset_id,
                timestamp: Utc::now(),
                value: i as f64,
                dimensions: vec![],
            };
            engine.ingest_data_point(&point).await.unwrap();
        }

        let stats = engine.compute_statistics(dataset_id).await.unwrap();
        assert_eq!(stats.mean, 3.0);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 5.0);
    }
}
