//! Least Frequently Used (LFU) Eviction Policy
//!
//! Tracks access frequency and evicts items with the lowest frequency.
//! Uses timestamp for tie-breaking (LRU within same frequency).

use super::EvictionPolicy;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct LfuPolicy<K: Hash + Eq + Clone> {
    frequencies: Mutex<HashMap<K, usize>>,
    access_times: Mutex<HashMap<K, u64>>,
    current_time: AtomicUsize,
    capacity: usize,
}

impl<K: Hash + Eq + Clone> LfuPolicy<K> {
    pub fn new(capacity: usize) -> Self {
        Self {
            frequencies: Mutex::new(HashMap::new()),
            access_times: Mutex::new(HashMap::new()),
            current_time: AtomicUsize::new(0),
            capacity,
        }
    }

    fn next_timestamp(&self) -> u64 {
        self.current_time.fetch_add(1, Ordering::Relaxed) as u64
    }
}

impl<K: Hash + Eq + Clone + Send + Sync> EvictionPolicy for LfuPolicy<K> {
    type Key = K;

    fn record_access(&self, key: &K) {
        let timestamp = self.next_timestamp();
        let mut frequencies = self.frequencies.lock();
        let mut access_times = self.access_times.lock();

        // Increment frequency or initialize to 1
        let freq = frequencies.entry(key.clone()).or_insert(0);
        *freq += 1;

        // Update access time
        access_times.insert(key.clone(), timestamp);
    }

    fn evict(&self) -> Option<K> {
        let mut frequencies = self.frequencies.lock();
        let mut access_times = self.access_times.lock();

        if frequencies.is_empty() {
            return None;
        }

        // Find key with minimum frequency (ties broken by LRU via timestamp)
        let evict_key = frequencies
            .iter()
            .min_by(|a, b| {
                let cmp = a.1.cmp(b.1);
                if cmp == std::cmp::Ordering::Equal {
                    // Tie: use LRU (earlier timestamp = evict first)
                    let time_a = access_times.get(a.0).copied().unwrap_or(u64::MAX);
                    let time_b = access_times.get(b.0).copied().unwrap_or(u64::MAX);
                    time_a.cmp(&time_b)
                } else {
                    cmp
                }
            })
            .map(|(k, _)| k.clone());

        if let Some(key) = evict_key {
            frequencies.remove(&key);
            access_times.remove(&key);
            Some(key)
        } else {
            None
        }
    }

    fn clear(&self) {
        self.frequencies.lock().clear();
        self.access_times.lock().clear();
        self.current_time.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lfu_eviction() {
        let policy = LfuPolicy::new(3);

        policy.record_access(&"a");
        policy.record_access(&"b");
        policy.record_access(&"b");
        policy.record_access(&"c");
        policy.record_access(&"c");
        policy.record_access(&"c");

        // Frequencies: a=1, b=2, c=3
        // "a" is least frequently used
        assert_eq!(policy.evict(), Some("a"));
    }

    #[test]
    fn test_lfu_tie_breaking() {
        let policy = LfuPolicy::new(3);

        policy.record_access(&"a");
        policy.record_access(&"b");
        policy.record_access(&"c");

        // All have frequency 1, "a" accessed first (LRU)
        assert_eq!(policy.evict(), Some("a"));
    }

    #[test]
    fn test_lfu_frequency_update() {
        let policy = LfuPolicy::new(2);

        policy.record_access(&"a");
        policy.record_access(&"a");
        policy.record_access(&"b");

        // a=2, b=1; "b" should evict first
        assert_eq!(policy.evict(), Some("b"));
        assert_eq!(policy.evict(), Some("a"));
    }
}
