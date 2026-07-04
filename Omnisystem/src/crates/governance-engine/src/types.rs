use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GovernancePolicy {
    pub policy_id: Uuid,
    pub name: String,
    pub dataset_id: Uuid,
    pub access_level: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub retention_id: Uuid,
    pub dataset_id: Uuid,
    pub retention_days: u32,
    pub deletion_date: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComplianceCheck {
    pub check_id: Uuid,
    pub dataset_id: Uuid,
    pub compliance_status: String,
    pub checked_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataAccessLog {
    pub log_id: Uuid,
    pub dataset_id: Uuid,
    pub user_id: String,
    pub access_time: DateTime<Utc>,
}
