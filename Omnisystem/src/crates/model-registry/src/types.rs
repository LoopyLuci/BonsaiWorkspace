use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisteredModel {
    pub model_id: Uuid,
    pub name: String,
    pub current_version: String,
    pub owner: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelVersion {
    pub version_id: Uuid,
    pub model_id: Uuid,
    pub version: String,
    pub stage: ModelStage,
    pub metrics: Vec<(String, f32)>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum ModelStage {
    Development,
    Staging,
    Production,
    Archived,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrainingJob {
    pub job_id: Uuid,
    pub model_id: Uuid,
    pub status: JobStatus,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub metrics: Vec<(String, f32)>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelDeployment {
    pub deployment_id: Uuid,
    pub model_id: Uuid,
    pub version: String,
    pub environment: String,
    pub deployed_at: DateTime<Utc>,
    pub status: DeploymentStatus,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum DeploymentStatus {
    InProgress,
    Active,
    Inactive,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub metadata_id: Uuid,
    pub model_id: Uuid,
    pub description: String,
    pub tags: Vec<String>,
    pub performance_metrics: Vec<(String, f32)>,
}
