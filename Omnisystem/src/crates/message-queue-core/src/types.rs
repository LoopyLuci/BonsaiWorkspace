use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Topic {
    pub topic_id: Uuid,
    pub name: String,
    pub partition_count: u32,
    pub replication_factor: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Partition {
    pub partition_id: Uuid,
    pub topic_name: String,
    pub partition_number: u32,
    pub leader_broker: u32,
    pub replicas: Vec<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub message_id: Uuid,
    pub topic_name: String,
    pub partition: u32,
    pub offset: u64,
    pub key: Option<String>,
    pub value: Vec<u8>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsumerGroup {
    pub group_id: Uuid,
    pub name: String,
    pub members: Vec<String>,
    pub topics: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsumerOffset {
    pub offset_id: Uuid,
    pub group_name: String,
    pub topic_name: String,
    pub partition: u32,
    pub offset: u64,
    pub last_commit: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Broker {
    pub broker_id: u32,
    pub host: String,
    pub port: u16,
    pub is_leader: bool,
}
