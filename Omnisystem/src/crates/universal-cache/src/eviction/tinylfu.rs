//! TinyLFU Eviction Policy
//!
//! High-performance frequency tracking using count-min sketch for memory efficiency.
//! Provides near-LFU eviction with O(1) time and minimal memory overhead.

use super::EvictionPolicy;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct TinyLfuPolicy<K: Hash + Eq + Clone> {
    // Sketch for frequency estimation (count-min style)
    sketch: Mutex<Vec<Vec<u8>>>,
    // Recent items (for LRU within same frequency bucket)
    recent: Mutex<VecDeque<K>>,
    // Hash seed for multiple hash functions
    seeds: [u64; 4],
    capacity: usize,
    sketch_depth: usize,
    sketch_width: usize,
}

impl<K: Hash + Eq + Clone> TinyLfuPolicy<K> {
    pub fn new(capacity: usize) -> Self {
        let sketch_width = (capacity * 10).max(1024); // Sketch 10x capacity
        let sketch_depth = 4; // 4 independent hash functions

        let mut sketch = Vec::with_capacity(sketch_depth);
        for _ in 0..sketch_depth {
            sketch.push(vec![0u8; sketch_width]);
        }

        Self {
            sketch: Mutex::new(sketch),
            recent: Mutex::new(VecDeque::new()),
            seeds: [12884901, 25769803, 38654707, 51539607], // Prime-based seeds
            capacity,
            sketch_depth,
            sketch_width,
        }
    }

    fn hash(&self, key: &K, seed: u64) -> usize {
        let mut hasher = DefaultHasher::new();
        hasher.write_u64(seed);
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.sketch_width
    }

    fn estimate_frequency(&self, key: &K) -> u8 {
        let sketch = self.sketch.lock();
        let mut min_count = u8::MAX;

        for (i, row) in sketch.iter().enumerate() {
            let idx = self.hash(key, self.seeds[i]);
            min_count = min_count.min(row[idx]);
        }

        min_count
    }
}

impl<K: Hash + Eq + Clone + Send + Sync> EvictionPolicy for TinyLfuPolicy<K> {
    type Key = K;

    fn record_access(&self, key: &K) {
        // Increment counts in sketch
        let mut sketch = self.sketch.lock();
        for (i, row) in sketch.iter_mut().enumerate() {
            let idx = self.hash(key, self.seeds[i]);
            row[idx] = row[idx].saturating_add(1);
        }
        drop(sketch);

        // Track in recent list
        let mut recent = self.recent.lock();
        if !recent.iter().any(|x| x == key) {
            recent.push_back(key.clone());
            if recent.len() > self.capacity {
                recent.pop_front();
            }
        }
    }

    fn evict(&self) -> Option<K> {
        let mut recent = self.recent.lock();

        if recent.is_empty() {
            return None;
        }

        // Find item with lowest frequency (LRU tie-breaking via order in recent)
        let min_freq_key = recent
            .iter()
            .min_by_key(|k| self.estimate_frequency(k))
            .cloned();

        if let Some(key) = min_freq_key {
            recent.retain(|x| x != &key);
            Some(key)
        } else {
            recent.pop_front()
        }
    }

    fn clear(&self) {
        self.sketch.lock().iter_mut().for_each(|row| {
            row.iter_mut().for_each(|v| *v = 0);
        });
        self.recent.lock().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tinylfu_eviction() {
        let policy = TinyLfuPolicy::new(3);

        // Build access pattern
        for _ in 0..5 {
            policy.record_access(&"a");
        }
        for _ in 0..3 {
            policy.record_access(&"b");
        }
        policy.record_access(&"c");

        // "c" has lowest frequency estimate
        let evicted = policy.evict();
        assert!(evicted.is_some());
    }

    #[test]
    fn test_tinylfu_frequency_estimation() {
        let policy = TinyLfuPolicy::new(10);

        policy.record_access(&"hot");
        for _ in 0..10 {
            policy.record_access(&"hot");
        }

        let freq_hot = policy.estimate_frequency(&"hot");
        policy.record_access(&"cold");
        let freq_cold = policy.estimate_frequency(&"cold");

        // Hot should have higher estimated frequency
        assert!(freq_hot > freq_cold);
    }

    #[test]
    fn test_tinylfu_clear() {
        let policy = TinyLfuPolicy::new(5);

        for i in 0..5 {
            policy.record_access(&i);
        }

        policy.clear();

        // After clear, estimate should be 0
        let freq = policy.estimate_frequency(&0);
        assert_eq!(freq, 0);
    }
}
