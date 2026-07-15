//! Least Recently Used (LRU) Eviction Policy
//!
//! Thread-safe LRU implementation using a concurrent ordered map.
//! Maintains access order and evicts the least recently accessed item.

use super::EvictionPolicy;
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct LruPolicy<K: Hash + Eq + Clone + Ord> {
    // Track access order: (timestamp, key) -> ()
    access_order: Mutex<BTreeMap<(u64, K), ()>>,
    // Reverse index so we can find a key's current timestamp in O(log n)
    // instead of scanning every prior timestamp.
    key_timestamps: Mutex<std::collections::HashMap<K, u64>>,
    current_time: AtomicUsize,
    capacity: usize,
}

impl<K: Hash + Eq + Clone + Ord> LruPolicy<K> {
    pub fn new(capacity: usize) -> Self {
        Self {
            access_order: Mutex::new(BTreeMap::new()),
            key_timestamps: Mutex::new(std::collections::HashMap::new()),
            current_time: AtomicUsize::new(0),
            capacity,
        }
    }

    fn next_timestamp(&self) -> u64 {
        self.current_time.fetch_add(1, Ordering::Relaxed) as u64
    }
}

impl<K: Hash + Eq + Clone + Ord + Send + Sync> EvictionPolicy for LruPolicy<K> {
    type Key = K;

    fn record_access(&self, key: &K) {
        let timestamp = self.next_timestamp();
        let mut order = self.access_order.lock();
        let mut timestamps = self.key_timestamps.lock();

        // Remove old entry if it exists, using the reverse index instead of
        // scanning every prior timestamp.
        if let Some(old_ts) = timestamps.get(key) {
            order.remove(&(*old_ts, key.clone()));
        }

        // Add with new timestamp
        order.insert((timestamp, key.clone()), ());
        timestamps.insert(key.clone(), timestamp);
    }

    fn evict(&self) -> Option<K> {
        let mut order = self.access_order.lock();
        let mut timestamps = self.key_timestamps.lock();

        if let Some((&(ts, ref key), _)) = order.iter().next() {
            let key_clone = key.clone();
            order.remove(&(ts, key_clone.clone()));
            timestamps.remove(&key_clone);
            Some(key_clone)
        } else {
            None
        }
    }

    fn clear(&self) {
        self.access_order.lock().clear();
        self.key_timestamps.lock().clear();
        self.current_time.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_eviction_order() {
        let policy = LruPolicy::new(3);

        // Access in order
        policy.record_access(&"a");
        policy.record_access(&"b");
        policy.record_access(&"c");

        // Most recent should be "c", least recent "a"
        assert_eq!(policy.evict(), Some("a"));
    }

    #[test]
    fn test_lru_update_order() {
        let policy = LruPolicy::new(3);

        policy.record_access(&"a");
        policy.record_access(&"b");
        policy.record_access(&"a"); // Re-access "a"

        // "b" is now least recently used
        assert_eq!(policy.evict(), Some("b"));
    }
}
