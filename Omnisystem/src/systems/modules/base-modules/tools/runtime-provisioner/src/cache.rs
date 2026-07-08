//! Runtime caching system

use crate::{RuntimeType, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub runtime_type: RuntimeType,
    pub version: String,
    pub location: PathBuf,
    pub checksum: String,
    pub size_bytes: u64,
    pub cached_at: chrono::DateTime<chrono::Utc>,
}

/// Thread-safe runtime cache
pub struct Cache {
    entries: dashmap::DashMap<String, CacheEntry>,
    cache_dir: PathBuf,
}

impl Cache {
    /// Create new cache
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self {
            entries: dashmap::DashMap::new(),
            cache_dir,
        })
    }

    /// Add entry to cache
    pub fn add(&self, entry: CacheEntry) {
        let key = format!("{}:{}", entry.runtime_type.name(), entry.version);
        self.entries.insert(key, entry);
    }

    /// Get cache entry
    pub fn get(&self, runtime_type: RuntimeType, version: &str) -> Option<CacheEntry> {
        let key = format!("{}:{}", runtime_type.name(), version);
        self.entries.get(&key).map(|e| e.clone())
    }

    /// Check if runtime is cached
    pub fn exists(&self, runtime_type: RuntimeType, version: &str) -> bool {
        let key = format!("{}:{}", runtime_type.name(), version);
        self.entries.contains_key(&key)
    }

    /// List all cached entries
    pub fn list(&self) -> Vec<CacheEntry> {
        self.entries.iter().map(|e| e.value().clone()).collect()
    }

    /// Remove entry from cache
    pub fn remove(&self, runtime_type: RuntimeType, version: &str) {
        let key = format!("{}:{}", runtime_type.name(), version);
        self.entries.remove(&key);
    }

    /// Get total cache size in bytes
    pub fn total_size(&self) -> u64 {
        self.entries.iter().map(|e| e.value().size_bytes).sum()
    }

    /// Clear cache
    pub fn clear(&self) -> Result<()> {
        self.entries.clear();
        std::fs::remove_dir_all(&self.cache_dir)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }

    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }
}

/// Cache manager with persistence
pub struct CacheManager {
    cache: Cache,
    manifest_path: PathBuf,
}

impl CacheManager {
    /// Create cache manager
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        let cache = Cache::new(cache_dir.clone())?;
        let manifest_path = cache_dir.join("manifest.json");

        Ok(Self {
            cache,
            manifest_path,
        })
    }

    /// Save cache to disk
    pub fn save(&self) -> Result<()> {
        let entries: Vec<CacheEntry> = self.cache.list();
        let json = serde_json::to_string_pretty(&entries)?;
        std::fs::write(&self.manifest_path, json)?;
        Ok(())
    }

    /// Load cache from disk
    pub fn load(&self) -> Result<()> {
        if !self.manifest_path.exists() {
            return Ok(());
        }

        let json = std::fs::read_to_string(&self.manifest_path)?;
        let entries: Vec<CacheEntry> = serde_json::from_str(&json)?;

        for entry in entries {
            self.cache.add(entry);
        }

        Ok(())
    }

    pub fn cache(&self) -> &Cache {
        &self.cache
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_creation() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::new(dir.path().to_path_buf());
        assert!(cache.is_ok());
    }

    #[test]
    fn test_cache_add_and_get() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::new(dir.path().to_path_buf()).unwrap();

        let entry = CacheEntry {
            runtime_type: RuntimeType::Rust,
            version: "1.75.0".to_string(),
            location: PathBuf::from("/tmp/rust"),
            checksum: "abc123".to_string(),
            size_bytes: 1024,
            cached_at: chrono::Utc::now(),
        };

        cache.add(entry.clone());
        let retrieved = cache.get(RuntimeType::Rust, "1.75.0");
        assert!(retrieved.is_some());
    }
}
