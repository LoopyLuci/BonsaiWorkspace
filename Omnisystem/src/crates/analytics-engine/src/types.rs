use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataPoint {
    pub point_id: Uuid,
    pub dataset_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub dimensions: Vec<(String, String)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Aggregation {
    pub agg_id: Uuid,
    pub dataset_id: Uuid,
    pub agg_type: String,
    pub time_bucket: String,
    pub result: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeSeriesTrend {
    pub trend_id: Uuid,
    pub dataset_id: Uuid,
    pub trend_direction: String,
    pub magnitude: f64,
    pub confidence: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatisticalSummary {
    pub summary_id: Uuid,
    pub dataset_id: Uuid,
    pub mean: f64,
    pub stddev: f64,
    pub min: f64,
    pub max: f64,
    pub percentile_95: f64,
}
