use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryPlan {
    pub plan_id: Uuid,
    pub name: String,
    pub resource_id: String,
    pub steps: Vec<RecoveryStep>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryStep {
    pub step_id: Uuid,
    pub order: u32,
    pub action: String,
    pub timeout_seconds: u32,
    pub rollback_action: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryPoint {
    pub point_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub resource_id: String,
    pub backup_id: Uuid,
    pub rpo_seconds: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryExecution {
    pub execution_id: Uuid,
    pub plan_id: Uuid,
    pub status: ExecutionStatus,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub rto_seconds: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Copy, Hash)]
pub enum ExecutionStatus {
    Planning,
    Executing,
    Validating,
    Completed,
    Failed,
    RollingBack,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryTest {
    pub test_id: Uuid,
    pub plan_id: Uuid,
    pub test_name: String,
    pub last_tested: DateTime<Utc>,
    pub success: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub check_id: Uuid,
    pub resource_id: String,
    pub healthy: bool,
    pub timestamp: DateTime<Utc>,
    pub issues: Vec<String>,
}
