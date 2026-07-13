use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsumerLag {
    pub lag_id: Uuid,
    pub consumer_group: String,
    pub topic: String,
    pub partition: u32,
    pub lag_messages: u64,
    pub lag_timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Throughput {
    pub throughput_id: Uuid,
    pub topic: String,
    pub messages_per_second: f32,
    pub bytes_per_second: u64,
    pub measured_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueueHealth {
    pub health_id: Uuid,
    pub topic: String,
    pub is_healthy: bool,
    pub error_rate: f32,
    pub checked_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Alert {
    pub alert_id: Uuid,
    pub alert_type: String,
    pub severity: String,
    pub message: String,
    pub triggered_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueueMetrics {
    pub metrics_id: Uuid,
    pub topic: String,
    pub total_messages: u64,
    pub failed_messages: u64,
    pub avg_latency_ms: f32,
}
