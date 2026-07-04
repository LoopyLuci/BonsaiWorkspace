//! Omnisystem Concurrent Collections (OCC)
//!
//! High-performance, lock-free concurrent data structures.
//! Zero external dependencies. Enterprise-grade quality.
//!
//! # Features
//!
//! - **Lock-free HashMap**: Read-free locking on concurrent access
//! - **MPMC Queue**: Multi-producer, multi-consumer queue
//! - **Sharded Collections**: Shard-based writes for horizontal scalability
//! - **Streaming Iterators**: Zero-allocation iteration
//! - **DashMap compatibility**: Drop-in replacement API

pub mod concurrent_map;
pub mod concurrent_queue;
pub mod sharded;

pub use concurrent_map::ConcurrentMap;
pub use concurrent_queue::{mpsc_queue, MpscSender, MpscReceiver};
pub use sharded::ShardedMap;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Statistics for concurrent collections
#[derive(Debug, Clone)]
pub struct CollectionStats {
    /// Total entries stored
    pub len: usize,
    /// Capacity
    pub capacity: usize,
    /// Number of shards (for sharded collections)
    pub shard_count: usize,
    /// Total read operations
    pub read_operations: usize,
    /// Total write operations
    pub write_operations: usize,
    /// Lock contentions detected
    pub contentions: usize,
}

/// Hash function trait for collections
pub trait Hasher {
    fn hash(&self, value: &[u8]) -> u64;
}

/// Default FNV-1a hasher (no external deps)
pub struct Fnv1aHasher;

impl Fnv1aHasher {
    /// Hash bytes using FNV-1a algorithm
    pub fn hash(data: &[u8]) -> u64 {
        const FNV_PRIME: u64 = 0x100000001b3;
        const FNV_OFFSET: u64 = 0xcbf29ce484222325;

        let mut hash = FNV_OFFSET;
        for byte in data {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }
}

impl Default for Fnv1aHasher {
    fn default() -> Self {
        Fnv1aHasher
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fnv1a_hasher() {
        let hash1 = Fnv1aHasher::hash(b"test");
        let hash2 = Fnv1aHasher::hash(b"test");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_fnv1a_different_inputs() {
        let hash1 = Fnv1aHasher::hash(b"test1");
        let hash2 = Fnv1aHasher::hash(b"test2");
        assert_ne!(hash1, hash2);
    }
}
