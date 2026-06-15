use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditLog {
    pub log_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub result: AuditOutcome,
    pub details: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum AuditOutcome {
    Success,
    Failure,
    PartialSuccess,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogIntegrity {
    pub integrity_id: Uuid,
    pub log_id: Uuid,
    pub hash: String,
    pub previous_hash: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub policy_id: Uuid,
    pub log_type: String,
    pub retention_days: u32,
    pub archive_after_days: u32,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditQuery {
    pub query_id: Uuid,
    pub actor_filter: Option<String>,
    pub action_filter: Option<String>,
    pub resource_filter: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditReport {
    pub report_id: Uuid,
    pub generated_at: DateTime<Utc>,
    pub total_logs: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub actors: Vec<String>,
}
