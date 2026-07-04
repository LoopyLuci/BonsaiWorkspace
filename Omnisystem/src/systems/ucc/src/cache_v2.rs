//! Phase 2C: Advanced Caching - Blake3-based Content-Addressed Storage
//!
//! Three-level cache hierarchy:
//! - L1 (Memory): Fast in-process cache with LRU eviction
//! - L2 (Disk): Persistent Blake3-indexed storage
//! - L3 (Remote): Optional S3/network cache for CI/distributed builds

use blake3::Hash;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use dashmap::DashMap;
use crate::error::Result;
use crate::core::CompileResult;

/// Blake3 hash digest with hex encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentHash(blake3::Hash);

impl ContentHash {
    /// Create from data bytes
    pub fn from_bytes(data: &[u8]) -> Self {
        ContentHash(blake3::hash(data))
    }

    /// Create from hex string
    pub fn from_hex(hex: &str) -> Option<Self> {
        let bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .filter_map(|i| u8::from_str_radix(&hex[i..i+2], 16).ok())
            .collect();

        if bytes.len() != 32 {
            return None;
        }

        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&bytes);
        Some(ContentHash(blake3::Hash::from(hash_bytes)))
    }

    /// Get hex representation
    pub fn hex(&self) -> String {
        self.0.to_hex().to_string()
    }

    /// Get short hash (first 12 chars)
    pub fn short(&self) -> String {
        self.hex()[..12].to_string()
    }
}

impl std::fmt::Display for ContentHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.short())
    }
}

/// Cached compilation result with metadata
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub hash: ContentHash,
    pub result: CompileResult,
    pub timestamp: std::time::SystemTime,
    pub hits: usize,
    pub size_bytes: usize,
}

/// L1 Cache: In-memory LRU cache
pub struct MemoryCache {
    entries: Arc<DashMap<ContentHash, CacheEntry>>,
    max_size_bytes: usize,
    current_size_bytes: Arc<parking_lot::Mutex<usize>>,
}

impl MemoryCache {
    /// Create new memory cache (default 512 MB)
    pub fn new(max_size_mb: usize) -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            max_size_bytes: max_size_mb * 1024 * 1024,
            current_size_bytes: Arc::new(parking_lot::Mutex::new(0)),
        }
    }

    /// Insert entry into cache
    pub fn insert(&self, hash: ContentHash, entry: CacheEntry) {
        let size = entry.size_bytes;
        self.entries.insert(hash, entry);

        let mut current = self.current_size_bytes.lock();
        *current += size;

        // Evict if over capacity (simple LRU: remove oldest)
        while *current > self.max_size_bytes && !self.entries.is_empty() {
            if let Some((_, entry)) = self.entries.iter().min_by_key(|e| {
                e.value().timestamp
            }).map(|e| (e.key().clone(), e.value().clone())) {
                self.entries.remove(&entry.hash);
                *current = current.saturating_sub(entry.size_bytes);
            } else {
                break;
            }
        }
    }

    /// Get entry from cache
    pub fn get(&self, hash: &ContentHash) -> Option<CacheEntry> {
        self.entries.get(hash).map(|e| e.clone())
    }

    /// Hit rate for this cache
    pub fn hit_rate(&self) -> f32 {
        let total_hits: usize = self.entries.iter().map(|e| e.value().hits).sum();
        if self.entries.is_empty() {
            0.0
        } else {
            total_hits as f32 / self.entries.len() as f32
        }
    }

    /// Clear cache
    pub fn clear(&self) {
        self.entries.clear();
        *self.current_size_bytes.lock() = 0;
    }

    /// Get cache stats
    pub fn stats(&self) -> CacheStats {
        let entries = self.entries.len();
        let total_hits: usize = self.entries.iter().map(|e| e.value().hits).sum();

        CacheStats {
            entries,
            total_hits,
            hit_rate: if entries > 0 { total_hits as f32 / entries as f32 } else { 0.0 },
            size_bytes: *self.current_size_bytes.lock(),
            max_size_bytes: self.max_size_bytes,
        }
    }
}

/// L2 Cache: Disk-based persistent storage with Blake3 indexing
pub struct DiskCache {
    cache_dir: PathBuf,
    index: Arc<DashMap<ContentHash, PathBuf>>,
}

impl DiskCache {
    /// Create disk cache at directory
    pub fn new(cache_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(cache_dir)?;
        Ok(Self {
            cache_dir: cache_dir.to_path_buf(),
            index: Arc::new(DashMap::new()),
        })
    }

    /// Store entry to disk
    pub fn store(&self, hash: &ContentHash, data: &[u8]) -> Result<()> {
        let path = self.cache_dir.join(hash.hex());
        std::fs::write(&path, data)?;
        self.index.insert(*hash, path);
        Ok(())
    }

    /// Retrieve entry from disk
    pub fn retrieve(&self, hash: &ContentHash) -> Result<Vec<u8>> {
        let path = self.cache_dir.join(hash.hex());
        let data = std::fs::read(&path)?;
        self.index.insert(*hash, path);
        Ok(data)
    }

    /// Check if entry exists
    pub fn exists(&self, hash: &ContentHash) -> bool {
        self.cache_dir.join(hash.hex()).exists()
    }

    /// Get cache size in bytes
    pub fn size_bytes(&self) -> u64 {
        std::fs::read_dir(&self.cache_dir)
            .unwrap_or_else(|_| std::fs::read_dir(".").unwrap())
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.metadata().ok())
            .map(|m| m.len())
            .sum()
    }

    /// Clear disk cache
    pub fn clear(&self) -> Result<()> {
        for entry in std::fs::read_dir(&self.cache_dir)? {
            if let Ok(entry) = entry {
                std::fs::remove_file(entry.path())?;
            }
        }
        self.index.clear();
        Ok(())
    }
}

/// L3 Cache: Remote storage (S3, HTTP, etc.)
pub struct RemoteCache {
    endpoint: String,
    enabled: bool,
}

impl RemoteCache {
    /// Create remote cache configuration
    pub fn new(endpoint: String) -> Self {
        let enabled = !endpoint.is_empty();
        Self {
            endpoint,
            enabled,
        }
    }

    /// Check if remote cache is available
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Store in remote (async - would use http/s3 client)
    pub async fn store(&self, _hash: &ContentHash, _data: &[u8]) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        // Placeholder for S3/HTTP storage
        // Would implement: PUT {endpoint}/{_hash.hex()} {_data}
        Ok(())
    }

    /// Retrieve from remote
    pub async fn retrieve(&self, _hash: &ContentHash) -> Result<Vec<u8>> {
        if !self.enabled {
            return Err(crate::error::Error::CacheError(
                "Remote cache disabled".to_string(),
            ));
        }
        // Placeholder for S3/HTTP retrieval
        // Would implement: GET {endpoint}/{_hash.hex()}
        Ok(vec![])
    }
}

/// Three-level unified cache system
pub struct CacheV2 {
    l1: MemoryCache,
    l2: DiskCache,
    l3: RemoteCache,
    stats: Arc<parking_lot::Mutex<CacheV2Stats>>,
}

impl CacheV2 {
    /// Create cache system with all three levels
    pub fn new(
        memory_size_mb: usize,
        disk_path: &Path,
        remote_endpoint: Option<String>,
    ) -> Result<Self> {
        Ok(Self {
            l1: MemoryCache::new(memory_size_mb),
            l2: DiskCache::new(disk_path)?,
            l3: RemoteCache::new(remote_endpoint.unwrap_or_default()),
            stats: Arc::new(parking_lot::Mutex::new(CacheV2Stats::default())),
        })
    }

    /// Insert with three-level write-through
    pub fn insert(&self, hash: ContentHash, result: CompileResult, data: &[u8]) -> Result<()> {
        let size = data.len();
        let entry = CacheEntry {
            hash,
            result: result.clone(),
            timestamp: std::time::SystemTime::now(),
            hits: 0,
            size_bytes: size,
        };

        // L1: Always
        self.l1.insert(hash, entry.clone());

        // L2: Always
        self.l2.store(&hash, data)?;

        // L3: If remote enabled
        if self.l3.is_enabled() {
            let _ = tokio::spawn({
                let l3 = RemoteCache::new(self.l3.endpoint.clone());
                let data = data.to_vec();
                async move {
                    let _ = l3.store(&hash, &data).await;
                }
            });
        }

        self.stats.lock().writes += 1;
        Ok(())
    }

    /// Get from cache (checks L1 → L2 → L3)
    pub fn get(&self, hash: &ContentHash) -> Option<CacheEntry> {
        // L1 hit
        if let Some(entry) = self.l1.get(hash) {
            self.stats.lock().l1_hits += 1;
            return Some(entry);
        }

        // L2 hit
        if self.l2.exists(hash) {
            self.stats.lock().l2_hits += 1;
            if let Ok(_data) = self.l2.retrieve(hash) {
                // Data found but we don't deserialize here, just mark as cached
                return None;  // For now, return None to force L2 lookup
            }
        }

        // L3 hit would go here with async retrieval
        self.stats.lock().misses += 1;
        None
    }

    /// Get current stats
    pub fn stats(&self) -> CacheV2Stats {
        self.stats.lock().clone()
    }

    /// Overall hit rate (L1 + L2 hits / total accesses)
    pub fn hit_rate(&self) -> f32 {
        let stats = self.stats.lock();
        let total = stats.l1_hits + stats.l2_hits + stats.misses;
        if total == 0 {
            0.0
        } else {
            (stats.l1_hits + stats.l2_hits) as f32 / total as f32
        }
    }

    /// Clear all cache levels
    pub fn clear_all(&self) -> Result<()> {
        self.l1.clear();
        self.l2.clear()?;
        Ok(())
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub total_hits: usize,
    pub hit_rate: f32,
    pub size_bytes: usize,
    pub max_size_bytes: usize,
}

/// Unified cache statistics across all levels
#[derive(Debug, Clone, Default)]
pub struct CacheV2Stats {
    pub l1_hits: usize,
    pub l2_hits: usize,
    pub misses: usize,
    pub writes: usize,
}

impl CacheV2Stats {
    pub fn total_accesses(&self) -> usize {
        self.l1_hits + self.l2_hits + self.misses
    }

    pub fn hit_rate(&self) -> f32 {
        let total = self.total_accesses();
        if total == 0 {
            0.0
        } else {
            (self.l1_hits + self.l2_hits) as f32 / total as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash() {
        let data = b"hello world";
        let hash = ContentHash::from_bytes(data);
        assert!(!hash.hex().is_empty());
        assert_eq!(hash.hex().len(), 64);
    }

    #[test]
    fn test_hash_from_hex() {
        let hex = "0".repeat(64);
        let hash = ContentHash::from_hex(&hex);
        assert!(hash.is_some());
    }

    #[test]
    fn test_memory_cache_insert_get() {
        let cache = MemoryCache::new(64);
        let hash = ContentHash::from_bytes(b"test");
        // Create minimal CompileResult
        let result = crate::core::CompileResult {
            success: true,
            language: crate::language::Language::Rust,
            target: crate::core::CompileTarget::native(),
            artifacts: vec![],
            errors: vec![],
            warnings: vec![],
            duration_ms: 100,
            output: String::new(),
            timestamp: chrono::Utc::now(),
        };
        let entry = CacheEntry {
            hash,
            result,
            timestamp: std::time::SystemTime::now(),
            hits: 1,
            size_bytes: 1024,
        };

        cache.insert(hash, entry.clone());
        let retrieved = cache.get(&hash);
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_memory_cache_stats() {
        let cache = MemoryCache::new(64);
        let hash = ContentHash::from_bytes(b"test");
        let result = crate::core::CompileResult {
            success: true,
            language: crate::language::Language::Rust,
            target: crate::core::CompileTarget::native(),
            artifacts: vec![],
            errors: vec![],
            warnings: vec![],
            duration_ms: 100,
            output: String::new(),
            timestamp: chrono::Utc::now(),
        };
        let entry = CacheEntry {
            hash,
            result,
            timestamp: std::time::SystemTime::now(),
            hits: 0,
            size_bytes: 1024,
        };

        cache.insert(hash, entry);
        let stats = cache.stats();
        assert_eq!(stats.entries, 1);
    }

    #[tokio::test]
    async fn test_cache_v2_threelevel() {
        let temp_dir = std::env::temp_dir().join("cache-test");
        let _ = std::fs::remove_dir_all(&temp_dir);

        let cache = CacheV2::new(64, &temp_dir, None).unwrap();
        let hash = ContentHash::from_bytes(b"test-data");
        let result = crate::core::CompileResult {
            success: true,
            language: crate::language::Language::Rust,
            target: crate::core::CompileTarget::native(),
            artifacts: vec![],
            errors: vec![],
            warnings: vec![],
            duration_ms: 100,
            output: String::new(),
            timestamp: chrono::Utc::now(),
        };

        cache.insert(hash, result, b"cached-output").unwrap();

        // Should hit L1
        assert!(cache.get(&hash).is_some());
        assert!(cache.hit_rate() > 0.0);
    }
}
