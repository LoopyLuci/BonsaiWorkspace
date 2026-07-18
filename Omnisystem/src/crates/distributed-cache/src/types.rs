use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheConfig {
    pub max_size_bytes: u64,
    pub max_entries: usize,
    pub eviction_policy: String,
    pub ttl_seconds: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub value: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub entries: u64,
    pub memory_used_bytes: u64,
    pub hit_rate: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplicationConfig {
    pub replica_count: u32,
    pub sync_mode: String,
}
