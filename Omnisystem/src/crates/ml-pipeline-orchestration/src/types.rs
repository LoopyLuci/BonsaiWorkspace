use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pipeline {
    pub pipeline_id: Uuid,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub status: PipelineStatus,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum PipelineStatus {
    Draft,
    Active,
    Paused,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineTask {
    pub task_id: Uuid,
    pub pipeline_id: Uuid,
    pub task_name: String,
    pub task_type: TaskType,
    pub dependencies: Vec<Uuid>,
    pub status: TaskStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum TaskType {
    DataPreprocessing,
    FeatureEngineering,
    ModelTraining,
    Evaluation,
    Deployment,
    Custom(String),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum TaskStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineExecution {
    pub execution_id: Uuid,
    pub pipeline_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub execution_status: ExecutionStatus,
    pub task_results: Vec<(Uuid, TaskStatus)>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum ExecutionStatus {
    Queued,
    Running,
    Succeeded,
    PartiallyFailed,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineSchedule {
    pub schedule_id: Uuid,
    pub pipeline_id: Uuid,
    pub schedule_type: ScheduleType,
    pub next_run: DateTime<Utc>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum ScheduleType {
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Custom(String),
}
