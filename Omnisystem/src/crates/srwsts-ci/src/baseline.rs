//! Baseline management: load, verify, and compare baselines from CAS

use crate::errors::{CIError, CIResult};
use crate::metrics::MetricsSnapshot;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tracing::{debug, info, warn};

/// Baseline version identifier
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct BaselineVersion {
    pub version: String,
    pub commit_hash: String,
    pub timestamp: DateTime<Utc>,
    pub approved_by: Option<String>,
}

/// Integrity verification for baseline authenticity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineIntegrity {
    pub content_hash: String,
    pub metadata_hash: String,
    pub computed_at: DateTime<Utc>,
    pub verified: bool,
}

/// Complete baseline record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    pub version: BaselineVersion,
    pub metrics: MetricsSnapshot,
    pub test_results: HashMap<String, TestResult>,
    pub integrity: BaselineIntegrity,
    pub created_at: DateTime<Utc>,
}

/// Result of a single test in baseline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_name: String,
    pub passed: bool,
    pub duration_ms: u64,
    pub error_message: Option<String>,
    pub determinism_runs: Vec<u64>,
}

/// Trait for CAS operations (can be mocked in tests)
#[async_trait]
pub trait CasBackend: Send + Sync {
    async fn load_baseline(&self, hash: &str) -> CIResult<Vec<u8>>;
    async fn verify_approved(&self, hash: &str) -> CIResult<bool>;
}

/// Mock CAS implementation for testing
pub struct MockCas {
    baselines: std::sync::Mutex<HashMap<String, Vec<u8>>>,
    approved: std::sync::Mutex<std::collections::HashSet<String>>,
}

impl MockCas {
    pub fn new() -> Self {
        Self {
            baselines: std::sync::Mutex::new(HashMap::new()),
            approved: std::sync::Mutex::new(std::collections::HashSet::new()),
        }
    }

    pub fn insert_baseline(&self, hash: String, data: Vec<u8>) {
        self.baselines.lock().unwrap().insert(hash, data);
    }

    pub fn approve(&self, hash: String) {
        self.approved.lock().unwrap().insert(hash);
    }
}

#[async_trait]
impl CasBackend for MockCas {
    async fn load_baseline(&self, hash: &str) -> CIResult<Vec<u8>> {
        self.baselines
            .lock()
            .unwrap()
            .get(hash)
            .cloned()
            .ok_or_else(|| CIError::BaselineNotFound(hash.to_string()))
    }

    async fn verify_approved(&self, hash: &str) -> CIResult<bool> {
        Ok(self.approved.lock().unwrap().contains(hash))
    }
}

impl Default for MockCas {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages baseline loading, verification, and comparison
pub struct BaselineManager {
    cas: Arc<dyn CasBackend>,
    cache_dir: PathBuf,
    loaded_baselines: dashmap::DashMap<String, Baseline>,
}

impl BaselineManager {
    /// Create new baseline manager with CAS backend
    pub async fn new(cas: Arc<dyn CasBackend>, cache_dir: impl AsRef<Path>) -> CIResult<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        fs::create_dir_all(&cache_dir).await?;

        info!("BaselineManager initialized with cache at {:?}", cache_dir);

        Ok(Self {
            cas,
            cache_dir,
            loaded_baselines: dashmap::DashMap::new(),
        })
    }

    /// Load baseline from CAS with verification
    pub async fn load_baseline(&self, content_hash: &str) -> CIResult<Baseline> {
        // Check cache first
        if let Some(baseline) = self.loaded_baselines.get(content_hash) {
            debug!("Baseline {} loaded from cache", content_hash);
            return Ok(baseline.clone());
        }

        // Verify approval in CAS
        if !self.cas.verify_approved(content_hash).await? {
            warn!("Baseline {} not council-approved", content_hash);
            return Err(CIError::ApprovalRequired(
                format!("Baseline {} requires council approval", content_hash),
            ));
        }

        // Load from CAS
        let data = self.cas.load_baseline(content_hash).await?;

        // Parse and verify integrity
        let baseline: Baseline = serde_json::from_slice(&data)
            .map_err(|e| CIError::SerializationError(e))?;

        let computed_hash = self.compute_content_hash(&data)?;
        if computed_hash != baseline.integrity.content_hash {
            return Err(CIError::IntegrityCheckFailed {
                expected: baseline.integrity.content_hash.clone(),
                actual: computed_hash,
            });
        }

        debug!(
            "Baseline {} integrity verified, metrics count: {}",
            content_hash,
            baseline.metrics.latencies.len()
        );

        // Cache it
        self.loaded_baselines.insert(content_hash.to_string(), baseline.clone());

        info!("Baseline {} loaded and verified from CAS", content_hash);
        Ok(baseline)
    }

    /// Load baseline with version specification
    pub async fn load_baseline_version(
        &self,
        version: &BaselineVersion,
    ) -> CIResult<Baseline> {
        self.load_baseline(&version.commit_hash).await
    }

    /// Compute content hash for integrity verification
    pub fn compute_content_hash(&self, data: &[u8]) -> CIResult<String> {
        let hash = blake3::hash(data);
        Ok(hash.to_hex().to_string())
    }

    /// Verify baseline integrity
    pub fn verify_integrity(&self, baseline: &Baseline) -> CIResult<BaselineIntegrity> {
        let data = serde_json::to_vec(&baseline.metrics)
            .map_err(|e| CIError::SerializationError(e))?;
        let content_hash = self.compute_content_hash(&data)?;

        if content_hash != baseline.integrity.content_hash {
            return Err(CIError::IntegrityCheckFailed {
                expected: baseline.integrity.content_hash.clone(),
                actual: content_hash,
            });
        }

        Ok(BaselineIntegrity {
            content_hash,
            metadata_hash: baseline.integrity.metadata_hash.clone(),
            computed_at: Utc::now(),
            verified: true,
        })
    }

    /// Store baseline locally (for testing and fallback)
    pub async fn store_local(&self, baseline: &Baseline) -> CIResult<String> {
        let hash = blake3::hash(
            serde_json::to_vec(baseline)
                .map_err(|e| CIError::SerializationError(e))?
                .as_slice(),
        );
        let hash_str = hash.to_hex().to_string();

        let path = self.cache_dir.join(&hash_str);
        let data = serde_json::to_string(baseline)
            .map_err(|e| CIError::SerializationError(e))?;
        fs::write(&path, &data).await?;

        info!("Baseline stored locally at {:?}", path);
        Ok(hash_str)
    }

    /// Get cached baseline
    pub fn get_cached(&self, hash: &str) -> Option<Baseline> {
        self.loaded_baselines.get(hash).map(|r| r.clone())
    }

    /// Clear cache (for testing)
    pub fn clear_cache(&self) {
        self.loaded_baselines.clear();
    }

    /// List all cached baselines
    pub fn list_cached(&self) -> Vec<String> {
        self.loaded_baselines
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{PerformanceMetrics, MetricsSnapshot};

    fn create_test_baseline() -> Baseline {
        Baseline {
            version: BaselineVersion {
                version: "1.0.0".to_string(),
                commit_hash: "abc123".to_string(),
                timestamp: Utc::now(),
                approved_by: Some("test_admin".to_string()),
            },
            metrics: MetricsSnapshot {
                latencies: vec![10.0, 15.0, 20.0],
                throughputs: vec![100.0, 150.0],
                memory_peak: 512.0,
                cpu_average: 45.0,
                io_operations: 42,
                timestamp: Utc::now(),
            },
            test_results: {
                let mut map = HashMap::new();
                map.insert(
                    "test_basic".to_string(),
                    TestResult {
                        test_name: "test_basic".to_string(),
                        passed: true,
                        duration_ms: 100,
                        error_message: None,
                        determinism_runs: vec![100, 101, 100],
                    },
                );
                map
            },
            integrity: BaselineIntegrity {
                content_hash: "def456".to_string(),
                metadata_hash: "ghi789".to_string(),
                computed_at: Utc::now(),
                verified: true,
            },
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_baseline_loading() {
        let cas = Arc::new(MockCas::new());
        let baseline = create_test_baseline();

        let data = serde_json::to_vec(&baseline).unwrap();
        let hash = blake3::hash(&data).to_hex().to_string();

        cas.insert_baseline(hash.clone(), data);
        cas.approve(hash.clone());

        let manager = BaselineManager::new(cas, "/tmp/baselines").await.unwrap();
        let loaded = manager.load_baseline(&hash).await.unwrap();

        assert_eq!(loaded.version.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_baseline_not_approved() {
        let cas = Arc::new(MockCas::new());
        let baseline = create_test_baseline();

        let data = serde_json::to_vec(&baseline).unwrap();
        let hash = blake3::hash(&data).to_hex().to_string();

        cas.insert_baseline(hash.clone(), data);
        // Don't approve it

        let manager = BaselineManager::new(cas, "/tmp/baselines").await.unwrap();
        let result = manager.load_baseline(&hash).await;

        assert!(matches!(result, Err(CIError::ApprovalRequired(_))));
    }

    #[tokio::test]
    async fn test_baseline_integrity_check() {
        let cas = Arc::new(MockCas::new());
        let mut baseline = create_test_baseline();

        let data = serde_json::to_vec(&baseline).unwrap();
        let hash = blake3::hash(&data).to_hex().to_string();

        // Set correct hash
        baseline.integrity.content_hash = hash.clone();
        let correct_data = serde_json::to_vec(&baseline).unwrap();

        cas.insert_baseline(hash.clone(), correct_data);
        cas.approve(hash.clone());

        let manager = BaselineManager::new(cas, "/tmp/baselines").await.unwrap();
        let loaded = manager.load_baseline(&hash).await.unwrap();
        let verified = manager.verify_integrity(&loaded).unwrap();

        assert!(verified.verified);
    }

    #[tokio::test]
    async fn test_baseline_caching() {
        let cas = Arc::new(MockCas::new());
        let baseline = create_test_baseline();

        let data = serde_json::to_vec(&baseline).unwrap();
        let hash = blake3::hash(&data).to_hex().to_string();

        cas.insert_baseline(hash.clone(), data);
        cas.approve(hash.clone());

        let manager = BaselineManager::new(cas, "/tmp/baselines").await.unwrap();

        // First load
        let _ = manager.load_baseline(&hash).await.unwrap();

        // Check cache
        assert!(manager.get_cached(&hash).is_some());

        // List cached
        let cached_list = manager.list_cached();
        assert_eq!(cached_list.len(), 1);
        assert!(cached_list.contains(&hash));
    }

    #[tokio::test]
    async fn test_baseline_version_loading() {
        let cas = Arc::new(MockCas::new());
        let baseline = create_test_baseline();

        let data = serde_json::to_vec(&baseline).unwrap();
        let hash = blake3::hash(&data).to_hex().to_string();

        cas.insert_baseline(hash.clone(), data);
        cas.approve(hash.clone());

        let manager = BaselineManager::new(cas, "/tmp/baselines").await.unwrap();

        let version = BaselineVersion {
            version: "1.0.0".to_string(),
            commit_hash: hash.clone(),
            timestamp: Utc::now(),
            approved_by: Some("test".to_string()),
        };

        let loaded = manager.load_baseline_version(&version).await.unwrap();
        assert_eq!(loaded.version.version, "1.0.0");
    }

    #[test]
    fn test_compute_content_hash() {
        let cas = std::sync::Arc::new(MockCas::new());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let manager = rt
            .block_on(BaselineManager::new(cas, "/tmp/baselines"))
            .unwrap();

        let data = b"test data";
        let hash = manager.compute_content_hash(data).unwrap();

        // Same data should produce same hash
        let hash2 = manager.compute_content_hash(data).unwrap();
        assert_eq!(hash, hash2);

        // Different data should produce different hash
        let hash3 = manager.compute_content_hash(b"different data").unwrap();
        assert_ne!(hash, hash3);
    }

    #[tokio::test]
    async fn test_local_baseline_storage() {
        let cas = Arc::new(MockCas::new());
        let baseline = create_test_baseline();
        let manager = BaselineManager::new(cas, "/tmp/baselines_test").await.unwrap();

        let hash = manager.store_local(&baseline).await.unwrap();
        assert!(!hash.is_empty());

        // Verify file exists
        let path = manager.cache_dir.join(&hash);
        assert!(path.exists());
    }

    #[tokio::test]
    async fn test_baseline_clear_cache() {
        let cas = Arc::new(MockCas::new());
        let baseline = create_test_baseline();

        let data = serde_json::to_vec(&baseline).unwrap();
        let hash = blake3::hash(&data).to_hex().to_string();

        cas.insert_baseline(hash.clone(), data);
        cas.approve(hash.clone());

        let manager = BaselineManager::new(cas, "/tmp/baselines").await.unwrap();
        let _ = manager.load_baseline(&hash).await.unwrap();

        assert!(manager.get_cached(&hash).is_some());
        manager.clear_cache();
        assert!(manager.get_cached(&hash).is_none());
    }

    #[test]
    fn test_baseline_version_equality() {
        let v1 = BaselineVersion {
            version: "1.0.0".to_string(),
            commit_hash: "abc".to_string(),
            timestamp: Utc::now(),
            approved_by: None,
        };

        let v2 = BaselineVersion {
            version: "1.0.0".to_string(),
            commit_hash: "abc".to_string(),
            timestamp: Utc::now(),
            approved_by: None,
        };

        assert_eq!(v1, v2);
    }

    #[test]
    fn test_test_result_serialization() {
        let result = TestResult {
            test_name: "test".to_string(),
            passed: true,
            duration_ms: 100,
            error_message: None,
            determinism_runs: vec![100, 101, 100],
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: TestResult = serde_json::from_str(&json).unwrap();

        assert_eq!(result.test_name, deserialized.test_name);
        assert_eq!(result.duration_ms, deserialized.duration_ms);
    }
}
