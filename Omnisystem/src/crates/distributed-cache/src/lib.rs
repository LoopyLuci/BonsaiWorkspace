//! distributed-cache: an in-memory cache with TTL expiry, capacity-based
//! eviction, hit/miss statistics ([`cache::DistributedCache`]), and a
//! cluster membership tracker for replica nodes ([`replication::ReplicationManager`]).

pub mod cache;
pub mod error;
pub mod replication;
pub mod types;

pub use cache::DistributedCache;
pub use error::{CacheError, CacheResult};
pub use replication::ReplicationManager;
pub use types::{CacheConfig, CacheEntry, CacheStats, ReplicationConfig};
