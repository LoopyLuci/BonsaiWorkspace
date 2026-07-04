use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityRule {
    pub rule_id: Uuid,
    pub name: String,
    pub dataset_id: Uuid,
    pub threshold: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataProfile {
    pub profile_id: Uuid,
    pub dataset_id: Uuid,
    pub null_count: u64,
    pub unique_count: u64,
    pub min_value: f64,
    pub max_value: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Anomaly {
    pub anomaly_id: Uuid,
    pub dataset_id: Uuid,
    pub record_index: u64,
    pub anomaly_score: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    pub result_id: Uuid,
    pub dataset_id: Uuid,
    pub passed: bool,
    pub violations: Vec<String>,
}
