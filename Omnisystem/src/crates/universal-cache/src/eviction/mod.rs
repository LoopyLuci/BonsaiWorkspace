//! Cache eviction policies

pub mod lru;
pub mod lfu;
pub mod arc;
pub mod tinylfu;

pub use lru::LruPolicy;
pub use lfu::LfuPolicy;
pub use arc::ArcPolicy;
pub use tinylfu::TinyLfuPolicy;

/// Core eviction policy trait
pub trait EvictionPolicy: Send + Sync {
    type Key;

    /// Record access to a key (called on cache hit/update)
    fn record_access(&self, key: &Self::Key);

    /// Evict and return the next victim key
    fn evict(&self) -> Option<Self::Key>;

    /// Clear all state
    fn clear(&self);
}
