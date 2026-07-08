use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub source: String,
    pub severity: Severity,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum EventType {
    UnauthorizedAccess,
    DataExfiltration,
    MalwareDetected,
    BruteForceAttempt,
    AnomalousActivity,
    Custom(String),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnomalyDetection {
    pub anomaly_id: Uuid,
    pub event_signature: String,
    pub anomaly_score: f64,
    pub detected_at: DateTime<Utc>,
    pub is_anomalous: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThreatIncident {
    pub incident_id: Uuid,
    pub events: Vec<Uuid>,
    pub threat_score: f64,
    pub status: IncidentStatus,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum IncidentStatus {
    Detected,
    Investigating,
    Contained,
    Resolved,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorrelatedEvents {
    pub correlation_id: Uuid,
    pub event_ids: Vec<Uuid>,
    pub correlation_score: f64,
    pub pattern: String,
    pub timestamp: DateTime<Utc>,
}
