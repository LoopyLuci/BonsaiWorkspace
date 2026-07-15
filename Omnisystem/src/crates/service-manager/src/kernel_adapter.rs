//! Kernel interface adapter (mocked for Phase 2, real in Phase 1)
//!
//! In Phase 1, this will be replaced with real kernel syscalls:
//! - snapshot_vault(vault_id) -> blake3_hash
//! - restore_vault(snapshot_hash) -> vault_id

use crate::error::{Result, SLMError};
use log::debug;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Mock kernel interface for Phase 2
/// Real implementation will call UOSC kernel syscalls
pub struct KernelAdapter {
    /// Mock vault ID counter
    next_vault_id: Arc<RwLock<u64>>,

    /// Mock snapshot storage (vault_id -> snapshot_data)
    snapshots: Arc<RwLock<HashMap<u64, Vec<u8>>>>,

    /// Mock snapshot hashes
    snapshot_hashes: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl KernelAdapter {
    /// Create a new kernel adapter
    pub fn new() -> Self {
        Self {
            next_vault_id: Arc::new(RwLock::new(1)),
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            snapshot_hashes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new vault (mock: just allocate an ID)
    pub async fn create_vault(&self, _binary_hash: &str) -> Result<u64> {
        let mut counter = self.next_vault_id.write().await;
        let vault_id = *counter;
        *counter += 1;
        debug!("Created mock vault {}", vault_id);
        Ok(vault_id)
    }

    /// Take a snapshot of a vault (mock: store in memory)
    pub async fn snapshot_vault(&self, vault_id: u64) -> Result<String> {
        debug!("Snapshotting vault {}", vault_id);

        // Generate mock snapshot data
        let mock_data = format!("vault_{}_snapshot_data", vault_id).into_bytes();

        // Store snapshot
        let hash_obj = blake3::hash(&mock_data);
        let hash_str = hash_obj.to_hex().to_string();

        let mut snapshots = self.snapshots.write().await;
        snapshots.insert(vault_id, mock_data.clone());

        let mut hashes = self.snapshot_hashes.write().await;
        hashes.insert(hash_str.clone(), mock_data);

        debug!("Snapshot of vault {} -> hash {}", vault_id, hash_str);
        Ok(hash_str)
    }

    /// Restore a vault from a snapshot (mock: create new vault with same ID pattern)
    pub async fn restore_vault(&self, snapshot_hash: &str) -> Result<u64> {
        debug!("Restoring vault from snapshot {}", snapshot_hash);

        // Look up snapshot by hash
        let hashes = self.snapshot_hashes.read().await;
        if !hashes.contains_key(snapshot_hash) {
            return Err(SLMError::RestoreFailed(format!(
                "Snapshot not found: {}",
                snapshot_hash
            )));
        }

        // Create new vault
        let mut counter = self.next_vault_id.write().await;
        let new_vault_id = *counter;
        *counter += 1;

        debug!("Restored vault as new vault {}", new_vault_id);
        Ok(new_vault_id)
    }

    /// Destroy a vault
    pub async fn destroy_vault(&self, vault_id: u64) -> Result<()> {
        debug!("Destroying vault {}", vault_id);
        let mut snapshots = self.snapshots.write().await;
        snapshots.remove(&vault_id);
        Ok(())
    }

    /// Get snapshot data (for testing)
    pub async fn get_snapshot_data(&self, snapshot_hash: &str) -> Result<Vec<u8>> {
        let hashes = self.snapshot_hashes.read().await;
        hashes
            .get(snapshot_hash)
            .cloned()
            .ok_or_else(|| {
                SLMError::SnapshotFailed(format!("Snapshot not found: {}", snapshot_hash))
            })
    }

    /// Verify snapshot integrity (mock: always succeeds)
    pub async fn verify_snapshot(&self, snapshot_hash: &str) -> Result<bool> {
        let hashes = self.snapshot_hashes.read().await;
        Ok(hashes.contains_key(snapshot_hash))
    }
}

impl Default for KernelAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_vault() {
        let adapter = KernelAdapter::new();
        let vault1 = adapter.create_vault("binary_hash_1").await.unwrap();
        let vault2 = adapter.create_vault("binary_hash_2").await.unwrap();
        assert_ne!(vault1, vault2);
    }

    #[tokio::test]
    async fn test_snapshot_and_restore() {
        let adapter = KernelAdapter::new();
        let vault = adapter.create_vault("binary_hash").await.unwrap();

        // Snapshot
        let hash = adapter.snapshot_vault(vault).await.unwrap();
        assert!(!hash.is_empty());

        // Verify snapshot exists
        assert!(adapter.verify_snapshot(&hash).await.unwrap());

        // Get snapshot data
        let data = adapter.get_snapshot_data(&hash).await.unwrap();
        assert!(!data.is_empty());

        // Restore
        let restored_vault = adapter.restore_vault(&hash).await.unwrap();
        assert_ne!(vault, restored_vault);
    }

    #[tokio::test]
    async fn test_restore_nonexistent_snapshot() {
        let adapter = KernelAdapter::new();
        let result = adapter.restore_vault("nonexistent_hash").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_destroy_vault() {
        let adapter = KernelAdapter::new();
        let vault = adapter.create_vault("binary_hash").await.unwrap();
        assert!(adapter.destroy_vault(vault).await.is_ok());
    }
}
