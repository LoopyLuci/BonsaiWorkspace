use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamEvent {
    pub event_id: Uuid,
    pub stream_name: String,
    pub timestamp: DateTime<Utc>,
    pub data: HashMap<String, String>,
    pub sequence: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamWindow {
    pub window_id: Uuid,
    pub stream_name: String,
    pub window_type: WindowType,
    pub window_size_ms: u64,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub event_count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum WindowType {
    Tumbling,
    Sliding,
    Session,
    Custom(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Aggregation {
    pub agg_id: Uuid,
    pub window_id: Uuid,
    pub agg_type: String,
    pub result: f64,
    pub computed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamState {
    pub state_id: Uuid,
    pub stream_name: String,
    pub key: String,
    pub state_value: Vec<u8>,
    pub updated_at: DateTime<Utc>,
    pub ttl_ms: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessedResult {
    pub result_id: Uuid,
    pub source_stream: String,
    pub operation: String,
    pub output: String,
    pub processed_at: DateTime<Utc>,
    pub latency_ms: u64,
}
