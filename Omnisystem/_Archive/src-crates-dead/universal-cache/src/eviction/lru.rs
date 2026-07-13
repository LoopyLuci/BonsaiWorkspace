//! Least Recently Used (LRU) Eviction Policy
//!
//! Thread-safe LRU implementation using a concurrent ordered map.
//! Maintains access order and evicts the least recently accessed item.

use super::EvictionPolicy;
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct LruPolicy<K: Hash + Eq + Clone> {
    // Track access order: (timestamp, key) -> ()
    access_order: Mutex<BTreeMap<(u64, K), ()>>,
    current_time: AtomicUsize,
    capacity: usize,
}

impl<K: Hash + Eq + Clone> LruPolicy<K> {
    pub fn new(capacity: usize) -> Self {
        Self {
            access_order: Mutex::new(BTreeMap::new()),
            current_time: AtomicUsize::new(0),
            capacity,
        }
    }

    fn next_timestamp(&self) -> u64 {
        self.current_time.fetch_add(1, Ordering::Relaxed) as u64
    }
}

impl<K: Hash + Eq + Clone + Send + Sync> EvictionPolicy for LruPolicy<K> {
    type Key = K;

    fn record_access(&self, key: &K) {
        let timestamp = self.next_timestamp();
        let mut order = self.access_order.lock();

        // Remove old entry if exists
        for t in 0..self.current_time.load(Ordering::Relaxed) as u64 {
            if let Some(_) = order.remove(&(t, key.clone())) {
                break;
            }
        }

        // Add with new timestamp
        order.insert((timestamp, key.clone()), ());
    }

    fn evict(&self) -> Option<K> {
        let mut order = self.access_order.lock();
        if let Some(((_, key), _)) = order.iter().next() {
            let key_clone = key.clone();
            order.remove(&(0, key_clone.clone()));
            Some(key_clone)
        } else {
            None
        }
    }

    fn clear(&self) {
        self.access_order.lock().clear();
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
