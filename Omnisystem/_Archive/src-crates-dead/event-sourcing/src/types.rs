use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainEvent {
    pub event_id: Uuid,
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub version: u32,
    pub timestamp: DateTime<Utc>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventStore {
    pub store_id: Uuid,
    pub aggregate_type: String,
    pub event_count: u64,
    pub last_event_time: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub snapshot_id: Uuid,
    pub aggregate_id: Uuid,
    pub version: u32,
    pub state: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventProjection {
    pub projection_id: Uuid,
    pub aggregate_id: Uuid,
    pub projected_state: Vec<u8>,
    pub version: u32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayLog {
    pub replay_id: Uuid,
    pub aggregate_id: Uuid,
    pub start_version: u32,
    pub end_version: u32,
    pub events_replayed: u32,
    pub completed_at: DateTime<Utc>,
}
