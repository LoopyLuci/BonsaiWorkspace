use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub key: String,
    pub value: Vec<u8>,
}

pub struct KernelCacheManager {
    entries: Arc<DashMap<String, CacheEntry>>,
}

impl KernelCacheManager {
    pub fn new() -> Self {
        Self { entries: Arc::new(DashMap::new()) }
    }
    
    pub fn set(&self, key: String, value: Vec<u8>) {
        let entry = CacheEntry { key: key.clone(), value };
        self.entries.insert(key, entry);
    }
    
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.entries.get(key).map(|e| e.value.clone())
    }
    
    pub fn remove(&self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }
    
    pub fn clear(&self) {
        self.entries.clear()
    }
    
    pub fn size(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_set_and_get() {
        let cache = KernelCacheManager::new();
        cache.set("key1".to_string(), vec![1, 2, 3]);
        assert_eq!(cache.get("key1"), Some(vec![1, 2, 3]));
    }
    
    #[test]
    fn test_remove() {
        let cache = KernelCacheManager::new();
        cache.set("key1".to_string(), vec![1, 2, 3]);
        assert!(cache.remove("key1"));
        assert_eq!(cache.get("key1"), None);
    }
}
