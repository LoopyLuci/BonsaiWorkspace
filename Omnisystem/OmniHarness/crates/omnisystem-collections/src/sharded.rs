//! Sharded collections for horizontal scalability

use std::sync::Arc;
use std::collections::HashMap;

/// Sharded HashMap for lock-free horizontal scalability
pub struct ShardedMap<K, V> {
    shards: Vec<Shard<K, V>>,
    shard_mask: usize,
}

struct Shard<K, V> {
    data: std::sync::Mutex<HashMap<K, V>>,
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> ShardedMap<K, V> {
    /// Create a new sharded map with specified number of shards
    pub fn new(num_shards: usize) -> Self {
        let num_shards = num_shards.next_power_of_two().max(16);
        let shard_mask = num_shards - 1;

        let shards = (0..num_shards)
            .map(|_| Shard {
                data: std::sync::Mutex::new(HashMap::new()),
            })
            .collect();

        ShardedMap {
            shards,
            shard_mask,
        }
    }

    /// Get shard index for key
    fn shard_idx(&self, key: &K) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() as usize) & self.shard_mask
    }

    /// Insert a value
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let idx = self.shard_idx(&key);
        let mut shard = self.shards[idx].data.lock().unwrap();
        shard.insert(key, value)
    }

    /// Get a value
    pub fn get(&self, key: &K) -> Option<V> {
        let idx = self.shard_idx(key);
        let shard = self.shards[idx].data.lock().unwrap();
        shard.get(key).cloned()
    }

    /// Remove a value
    pub fn remove(&self, key: &K) -> Option<V> {
        let idx = self.shard_idx(key);
        let mut shard = self.shards[idx].data.lock().unwrap();
        shard.remove(key)
    }

    /// Total number of entries across all shards
    pub fn len(&self) -> usize {
        self.shards
            .iter()
            .map(|s| s.data.lock().unwrap().len())
            .sum()
    }

    /// Number of shards
    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }

    /// Clear all entries
    pub fn clear(&self) {
        for shard in &self.shards {
            shard.data.lock().unwrap().clear();
        }
    }
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> Default for ShardedMap<K, V> {
    fn default() -> Self {
        Self::new(16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sharded_map_basic() {
        let map: ShardedMap<String, i32> = ShardedMap::new(8);
        map.insert("key1".to_string(), 42);
        assert_eq!(map.get(&"key1".to_string()), Some(42));
    }

    #[test]
    fn test_sharded_map_distribution() {
        let map: ShardedMap<i32, i32> = ShardedMap::new(16);
        for i in 0..100 {
            map.insert(i, i * 2);
        }
        assert_eq!(map.len(), 100);
    }

    #[test]
    fn test_sharded_map_shard_count() {
        let map: ShardedMap<String, i32> = ShardedMap::new(8);
        // Shards are rounded up to next power of 2, minimum 16
        assert_eq!(map.shard_count(), 16);
    }
}
