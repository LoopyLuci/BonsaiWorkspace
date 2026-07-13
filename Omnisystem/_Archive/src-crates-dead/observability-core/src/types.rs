use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Span {
    pub span_id: Uuid,
    pub trace_id: Uuid,
    pub operation_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub status: SpanStatus,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum SpanStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metric {
    pub metric_id: Uuid,
    pub name: String,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
    pub labels: Vec<(String, String)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trace {
    pub trace_id: Uuid,
    pub root_span_id: Uuid,
    pub service_name: String,
    pub start_time: DateTime<Utc>,
    pub total_duration_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DistributedContext {
    pub trace_id: Uuid,
    pub span_id: Uuid,
    pub parent_span_id: Option<Uuid>,
    pub baggage: Vec<(String, String)>,
}
