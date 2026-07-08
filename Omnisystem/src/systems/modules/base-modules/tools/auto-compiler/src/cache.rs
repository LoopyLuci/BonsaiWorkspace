//! Intelligent build cache system

use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub checksum: String,
    pub artifacts: Vec<PathBuf>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used: chrono::DateTime<chrono::Utc>,
    pub hit_count: u64,
}

/// Intelligent build cache
pub struct BuildCache {
    entries: dashmap::DashMap<String, CacheEntry>,
    cache_dir: PathBuf,
}

impl BuildCache {
    /// Create new build cache
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self {
            entries: dashmap::DashMap::new(),
            cache_dir,
        })
    }

    /// Get cached entry
    pub fn get(&self, key: &str) -> Option<CacheEntry> {
        self.entries.get(key).map(|e| e.clone())
    }

    /// Store cache entry
    pub fn store(&self, entry: CacheEntry) {
        self.entries.insert(entry.key.clone(), entry);
    }

    /// Check cache hit
    pub fn is_cached(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let entries = self
            .entries
            .iter()
            .map(|e| e.value().clone())
            .collect::<Vec<_>>();

        let total_hits = entries.iter().map(|e| e.hit_count).sum();
        let total_artifacts = entries.iter().map(|e| e.artifacts.len()).sum();

        CacheStats {
            entry_count: entries.len(),
            total_hits,
            total_artifacts,
            entries,
        }
    }

    /// Clear old cache entries (LRU)
    pub fn cleanup(&self, max_age_days: u64) -> Result<usize> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(max_age_days as i64);
        let mut removed = 0;

        self.entries.retain(|_, entry| {
            if entry.last_used < cutoff {
                removed += 1;
                false
            } else {
                true
            }
        });

        Ok(removed)
    }

    /// Clear all cache
    pub fn clear(&self) -> Result<()> {
        self.entries.clear();
        std::fs::remove_dir_all(&self.cache_dir)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub entry_count: usize,
    pub total_hits: u64,
    pub total_artifacts: usize,
    pub entries: Vec<CacheEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache = BuildCache::new(temp_dir.path().to_path_buf());
        assert!(cache.is_ok());
    }

    #[test]
    fn test_cache_store_and_retrieve() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache = BuildCache::new(temp_dir.path().to_path_buf()).unwrap();

        let entry = CacheEntry {
            key: "test:key".to_string(),
            checksum: "abc123".to_string(),
            artifacts: vec![],
            created_at: chrono::Utc::now(),
            last_used: chrono::Utc::now(),
            hit_count: 1,
        };

        cache.store(entry.clone());
        let retrieved = cache.get("test:key");
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_cache_stats() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache = BuildCache::new(temp_dir.path().to_path_buf()).unwrap();

        let entry = CacheEntry {
            key: "test:key".to_string(),
            checksum: "abc123".to_string(),
            artifacts: vec![],
            created_at: chrono::Utc::now(),
            last_used: chrono::Utc::now(),
            hit_count: 5,
        };

        cache.store(entry);
        let stats = cache.stats();
        assert_eq!(stats.entry_count, 1);
        assert_eq!(stats.total_hits, 5);
    }
}
