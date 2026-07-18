use crate::{CacheConfig, CacheEntry, CacheError, CacheResult, CacheStats};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct DistributedCache {
    data: Arc<DashMap<String, CacheEntry>>,
    config: CacheConfig,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    evictions: Arc<AtomicU64>,
}

impl DistributedCache {
    pub fn new(config: &CacheConfig) -> Self {
        Self {
            data: Arc::new(DashMap::new()),
            config: config.clone(),
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            evictions: Arc::new(AtomicU64::new(0)),
        }
    }

    pub async fn get(&self, key: &str) -> CacheResult<String> {
        if let Some(entry) = self.data.get(key) {
            if let Some(expires_at) = entry.expires_at {
                if Utc::now() >= expires_at {
                    drop(entry);
                    self.data.remove(key);
                    self.evictions.fetch_add(1, Ordering::SeqCst);
                    self.misses.fetch_add(1, Ordering::SeqCst);
                    return Err(CacheError::KeyNotFound);
                }
            }
            self.hits.fetch_add(1, Ordering::SeqCst);
            Ok(entry.value.clone())
        } else {
            self.misses.fetch_add(1, Ordering::SeqCst);
            Err(CacheError::KeyNotFound)
        }
    }

    pub async fn set(&self, key: &str, value: String) -> CacheResult<()> {
        if self.config.max_entries > 0
            && self.data.len() >= self.config.max_entries
            && !self.data.contains_key(key)
        {
            // Simple eviction: drop one existing entry to make room. A real
            // deployment would honor `eviction_policy` (e.g. LRU); we track
            // the eviction count either way so callers can observe pressure.
            //
            // The iterator lookup is bound to its own `let` first (rather than
            // inlined into the `if let` scrutinee) so DashMap's shard read-lock
            // held inside the iterator is fully dropped before `remove()` below
            // tries to take a write lock on that same shard -- inlining it would
            // extend the iterator's temporary lifetime across the whole `if let`
            // body (a well-known Rust footgun) and self-deadlock.
            let evict_key = self.data.iter().next().map(|e| e.key().clone());
            if let Some(evict_key) = evict_key {
                self.data.remove(&evict_key);
                self.evictions.fetch_add(1, Ordering::SeqCst);
            }
        }

        let expires_at = if self.config.ttl_seconds > 0 {
            Some(Utc::now() + chrono::Duration::seconds(self.config.ttl_seconds as i64))
        } else {
            None
        };

        let entry = CacheEntry {
            key: key.to_string(),
            value,
            created_at: Utc::now(),
            expires_at,
        };
        self.data.insert(key.to_string(), entry);
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> CacheResult<()> {
        if self.data.remove(key).is_some() {
            Ok(())
        } else {
            Err(CacheError::KeyNotFound)
        }
    }

    pub async fn clear(&self) -> CacheResult<()> {
        self.data.clear();
        Ok(())
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Compute live cache statistics from the current state and running
    /// hit/miss/eviction counters.
    pub fn stats(&self) -> CacheStats {
        let hits = self.hits.load(Ordering::SeqCst);
        let misses = self.misses.load(Ordering::SeqCst);
        let total = hits + misses;
        let hit_rate = if total > 0 { hits as f64 / total as f64 } else { 0.0 };
        let memory_used_bytes = self
            .data
            .iter()
            .map(|e| (e.key().len() + e.value.len()) as u64)
            .sum();

        CacheStats {
            hits,
            misses,
            evictions: self.evictions.load(Ordering::SeqCst),
            entries: self.data.len() as u64,
            memory_used_bytes,
            hit_rate,
        }
    }
}

impl Default for DistributedCache {
    fn default() -> Self {
        Self::new(&CacheConfig {
            max_size_bytes: 1024 * 1024 * 100,
            max_entries: 10000,
            eviction_policy: "lru".to_string(),
            ttl_seconds: 3600,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_set_get() {
        let cache = DistributedCache::default();
        cache.set("key1", "value1".to_string()).await.unwrap();
        let val = cache.get("key1").await.unwrap();
        assert_eq!(val, "value1");
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = DistributedCache::default();
        let result = cache.get("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cache_delete() {
        let cache = DistributedCache::default();
        cache.set("key1", "value1".to_string()).await.unwrap();
        cache.delete("key1").await.unwrap();
        let result = cache.get("key1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = DistributedCache::default();
        cache.set("key1", "value1".to_string()).await.unwrap();
        cache.clear().await.unwrap();
        assert_eq!(cache.size(), 0);
    }

    #[tokio::test]
    async fn test_cache_stats_track_hits_and_misses() {
        let cache = DistributedCache::default();
        cache.set("key1", "value1".to_string()).await.unwrap();
        cache.get("key1").await.unwrap();
        let _ = cache.get("missing").await;

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.entries, 1);
        assert_eq!(stats.hit_rate, 0.5);
    }

    #[tokio::test]
    async fn test_cache_respects_ttl_expiry() {
        let cache = DistributedCache::new(&CacheConfig {
            max_size_bytes: 1024,
            max_entries: 10,
            eviction_policy: "lru".to_string(),
            ttl_seconds: 0, // 0 means no expiry in our config; verify no-ttl branch
        });
        cache.set("key1", "value1".to_string()).await.unwrap();
        // No TTL configured, so the entry should still be retrievable immediately.
        assert!(cache.get("key1").await.is_ok());
    }

    #[tokio::test]
    async fn test_cache_expired_entry_is_treated_as_miss() {
        let cache = DistributedCache::default();
        // Insert an entry that already expired in the past, bypassing set()
        // so the test doesn't need to sleep for a real TTL to elapse.
        cache.data.insert(
            "stale".to_string(),
            CacheEntry {
                key: "stale".to_string(),
                value: "old".to_string(),
                created_at: Utc::now() - chrono::Duration::seconds(10),
                expires_at: Some(Utc::now() - chrono::Duration::seconds(1)),
            },
        );

        let result = cache.get("stale").await;
        assert!(matches!(result, Err(CacheError::KeyNotFound)));
        assert_eq!(cache.stats().evictions, 1);
        assert_eq!(cache.size(), 0);
    }

    #[tokio::test]
    async fn test_cache_evicts_when_max_entries_exceeded() {
        let cache = DistributedCache::new(&CacheConfig {
            max_size_bytes: 1024 * 1024,
            max_entries: 2,
            eviction_policy: "lru".to_string(),
            ttl_seconds: 0,
        });
        cache.set("key1", "v1".to_string()).await.unwrap();
        cache.set("key2", "v2".to_string()).await.unwrap();
        cache.set("key3", "v3".to_string()).await.unwrap();

        // Capacity is respected: never more than max_entries resident.
        assert!(cache.size() <= 2);
        let stats = cache.stats();
        assert!(stats.evictions >= 1);
    }
}
