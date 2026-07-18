use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlertRule {
    pub rule_id: Uuid,
    pub metric_name: String,
    pub threshold: f64,
    pub comparison_op: String,
    pub severity: AlertSeverity,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Alert {
    pub alert_id: Uuid,
    pub rule_id: Uuid,
    pub metric_value: f64,
    pub severity: AlertSeverity,
    pub created_at: DateTime<Utc>,
    pub status: AlertStatus,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum AlertStatus {
    Triggered,
    Acknowledged,
    Resolved,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IncidentRecord {
    pub incident_id: Uuid,
    pub alert_id: Uuid,
    pub severity: AlertSeverity,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub status: IncidentStatus,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum IncidentStatus {
    Open,
    InProgress,
    Resolved,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationRoute {
    pub route_id: Uuid,
    pub severity: AlertSeverity,
    pub channels: Vec<String>,
    pub recipients: Vec<String>,
}
