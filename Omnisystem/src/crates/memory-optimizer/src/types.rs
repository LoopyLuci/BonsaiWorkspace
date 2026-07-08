use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryBlock {
    pub block_id: u64,
    pub size_bytes: u64,
    pub allocated: bool,
    pub allocated_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GcStatistics {
    pub total_collections: u64,
    pub total_freed_bytes: u64,
    pub last_collection_time_ms: u64,
    pub heap_size_bytes: u64,
    pub heap_used_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryPool {
    pub pool_id: u64,
    pub block_size: u64,
    pub block_count: u64,
    pub free_blocks: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum CompressionLevel {
    None = 0,
    Fast = 1,
    Balanced = 2,
    High = 3,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompressedBlock {
    pub original_size: u64,
    pub compressed_size: u64,
    pub compression_ratio: f32,
    pub compression_level: CompressionLevel,
}
