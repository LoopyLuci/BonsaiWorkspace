use crate::{CacheEntry, Result, DataError};
use dashmap::DashMap;
use std::sync::Arc;

pub trait CacheLayer: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&self, key: String, value: String);
    fn clear(&self);
}

pub struct MultiLayerCache {
    l1: Arc<DashMap<String, String>>,
    l2: Arc<DashMap<String, String>>,
}

impl MultiLayerCache {
    pub fn new() -> Self {
        Self {
            l1: Arc::new(DashMap::new()),
            l2: Arc::new(DashMap::new()),
        }
    }

    pub fn get(&self, key: &str) -> Result<String> {
        if let Some(entry) = self.l1.get(key) {
            return Ok(entry.value().clone());
        }
        if let Some(entry) = self.l2.get(key) {
            return Ok(entry.value().clone());
        }
        Err(DataError::CacheMiss)
    }

    pub fn set(&self, key: String, value: String) {
        self.l1.insert(key, value);
    }
}

impl Default for MultiLayerCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cache_set_get() {
        let cache = MultiLayerCache::new();
        cache.set("k1".to_string(), "v1".to_string());
        assert!(cache.get("k1").is_ok());
    }
}
