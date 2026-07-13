use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: String,
    pub user_id: String,
    pub action: String,
    pub resource: String,
    pub timestamp: DateTime<Utc>,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditLog {
    pub log_id: String,
    pub events: Vec<AuditEvent>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub report_id: String,
    pub violations: u32,
    pub last_audit: DateTime<Utc>,
    pub status: String,
}
