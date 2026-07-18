use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Experiment {
    pub experiment_id: Uuid,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub status: ExperimentStatus,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum ExperimentStatus {
    Planning,
    Running,
    Completed,
    Failed,
    Archived,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExperimentRun {
    pub run_id: Uuid,
    pub experiment_id: Uuid,
    pub run_number: u32,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub status: RunStatus,
    pub metrics: HashMap<String, f64>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum RunStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hyperparameter {
    pub param_id: Uuid,
    pub run_id: Uuid,
    pub param_name: String,
    pub param_value: String,
    pub param_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricHistory {
    pub history_id: Uuid,
    pub run_id: Uuid,
    pub metric_name: String,
    pub values: Vec<(DateTime<Utc>, f64)>,
}
