use crate::{CacheConfig, CacheEntry, CacheError, CacheResult, CacheStats};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct DistributedCache {
    data: Arc<DashMap<String, String>>,
    stats: Arc<CacheStats>,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
}

impl DistributedCache {
    pub fn new(_config: &CacheConfig) -> Self {
        Self {
            data: Arc::new(DashMap::new()),
            stats: Arc::new(CacheStats {
                hits: 0,
                misses: 0,
                evictions: 0,
                entries: 0,
                memory_used_bytes: 0,
                hit_rate: 0.0,
            }),
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
        }
    }

    pub async fn get(&self, key: &str) -> CacheResult<String> {
        if let Some(entry) = self.data.get(key) {
            self.hits.fetch_add(1, Ordering::SeqCst);
            Ok(entry.clone())
        } else {
            self.misses.fetch_add(1, Ordering::SeqCst);
            Err(CacheError::KeyNotFound)
        }
    }

    pub async fn set(&self, key: &str, value: String) -> CacheResult<()> {
        self.data.insert(key.to_string(), value);
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
}
