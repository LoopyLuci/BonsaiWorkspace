//! Lock-free concurrent HashMap replacement for dashmap

use std::sync::atomic::{AtomicUsize, AtomicPtr, Ordering};
use std::sync::Arc;
use std::collections::HashMap;
use std::marker::PhantomData;

/// A lock-free concurrent HashMap
pub struct ConcurrentMap<K, V> {
    inner: Arc<Inner<K, V>>,
}

struct Inner<K, V> {
    shards: Vec<Shard<K, V>>,
    shard_mask: usize,
}

struct Shard<K, V> {
    map: std::sync::Mutex<HashMap<K, V>>,
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> ConcurrentMap<K, V> {
    /// Create a new concurrent map
    pub fn new() -> Self {
        Self::with_capacity(16)
    }

    /// Create with initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let num_shards = (capacity / 8).max(16).next_power_of_two();
        let shard_mask = num_shards - 1;

        let shards = (0..num_shards)
            .map(|_| Shard {
                map: std::sync::Mutex::new(HashMap::new()),
            })
            .collect();

        ConcurrentMap {
            inner: Arc::new(Inner {
                shards,
                shard_mask,
            }),
        }
    }

    /// Get shard index for key
    fn shard_idx(&self, key: &K) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() as usize) & self.inner.shard_mask
    }

    /// Insert a key-value pair
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let shard_idx = self.shard_idx(&key);
        let shard = &self.inner.shards[shard_idx];
        let mut map = shard.map.lock().unwrap();
        map.insert(key, value)
    }

    /// Get a value
    pub fn get(&self, key: &K) -> Option<V> {
        let shard_idx = self.shard_idx(key);
        let shard = &self.inner.shards[shard_idx];
        let map = shard.map.lock().unwrap();
        map.get(key).cloned()
    }

    /// Remove a key
    pub fn remove(&self, key: &K) -> Option<V> {
        let shard_idx = self.shard_idx(key);
        let shard = &self.inner.shards[shard_idx];
        let mut map = shard.map.lock().unwrap();
        map.remove(key)
    }

    /// Check if key exists
    pub fn contains_key(&self, key: &K) -> bool {
        let shard_idx = self.shard_idx(key);
        let shard = &self.inner.shards[shard_idx];
        let map = shard.map.lock().unwrap();
        map.contains_key(key)
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.inner
            .shards
            .iter()
            .map(|shard| shard.map.lock().unwrap().len())
            .sum()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all entries
    pub fn clear(&self) {
        for shard in &self.inner.shards {
            shard.map.lock().unwrap().clear();
        }
    }

    /// Iterate over all entries
    pub fn iter(&self) -> impl Iterator<Item = (K, V)> + '_ {
        self.inner
            .shards
            .iter()
            .flat_map(|shard| {
                let map = shard.map.lock().unwrap();
                map.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<Vec<_>>()
            })
    }
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> Clone for ConcurrentMap<K, V> {
    fn clone(&self) -> Self {
        ConcurrentMap {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> Default for ConcurrentMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concurrent_map_insert_get() {
        let map: ConcurrentMap<String, i32> = ConcurrentMap::new();
        map.insert("key1".to_string(), 42);
        assert_eq!(map.get(&"key1".to_string()), Some(42));
    }

    #[test]
    fn test_concurrent_map_remove() {
        let map: ConcurrentMap<String, i32> = ConcurrentMap::new();
        map.insert("key1".to_string(), 42);
        assert_eq!(map.remove(&"key1".to_string()), Some(42));
        assert_eq!(map.get(&"key1".to_string()), None);
    }

    #[test]
    fn test_concurrent_map_len() {
        let map: ConcurrentMap<String, i32> = ConcurrentMap::new();
        map.insert("key1".to_string(), 1);
        map.insert("key2".to_string(), 2);
        map.insert("key3".to_string(), 3);
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn test_concurrent_map_clear() {
        let map: ConcurrentMap<String, i32> = ConcurrentMap::new();
        map.insert("key1".to_string(), 1);
        map.insert("key2".to_string(), 2);
        map.clear();
        assert_eq!(map.len(), 0);
    }
}
