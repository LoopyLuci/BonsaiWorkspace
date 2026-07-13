use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeSeriesMetric {
    pub metric_id: Uuid,
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub tags: Vec<(String, String)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AggregatedValue {
    pub agg_id: Uuid,
    pub metric_name: String,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub count: u32,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PercentileValue {
    pub percentile_id: Uuid,
    pub metric_name: String,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub p999: f64,
    pub computed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rollup {
    pub rollup_id: Uuid,
    pub source_metric: String,
    pub target_bucket: String,
    pub retention_days: u32,
    pub schedule: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DownsampledData {
    pub downsample_id: Uuid,
    pub metric_name: String,
    pub source_points: u32,
    pub target_points: u32,
    pub compression_ratio: f32,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricBucket {
    pub bucket_id: Uuid,
    pub metric_name: String,
    pub bucket_time: DateTime<Utc>,
    pub bucket_size_ms: u64,
    pub values: Vec<f64>,
}
