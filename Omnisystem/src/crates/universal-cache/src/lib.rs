//! Universal Cache
//!
//! Concurrent, thread-safe caching with pluggable eviction policies
//! (LRU, LFU, ARC, TinyLFU), tiered storage abstractions, and a
//! consistent-hashing cluster layer for distributed deployments.

pub mod cache;
pub mod cluster;
pub mod entry;
pub mod error;
pub mod eviction;
pub mod metrics;
pub mod storage;
pub mod types;

pub use cache::{Cache, CacheBuilder, CacheConfig, CacheEntry};
pub use error::{Error, Result};
pub use eviction::EvictionPolicy;

/// Which eviction algorithm a [`Cache`] should use once it reaches capacity
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    /// Least Recently Used
    Lru,
    /// Least Frequently Used
    Lfu,
    /// Adaptive Replacement Cache
    Arc,
    /// TinyLFU (count-min sketch based frequency estimation)
    TinyLfu,
}

impl Default for Policy {
    fn default() -> Self {
        Policy::Arc
    }
}
