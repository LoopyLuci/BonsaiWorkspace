/// Incremental cache layer for bug hunt findings.
/// Uses content-addressed storage with BLAKE3 hashing.

use anyhow::Result;
use blake3::Hasher;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::finding::Finding;

/// A cached entry for a file's scan results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub file_path: PathBuf,
    pub file_hash: String,
    pub timestamp: i64,
    pub findings: Vec<Finding>,
}

/// In-memory cache with optional persistent storage.
pub struct ScanCache {
    cache_dir: PathBuf,
    memory_cache: HashMap<String, CacheEntry>,
}

impl ScanCache {
    /// Create or open a cache in the given directory.
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&cache_dir)?;
        let mut cache = ScanCache {
            cache_dir,
            memory_cache: HashMap::new(),
        };
        cache.load()?;
        Ok(cache)
    }

    /// Compute BLAKE3 hash of a file.
    pub fn hash_file(path: &Path) -> Result<String> {
        let contents = fs::read(path)?;
        let hash = blake3::hash(&contents);
        Ok(hash.to_hex().to_string())
    }

    /// Compute hash of a file's metadata (size, modified time).
    pub fn hash_metadata(path: &Path) -> Result<String> {
        let metadata = fs::metadata(path)?;
        let size = metadata.len();
        let modified = metadata.modified()?;
        let modified_secs = modified
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let mut hasher = Hasher::new();
        hasher.update(&size.to_le_bytes());
        hasher.update(&modified_secs.to_le_bytes());
        Ok(hasher.finalize().to_hex().to_string())
    }

    /// Check if a file is in cache and unchanged.
    pub fn is_cached(&self, file_path: &Path) -> bool {
        if let Ok(current_hash) = Self::hash_file(file_path) {
            let key = file_path.to_string_lossy().to_string();
            if let Some(entry) = self.memory_cache.get(&key) {
                return entry.file_hash == current_hash;
            }
        }
        false
    }

    /// Get cached findings for a file (if present and valid).
    pub fn get(&self, file_path: &Path) -> Option<Vec<Finding>> {
        let key = file_path.to_string_lossy().to_string();
        self.memory_cache.get(&key).map(|e| e.findings.clone())
    }

    /// Store findings for a file in the cache.
    pub fn put(&mut self, file_path: PathBuf, findings: Vec<Finding>) -> Result<()> {
        let file_hash = Self::hash_file(&file_path)?;
        let timestamp = chrono::Utc::now().timestamp();
        let key = file_path.to_string_lossy().to_string();

        let entry = CacheEntry {
            file_path: file_path.clone(),
            file_hash,
            timestamp,
            findings,
        };

        self.memory_cache.insert(key, entry);
        Ok(())
    }

    /// Persist cache to disk.
    pub fn save(&self) -> Result<()> {
        let manifest_path = self.cache_dir.join("manifest.json");
        let entries: Vec<_> = self.memory_cache.values().cloned().collect();
        let json = serde_json::to_string_pretty(&entries)?;
        fs::write(&manifest_path, json)?;
        info!("Saved cache to {:?}", manifest_path);
        Ok(())
    }

    /// Load cache from disk.
    fn load(&mut self) -> Result<()> {
        let manifest_path = self.cache_dir.join("manifest.json");
        if manifest_path.exists() {
            let json = fs::read_to_string(&manifest_path)?;
            if let Ok(entries) = serde_json::from_str::<Vec<CacheEntry>>(&json) {
                for entry in entries {
                    let key = entry.file_path.to_string_lossy().to_string();
                    self.memory_cache.insert(key, entry);
                }
                debug!("Loaded {} cache entries", self.memory_cache.len());
            }
        }
        Ok(())
    }

    /// Clear the cache.
    pub fn clear(&mut self) -> Result<()> {
        self.memory_cache.clear();
        fs::remove_dir_all(&self.cache_dir)?;
        fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }

    /// Get cache statistics.
    pub fn stats(&self) -> (usize, usize) {
        let total_files = self.memory_cache.len();
        let total_findings: usize = self
            .memory_cache
            .values()
            .map(|e| e.findings.len())
            .sum();
        (total_files, total_findings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_creation() -> Result<()> {
        let temp_dir = std::env::temp_dir().join("bonsai_cache_test");
        let _ = fs::remove_dir_all(&temp_dir);
        let cache = ScanCache::new(temp_dir)?;
        assert_eq!(cache.memory_cache.len(), 0);
        Ok(())
    }
}
