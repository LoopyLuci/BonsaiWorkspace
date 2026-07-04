use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContinuityPlan {
    pub plan_id: Uuid,
    pub name: String,
    pub organization: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RTO {
    pub rto_id: Uuid,
    pub resource_id: String,
    pub recovery_time_hours: u32,
    pub priority: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RPO {
    pub rpo_id: Uuid,
    pub resource_id: String,
    pub recovery_point_hours: u32,
    pub acceptable_data_loss: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SLA {
    pub sla_id: Uuid,
    pub service_name: String,
    pub availability_percent: f32,
    pub incident_response_minutes: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IncidentReport {
    pub report_id: Uuid,
    pub incident_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub severity: String,
    pub impact_summary: String,
    pub resolution_time_minutes: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComplianceStatus {
    pub status_id: Uuid,
    pub plan_id: Uuid,
    pub compliant: bool,
    pub missing_items: Vec<String>,
    pub last_audit: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContinuityMetrics {
    pub metrics_id: Uuid,
    pub plan_id: Uuid,
    pub actual_rto_hours: f32,
    pub actual_rpo_hours: f32,
    pub sla_achievement_percent: f32,
    pub test_success_rate: f32,
}
