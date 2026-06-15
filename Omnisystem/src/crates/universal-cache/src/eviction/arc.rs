//! Adaptive Replacement Cache (ARC) Eviction Policy
//!
//! Self-tuning cache that balances between recency (LRU) and frequency (LFU).
//! Maintains ghost lists for evicted items to adapt the cache's behavior.

use super::EvictionPolicy;
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct ArcPolicy<K: Hash + Eq + Clone> {
    // Recent items (accessed once, recently)
    t1: Mutex<VecDeque<K>>,
    // Frequent items (accessed multiple times)
    t2: Mutex<VecDeque<K>>,
    // Ghost recent items (evicted from T1)
    b1: Mutex<VecDeque<K>>,
    // Ghost frequent items (evicted from T2)
    b2: Mutex<VecDeque<K>>,

    // Adaptation parameter (target size for T1)
    p: AtomicUsize,
    capacity: usize,
    size: AtomicUsize,
}

impl<K: Hash + Eq + Clone> ArcPolicy<K> {
    pub fn new(capacity: usize) -> Self {
        Self {
            t1: Mutex::new(VecDeque::new()),
            t2: Mutex::new(VecDeque::new()),
            b1: Mutex::new(VecDeque::new()),
            b2: Mutex::new(VecDeque::new()),
            p: AtomicUsize::new(0),
            capacity,
            size: AtomicUsize::new(0),
        }
    }
}

impl<K: Hash + Eq + Clone + Send + Sync> EvictionPolicy for ArcPolicy<K> {
    type Key = K;

    fn record_access(&self, key: &K) {
        let mut t1 = self.t1.lock();
        let mut t2 = self.t2.lock();
        let mut b1 = self.b1.lock();
        let mut b2 = self.b2.lock();

        // Case 1: Hit in T1 (move to T2 as it's now frequent)
        if let Some(pos) = t1.iter().position(|x| x == key) {
            t1.remove(pos);
            t2.push_back(key.clone());
            return;
        }

        // Case 2: Hit in T2 (move to back as it's more recent)
        if let Some(pos) = t2.iter().position(|x| x == key) {
            t2.remove(pos);
            t2.push_back(key.clone());
            return;
        }

        // Case 3: Miss - determine if from ghost lists and adapt
        let ghost_hit_b1 = b1.iter().any(|x| x == key);
        let ghost_hit_b2 = b2.iter().any(|x| x == key);

        if ghost_hit_b1 {
            // Increase T1 target size (recency is valuable)
            let b2_len = b2.len();
            let b1_len = b1.len();
            let delta = if b1_len > 0 { b2_len / b1_len } else { 1 };
            let p = self.p.load(Ordering::Relaxed);
            self.p.store((p + delta).min(self.capacity), Ordering::Relaxed);
            b1.retain(|x| x != key);
        } else if ghost_hit_b2 {
            // Decrease T1 target size (frequency is valuable)
            let b1_len = b1.len();
            let b2_len = b2.len();
            let delta = if b2_len > 0 { b1_len / b2_len } else { 1 };
            let p = self.p.load(Ordering::Relaxed);
            self.p.store(p.saturating_sub(delta), Ordering::Relaxed);
            b2.retain(|x| x != key);
        }

        // Evict if cache is full
        let total = t1.len() + t2.len();
        if total >= self.capacity {
            let p = self.p.load(Ordering::Relaxed);
            if t1.len() > p {
                if let Some(evicted) = t1.pop_front() {
                    b1.push_back(evicted);
                    if b1.len() > self.capacity {
                        b1.pop_front();
                    }
                }
            } else {
                if let Some(evicted) = t2.pop_front() {
                    b2.push_back(evicted);
                    if b2.len() > self.capacity {
                        b2.pop_front();
                    }
                }
            }
        }

        // Add new access to T1 (assume first access is recent)
        t1.push_back(key.clone());
        self.size.fetch_add(1, Ordering::Relaxed);
    }

    fn evict(&self) -> Option<K> {
        let mut t1 = self.t1.lock();
        let mut t2 = self.t2.lock();

        let p = self.p.load(Ordering::Relaxed);
        if t1.len() > p {
            if let Some(key) = t1.pop_front() {
                self.size.fetch_sub(1, Ordering::Relaxed);
                return Some(key);
            }
        } else {
            if let Some(key) = t2.pop_front() {
                self.size.fetch_sub(1, Ordering::Relaxed);
                return Some(key);
            }
        }
        None
    }

    fn clear(&self) {
        self.t1.lock().clear();
        self.t2.lock().clear();
        self.b1.lock().clear();
        self.b2.lock().clear();
        self.p.store(0, Ordering::Relaxed);
        self.size.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_eviction() {
        let policy = ArcPolicy::new(3);

        policy.record_access(&"a");
        policy.record_access(&"b");
        policy.record_access(&"c");

        // T1: [a, b, c], T2: []
        let evicted = policy.evict();
        assert_eq!(evicted, Some("a"));
    }

    #[test]
    fn test_arc_frequency_detection() {
        let policy = ArcPolicy::new(3);

        policy.record_access(&"a");
        policy.record_access(&"a"); // Access "a" again
        policy.record_access(&"b");
        policy.record_access(&"c");

        // "a" should be in T2 (frequent), so it won't be evicted first
        let evicted = policy.evict();
        assert!(evicted != Some("a"));
    }

    #[test]
    fn test_arc_adaptation() {
        let policy = ArcPolicy::new(5);

        // Simulate workload preferring recency
        for i in 0..10 {
            policy.record_access(&i);
        }

        // P should have adapted
        let p = policy.p.load(Ordering::Relaxed);
        assert!(p > 0);
    }
}
