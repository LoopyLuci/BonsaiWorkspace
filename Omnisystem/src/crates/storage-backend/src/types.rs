use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageEntry {
    pub entry_id: Uuid,
    pub key: String,
    pub value: Vec<u8>,
    pub timestamp: DateTime<Utc>,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemTable {
    pub memtable_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub entry_count: u64,
    pub size_bytes: u64,
    pub is_immutable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SSTable {
    pub sstable_id: Uuid,
    pub level: u32,
    pub key_min: String,
    pub key_max: String,
    pub entry_count: u64,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompressionInfo {
    pub compression_id: Uuid,
    pub original_size: u64,
    pub compressed_size: u64,
    pub compression_ratio: f32,
    pub algorithm: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompactionTask {
    pub task_id: Uuid,
    pub level: u32,
    pub sstables: Vec<Uuid>,
    pub status: CompactionStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum CompactionStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WriteAheadLog {
    pub log_id: Uuid,
    pub sequence_number: u64,
    pub operation: String,
    pub timestamp: DateTime<Utc>,
    pub synced_to_disk: bool,
}
