//! Local caching with TTL for knowledge base results
//!
//! Improves performance by caching remote lookups with configurable TTL.

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use dashmap::DashMap;
use ahf_core::VerificationResult;

/// Cache entry with TTL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// The cached verification result
    pub result: VerificationResult,
    /// When this entry expires
    pub expires_at: SystemTime,
}

impl CacheEntry {
    /// Check if this entry is still valid
    pub fn is_valid(&self) -> bool {
        SystemTime::now() < self.expires_at
    }
}

/// Thread-safe cache with TTL
pub struct Cache {
    entries: DashMap<String, CacheEntry>,
    default_ttl: Duration,
}

impl Cache {
    /// Create a new cache
    pub fn new() -> Self {
        Cache {
            entries: DashMap::new(),
            default_ttl: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Create with custom TTL
    pub fn with_ttl(ttl: Duration) -> Self {
        Cache {
            entries: DashMap::new(),
            default_ttl: ttl,
        }
    }

    /// Generate cache key from claim text
    pub fn make_key(claim_text: &str) -> String {
        blake3::hash(claim_text.as_bytes()).to_hex().to_string()
    }

    /// Insert entry into cache
    pub fn insert(&self, key: String, result: VerificationResult) {
        let entry = CacheEntry {
            result,
            expires_at: SystemTime::now() + self.default_ttl,
        };
        self.entries.insert(key, entry);
    }

    /// Get entry from cache (if not expired)
    pub fn get(&self, key: &str) -> Option<VerificationResult> {
        if let Some(entry_ref) = self.entries.get(key) {
            if entry_ref.is_valid() {
                return Some(entry_ref.result.clone());
            } else {
                // Drop expired entry
                drop(entry_ref);
                self.entries.remove(key);
            }
        }
        None
    }

    /// Clear all expired entries
    pub fn cleanup_expired(&self) {
        self.entries.retain(|_, entry| entry.is_valid());
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            total_entries: self.entries.len(),
        }
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.entries.clear();
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total number of entries in cache
    pub total_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ahf_core::VerificationStatus;

    #[test]
    fn test_cache_entry_valid() {
        let entry = CacheEntry {
            result: VerificationResult {
                status: VerificationStatus::Valid,
                proof: None,
                reasoning: "test".to_string(),
                confidence: 0.95,
            },
            expires_at: SystemTime::now() + Duration::from_secs(60),
        };

        assert!(entry.is_valid());
    }

    #[test]
    fn test_cache_entry_expired() {
        let entry = CacheEntry {
            result: VerificationResult {
                status: VerificationStatus::Valid,
                proof: None,
                reasoning: "test".to_string(),
                confidence: 0.95,
            },
            expires_at: SystemTime::now() - Duration::from_secs(1),
        };

        assert!(!entry.is_valid());
    }

    #[test]
    fn test_cache_insert_and_get() {
        let cache = Cache::new();
        let key = "test_key".to_string();
        let result = VerificationResult {
            status: VerificationStatus::Valid,
            proof: None,
            reasoning: "test".to_string(),
            confidence: 0.95,
        };

        cache.insert(key.clone(), result.clone());
        let retrieved = cache.get(&key).unwrap();
        assert_eq!(retrieved.status, VerificationStatus::Valid);
    }

    #[test]
    fn test_cache_key_generation() {
        let key1 = Cache::make_key("test text");
        let key2 = Cache::make_key("test text");
        assert_eq!(key1, key2); // Deterministic
    }

    #[test]
    fn test_cache_cleanup() {
        let cache = Cache::with_ttl(Duration::from_millis(100));
        let key = "test_key".to_string();
        let result = VerificationResult {
            status: VerificationStatus::Valid,
            proof: None,
            reasoning: "test".to_string(),
            confidence: 0.95,
        };

        cache.insert(key.clone(), result);

        // Entry should be valid
        assert!(cache.get(&key).is_some());

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));

        // Cleanup
        cache.cleanup_expired();

        // Entry should be gone
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn test_cache_stats() {
        let cache = Cache::new();
        cache.insert(
            "key1".to_string(),
            VerificationResult {
                status: VerificationStatus::Valid,
                proof: None,
                reasoning: "test".to_string(),
                confidence: 0.95,
            },
        );
        cache.insert(
            "key2".to_string(),
            VerificationResult {
                status: VerificationStatus::Inconclusive,
                proof: None,
                reasoning: "test".to_string(),
                confidence: 0.5,
            },
        );

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 2);
    }

    #[test]
    fn test_cache_clear() {
        let cache = Cache::new();
        cache.insert(
            "key1".to_string(),
            VerificationResult {
                status: VerificationStatus::Valid,
                proof: None,
                reasoning: "test".to_string(),
                confidence: 0.95,
            },
        );

        assert_eq!(cache.stats().total_entries, 1);
        cache.clear();
        assert_eq!(cache.stats().total_entries, 0);
    }
}
