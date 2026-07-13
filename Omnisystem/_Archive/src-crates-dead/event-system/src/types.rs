use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub event_id: String,
    pub event_type: String,
    pub source: String,
    pub timestamp: DateTime<Utc>,
    pub payload: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventSubscription {
    pub subscription_id: String,
    pub event_type: String,
    pub handler_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventStats {
    pub total_events: u64,
    pub processed_events: u64,
    pub failed_events: u64,
    pub subscriptions: u32,
}
