use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventPattern {
    pub pattern_id: Uuid,
    pub name: String,
    pub conditions: Vec<String>,
    pub time_window_ms: u64,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatternMatch {
    pub match_id: Uuid,
    pub pattern_id: Uuid,
    pub matched_events: Vec<Uuid>,
    pub confidence: f64,
    pub matched_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventSequence {
    pub sequence_id: Uuid,
    pub event_ids: Vec<Uuid>,
    pub sequence_type: String,
    pub duration_ms: u64,
    pub detected_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventCorrelation {
    pub correlation_id: Uuid,
    pub primary_event_id: Uuid,
    pub related_event_ids: Vec<Uuid>,
    pub correlation_score: f64,
    pub correlation_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CEPAlert {
    pub alert_id: Uuid,
    pub pattern_id: Uuid,
    pub match_id: Uuid,
    pub severity: AlertSeverity,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}
