use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct DedupCache {
    inner:    Mutex<HashMap<String, Instant>>,
    capacity: usize,
    ttl:      Duration,
}

impl DedupCache {
    pub fn new(capacity: usize, ttl_secs: u64) -> Self {
        Self {
            inner:    Mutex::new(HashMap::with_capacity(capacity.min(1024))),
            capacity,
            ttl:      Duration::from_secs(ttl_secs),
        }
    }

    /// Returns `true` if this event is a duplicate (already seen within TTL).
    pub fn is_duplicate(&self, platform: &str, event_id: &str) -> bool {
        let key = format!("{platform}:{event_id}");
        let mut cache = self.inner.lock().unwrap();

        if let Some(ts) = cache.get(&key) {
            if ts.elapsed() < self.ttl {
                return true;
            }
        }

        // Evict expired entries before inserting when at capacity
        if cache.len() >= self.capacity {
            let ttl = self.ttl;
            cache.retain(|_, ts| ts.elapsed() < ttl);
        }

        cache.insert(key, Instant::now());
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_event_not_duplicate() {
        let cache = DedupCache::new(100, 60);
        assert!(!cache.is_duplicate("discord", "msg-1"));
    }

    #[test]
    fn second_call_same_event_is_duplicate() {
        let cache = DedupCache::new(100, 60);
        cache.is_duplicate("discord", "msg-2");
        assert!(cache.is_duplicate("discord", "msg-2"));
    }

    #[test]
    fn different_platforms_same_id_not_duplicate() {
        let cache = DedupCache::new(100, 60);
        cache.is_duplicate("discord", "msg-3");
        assert!(!cache.is_duplicate("telegram", "msg-3"));
    }

    #[test]
    fn ttl_zero_means_always_new() {
        let cache = DedupCache::new(100, 0);
        cache.is_duplicate("discord", "msg-4");
        // TTL=0: ts.elapsed() >= 0 == ttl, so not considered duplicate
        assert!(!cache.is_duplicate("discord", "msg-4"));
    }

    #[test]
    fn capacity_eviction_does_not_panic() {
        let cache = DedupCache::new(5, 60);
        for i in 0..20 {
            cache.is_duplicate("discord", &format!("msg-cap-{i}"));
        }
        // Just ensure no panic; cache is still functional
        assert!(!cache.is_duplicate("discord", "brand-new-id"));
    }
}

/// Fallback dedup key for emails without a Message-ID header.
pub fn email_fallback_key(from: &str, date: &str, subject: &str, body: &str) -> String {
    use sha2::{Digest, Sha256};
    let snippet = &body[..body.len().min(100)];
    let input = format!("{from}\0{date}\0{subject}\0{snippet}");
    hex::encode(Sha256::digest(input.as_bytes()))
}
