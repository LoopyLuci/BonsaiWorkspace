use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct QueryCache {
    cache: Arc<DashMap<String, CachedResult>>,
    ttl: Duration,
}

#[derive(Debug, Clone)]
pub struct CachedResult {
    pub result: String,
    pub timestamp: Instant,
}

impl QueryCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    pub fn get(&self, query: &str) -> Option<String> {
        if let Some(entry) = self.cache.get(query) {
            if entry.timestamp.elapsed() < self.ttl {
                return Some(entry.result.clone());
            } else {
                drop(entry);
                self.cache.remove(query);
            }
        }
        None
    }

    pub fn set(&self, query: String, result: String) {
        self.cache.insert(query, CachedResult {
            result,
            timestamp: Instant::now(),
        });
    }

    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    pub fn clear(&self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_cache() {
        let cache = QueryCache::new(60);
        cache.set("test".to_string(), "result".to_string());
        assert_eq!(cache.get("test"), Some("result".to_string()));
    }
}
