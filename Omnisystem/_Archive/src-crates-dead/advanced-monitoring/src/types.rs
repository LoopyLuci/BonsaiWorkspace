use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metric {
    pub metric_name: String,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
    pub labels: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeSeries {
    pub metric_name: String,
    pub data_points: Vec<(DateTime<Utc>, f64)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnalyticsResult {
    pub mean: f64,
    pub median: f64,
    pub p95: f64,
    pub p99: f64,
    pub min: f64,
    pub max: f64,
}
