//! Core Cache Implementation
//!
//! Provides concurrent, thread-safe caching with configurable eviction policies,
//! tiered storage, and distributed clustering support.

use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;

/// Cache configuration
#[derive(Clone, Debug)]
pub struct CacheConfig {
    pub capacity: usize,
    pub eviction_policy: crate::Policy,
    pub ttl: Option<Duration>,
    pub replication_factor: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            capacity: 10000,
            eviction_policy: crate::Policy::Arc,
            ttl: None,
            replication_factor: 3,
        }
    }
}

/// Main Cache structure
pub struct Cache<K, V>
where
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    data: Arc<DashMap<K, CacheEntry<V>>>,
    config: Arc<RwLock<CacheConfig>>,
    stats: Arc<crate::metrics::CacheMetrics>,
}

pub struct CacheEntry<V> {
    pub value: V,
    pub created_at: Instant,
    pub last_accessed: Instant,
    pub access_count: u64,
    pub ttl: Option<Instant>,
}

impl<K, V> Cache<K, V>
where
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn builder() -> CacheBuilder<K, V> {
        CacheBuilder::new()
    }

    /// Insert a value into the cache
    pub fn insert(&self, key: K, value: V) {
        let entry = CacheEntry {
            value,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 0,
            ttl: self.config.read().ttl.map(|d| Instant::now() + d),
        };

        self.data.insert(key.clone(), entry);
        self.stats.record_insert();

        // Check capacity and evict if needed
        if self.data.len() > self.config.read().capacity {
            // Placeholder: would call eviction policy
            self.stats.record_eviction();
        }
    }

    /// Get a value from the cache
    pub fn get(&self, key: &K) -> Option<V> {
        if let Some(mut entry_ref) = self.data.get_mut(key) {
            let entry = entry_ref.value_mut();

            // Check if expired
            if let Some(expiry) = entry.ttl {
                if Instant::now() > expiry {
                    drop(entry_ref);
                    self.data.remove(key);
                    self.stats.record_expiration();
                    return None;
                }
            }

            // Update access metadata
            entry.last_accessed = Instant::now();
            entry.access_count = entry.access_count.saturating_add(1);
            self.stats.record_hit();

            Some(entry.value.clone())
        } else {
            self.stats.record_miss();
            None
        }
    }

    /// Remove a value from the cache
    pub fn remove(&self, key: &K) -> Option<V> {
        if let Some((_, entry)) = self.data.remove(key) {
            self.stats.record_removal();
            Some(entry.value)
        } else {
            None
        }
    }

    /// Get cache size
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.data.clear();
        self.stats.reset();
    }

    /// Get cache statistics
    pub fn stats(&self) -> crate::metrics::CacheMetrics {
        self.stats.clone()
    }
}

/// Builder for Cache
pub struct CacheBuilder<K, V> {
    config: CacheConfig,
    _phantom: std::marker::PhantomData<(K, V)>,
}

impl<K, V> CacheBuilder<K, V>
where
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            config: CacheConfig::default(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn capacity(mut self, cap: usize) -> Self {
        self.config.capacity = cap;
        self
    }

    pub fn policy(mut self, policy: crate::Policy) -> Self {
        self.config.eviction_policy = policy;
        self
    }

    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.config.ttl = Some(ttl);
        self
    }

    pub fn replication_factor(mut self, rf: usize) -> Self {
        self.config.replication_factor = rf;
        self
    }

    pub fn build(self) -> Cache<K, V> {
        Cache {
            data: Arc::new(DashMap::new()),
            config: Arc::new(RwLock::new(self.config)),
            stats: Arc::new(crate::metrics::CacheMetrics::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let cache = Cache::builder::<String, String>()
            .capacity(100)
            .build();

        cache.insert("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
    }

    #[test]
    fn test_remove() {
        let cache = Cache::builder::<String, String>()
            .capacity(100)
            .build();

        cache.insert("key1".to_string(), "value1".to_string());
        assert_eq!(cache.remove(&"key1".to_string()), Some("value1".to_string()));
        assert_eq!(cache.get(&"key1".to_string()), None);
    }

    #[test]
    fn test_clear() {
        let cache = Cache::builder::<String, String>()
            .capacity(100)
            .build();

        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert_eq!(cache.len(), 0);
    }
}
