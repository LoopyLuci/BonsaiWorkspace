use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum AggregationPeriod {
    OneSecond,
    TenSeconds,
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    OneHour,
}

impl AggregationPeriod {
    pub fn duration_secs(&self) -> u64 {
        match self {
            AggregationPeriod::OneSecond => 1,
            AggregationPeriod::TenSeconds => 10,
            AggregationPeriod::OneMinute => 60,
            AggregationPeriod::FiveMinutes => 300,
            AggregationPeriod::FifteenMinutes => 900,
            AggregationPeriod::OneHour => 3600,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub labels: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AggregationWindow {
    pub period: AggregationPeriod,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub data_points: Vec<TimeSeriesPoint>,
    pub count: u64,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricTimeSeries {
    pub metric_name: String,
    pub windows: Vec<AggregationWindow>,
    pub retention_hours: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceObservabilityMetrics {
    pub service_id: String,
    pub request_count: u64,
    pub error_count: u64,
    pub success_rate: f64,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub avg_latency_ms: f64,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AggregatorConfig {
    pub window_size_secs: u64,
    pub retention_hours: u32,
    pub flush_interval_secs: u64,
    pub max_windows_per_metric: usize,
}

impl Default for AggregatorConfig {
    fn default() -> Self {
        Self {
            window_size_secs: 60,
            retention_hours: 24,
            flush_interval_secs: 300,
            max_windows_per_metric: 1440,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: DateTime<Utc>,
    pub metrics: HashMap<String, f64>,
    pub labels: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AggregationStats {
    pub total_metrics: usize,
    pub total_windows: usize,
    pub total_data_points: u64,
    pub last_rotation: DateTime<Utc>,
    pub uptime_secs: u64,
}
