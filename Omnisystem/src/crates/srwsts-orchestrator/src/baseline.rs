//! Baseline management with CAS-backed storage.

use crate::error::{OrchestratorError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use dashmap::DashMap;
use sha2::{Digest, Sha256};
use tracing::{info, debug};

/// CAS hash (SHA256).
pub type CasHash = String;

/// Baseline version identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BaselineVersion(pub u64);

impl BaselineVersion {
    pub fn next(&self) -> Self {
        BaselineVersion(self.0 + 1)
    }
}

impl std::fmt::Display for BaselineVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Baseline metric entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricEntry {
    /// Metric name.
    pub name: String,
    /// Value.
    pub value: f64,
    /// Unit (e.g., "ms", "MB", "ops/sec").
    pub unit: String,
    /// Standard deviation if available.
    pub std_dev: Option<f64>,
}

/// Golden baseline snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    /// Baseline name.
    pub name: String,
    /// Version of this baseline.
    pub version: BaselineVersion,
    /// CAS hash of baseline data.
    pub cas_hash: CasHash,
    /// When this baseline was created.
    pub created_at: DateTime<Utc>,
    /// When this baseline was last updated.
    pub updated_at: DateTime<Utc>,
    /// Metrics in this baseline.
    pub metrics: Vec<MetricEntry>,
    /// Metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Baseline {
    /// Create a new baseline.
    pub fn new(name: String, metrics: Vec<MetricEntry>) -> Self {
        let now = Utc::now();
        let cas_hash = Self::compute_hash(&metrics);

        Self {
            name,
            version: BaselineVersion(1),
            cas_hash,
            created_at: now,
            updated_at: now,
            metrics,
            metadata: HashMap::new(),
        }
    }

    /// Compute CAS hash for metrics.
    pub fn compute_hash(metrics: &[MetricEntry]) -> CasHash {
        let serialized = serde_json::to_string(metrics).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(serialized.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Get a metric by name.
    pub fn get_metric(&self, name: &str) -> Option<&MetricEntry> {
        self.metrics.iter().find(|m| m.name == name)
    }

    /// Update metrics and version.
    pub fn update(&mut self, metrics: Vec<MetricEntry>) {
        self.metrics = metrics;
        self.version = self.version.next();
        self.cas_hash = Self::compute_hash(&self.metrics);
        self.updated_at = Utc::now();
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Baseline manager with CAS integration.
pub struct BaselineManager {
    baselines: Arc<DashMap<String, Baseline>>,
    cas_storage: Arc<DashMap<CasHash, Vec<u8>>>, // Simulated CAS
}

impl BaselineManager {
    /// Create a new baseline manager.
    pub fn new() -> Self {
        Self {
            baselines: Arc::new(DashMap::new()),
            cas_storage: Arc::new(DashMap::new()),
        }
    }

    /// Register a new baseline.
    pub async fn register_baseline(&self, baseline: Baseline) -> Result<CasHash> {
        let name = baseline.name.clone();
        let cas_hash = baseline.cas_hash.clone();

        // Store in CAS (simulated)
        let serialized = serde_json::to_vec(&baseline)
            .map_err(|e| OrchestratorError::SerializationError(e))?;
        self.cas_storage.insert(cas_hash.clone(), serialized);

        // Store baseline metadata
        self.baselines.insert(name.clone(), baseline);
        info!("registered baseline: {} (hash: {})", name, cas_hash);

        Ok(cas_hash)
    }

    /// Get a baseline by name.
    pub fn get_baseline(&self, name: &str) -> Result<Baseline> {
        self.baselines
            .get(name)
            .map(|r| r.value().clone())
            .ok_or_else(|| OrchestratorError::BaselineNotFound(name.to_string()))
    }

    /// Get a baseline by CAS hash.
    pub fn get_baseline_by_hash(&self, hash: &str) -> Result<Baseline> {
        let data = self
            .cas_storage
            .get(hash)
            .ok_or_else(|| OrchestratorError::CasError("baseline not found".to_string()))?;

        serde_json::from_slice(&data)
            .map_err(|e| OrchestratorError::SerializationError(e))
    }

    /// Update an existing baseline.
    pub async fn update_baseline(&self, name: &str, metrics: Vec<MetricEntry>) -> Result<()> {
        let mut entry = self
            .baselines
            .get_mut(name)
            .ok_or_else(|| OrchestratorError::BaselineNotFound(name.to_string()))?;

        let old_hash = entry.cas_hash.clone();
        entry.update(metrics);
        let new_hash = entry.cas_hash.clone();

        // Update CAS
        let serialized = serde_json::to_vec(&*entry)
            .map_err(|e| OrchestratorError::SerializationError(e))?;
        self.cas_storage.insert(new_hash, serialized);

        debug!("updated baseline: {} ({} -> {})", name, old_hash, entry.cas_hash);

        Ok(())
    }

    /// List all baseline names.
    pub fn list_baselines(&self) -> Vec<String> {
        self.baselines
            .iter()
            .map(|r| r.key().clone())
            .collect()
    }

    /// Check if a baseline exists.
    pub fn baseline_exists(&self, name: &str) -> bool {
        self.baselines.contains_key(name)
    }

    /// Get all baselines.
    pub fn get_all_baselines(&self) -> Vec<Baseline> {
        self.baselines
            .iter()
            .map(|r| r.value().clone())
            .collect()
    }

    /// Delete a baseline.
    pub fn delete_baseline(&self, name: &str) -> Result<Baseline> {
        let (_, baseline) = self
            .baselines
            .remove(name)
            .ok_or_else(|| OrchestratorError::BaselineNotFound(name.to_string()))?;

        // Also remove from CAS
        self.cas_storage.remove(&baseline.cas_hash);

        info!("deleted baseline: {}", name);
        Ok(baseline)
    }

    /// Get baseline history (all versions).
    pub fn get_baseline_history(&self, name: &str) -> Result<Vec<Baseline>> {
        let baseline = self.get_baseline(name)?;
        // In a real implementation, this would fetch version history
        // For now, return current version
        Ok(vec![baseline])
    }

    /// Verify baseline integrity via CAS.
    pub fn verify_baseline(&self, name: &str) -> Result<bool> {
        let baseline = self.get_baseline(name)?;
        let computed_hash = Baseline::compute_hash(&baseline.metrics);
        Ok(computed_hash == baseline.cas_hash)
    }

    /// Get CAS statistics.
    pub fn cas_statistics(&self) -> CasStatistics {
        CasStatistics {
            total_baselines: self.baselines.len(),
            cas_entries: self.cas_storage.len(),
        }
    }
}

impl Default for BaselineManager {
    fn default() -> Self {
        Self::new()
    }
}

/// CAS statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasStatistics {
    pub total_baselines: usize,
    pub cas_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_baseline_creation() {
        let metrics = vec![MetricEntry {
            name: "latency".to_string(),
            value: 100.0,
            unit: "ms".to_string(),
            std_dev: Some(5.0),
        }];

        let baseline = Baseline::new("test_baseline".to_string(), metrics);
        assert_eq!(baseline.name, "test_baseline");
        assert_eq!(baseline.version, BaselineVersion(1));
        assert!(!baseline.cas_hash.is_empty());
    }

    #[tokio::test]
    async fn test_baseline_manager() {
        let manager = BaselineManager::new();
        let metrics = vec![MetricEntry {
            name: "throughput".to_string(),
            value: 1000.0,
            unit: "ops/sec".to_string(),
            std_dev: None,
        }];

        let baseline = Baseline::new("perf".to_string(), metrics);
        let hash = manager.register_baseline(baseline).await.unwrap();

        let retrieved = manager.get_baseline("perf").unwrap();
        assert_eq!(retrieved.name, "perf");
        assert_eq!(retrieved.cas_hash, hash);
    }

    #[tokio::test]
    async fn test_baseline_update() {
        let manager = BaselineManager::new();
        let metrics = vec![MetricEntry {
            name: "latency".to_string(),
            value: 100.0,
            unit: "ms".to_string(),
            std_dev: None,
        }];

        let baseline = Baseline::new("perf".to_string(), metrics);
        manager.register_baseline(baseline).await.unwrap();

        let new_metrics = vec![MetricEntry {
            name: "latency".to_string(),
            value: 95.0,
            unit: "ms".to_string(),
            std_dev: None,
        }];

        manager
            .update_baseline("perf", new_metrics)
            .await
            .unwrap();

        let updated = manager.get_baseline("perf").unwrap();
        assert_eq!(updated.version, BaselineVersion(2));
    }

    #[test]
    fn test_baseline_verification() {
        let metrics = vec![MetricEntry {
            name: "latency".to_string(),
            value: 100.0,
            unit: "ms".to_string(),
            std_dev: None,
        }];

        let baseline = Baseline::new("perf".to_string(), metrics);
        assert!(baseline.cas_hash.len() > 0);
    }
}
