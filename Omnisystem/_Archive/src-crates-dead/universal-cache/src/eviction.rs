//! Eviction Policy Trait and Implementations

/// Trait for eviction policies
pub trait EvictionPolicy: Send + Sync {
    type Key: Clone;
    fn record_access(&self, key: &Self::Key);
    fn evict(&self) -> Option<Self::Key>;
    fn clear(&self);
}

// Placeholder implementations - full implementations in separate modules
pub struct LruPolicy;
pub struct LfuPolicy;
pub struct ArcPolicy;
pub struct TinyLfuPolicy;
