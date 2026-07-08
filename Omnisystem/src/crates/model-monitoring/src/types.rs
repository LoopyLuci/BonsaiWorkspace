use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelPerformance {
    pub perf_id: Uuid,
    pub model_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub accuracy: f64,
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataDrift {
    pub drift_id: Uuid,
    pub model_id: Uuid,
    pub feature_name: String,
    pub drift_score: f64,
    pub drift_detected: bool,
    pub detected_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PredictionDrift {
    pub prediction_id: Uuid,
    pub model_id: Uuid,
    pub distribution_divergence: f64,
    pub baseline_distribution: String,
    pub current_distribution: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthCheck {
    pub check_id: Uuid,
    pub model_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub status: HealthStatus,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnomalyRecord {
    pub anomaly_id: Uuid,
    pub model_id: Uuid,
    pub input_signature: String,
    pub anomaly_score: f64,
    pub detected_at: DateTime<Utc>,
}
