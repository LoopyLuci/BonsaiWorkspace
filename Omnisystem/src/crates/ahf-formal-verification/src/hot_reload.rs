//! Hot-Reload Mechanism for Zero-Downtime Updates
//!
//! This module enables updating knowledge modules and policies without service interruption.

use crate::error::{VerificationError, VerificationResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use std::sync::RwLock;

/// A policy update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyUpdate {
    /// Unique identifier
    pub id: Uuid,
    /// Version of this update
    pub version: u64,
    /// Timestamp of update
    pub timestamp: DateTime<Utc>,
    /// Update details
    pub details: serde_json::Value,
    /// Authority that approved this update
    pub approved_by: String,
    /// Hash of update content
    pub content_hash: String,
    /// Can rollback to previous version?
    pub is_rollback_safe: bool,
}

impl PolicyUpdate {
    /// Create a new policy update
    pub fn new(
        version: u64,
        details: serde_json::Value,
        approved_by: impl Into<String>,
    ) -> Self {
        let id = Uuid::new_v4();
        let timestamp = Utc::now();
        let content_hash = blake3::hash(serde_json::to_string(&details).unwrap().as_bytes())
            .to_hex()
            .to_string();

        PolicyUpdate {
            id,
            version,
            timestamp,
            details,
            approved_by: approved_by.into(),
            content_hash,
            is_rollback_safe: true,
        }
    }

    /// Verify the update integrity
    pub fn verify(&self) -> VerificationResult<()> {
        let computed_hash = blake3::hash(serde_json::to_string(&self.details).unwrap().as_bytes())
            .to_hex()
            .to_string();

        if computed_hash != self.content_hash {
            return Err(VerificationError::hash_mismatch(
                self.content_hash.clone(),
                computed_hash,
            ));
        }

        Ok(())
    }
}

/// Hot-reload version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReloadVersion {
    /// Version number
    pub version: u64,
    /// Timestamp when this version was activated
    pub activated_at: DateTime<Utc>,
    /// Active policies in this version
    pub policies: HashMap<String, PolicyUpdate>,
    /// Rollback version (if any)
    pub previous_version: Option<u64>,
}

impl ReloadVersion {
    /// Create a new reload version
    pub fn new(version: u64, previous_version: Option<u64>) -> Self {
        ReloadVersion {
            version,
            activated_at: Utc::now(),
            policies: HashMap::new(),
            previous_version,
        }
    }

    /// Add a policy to this version
    pub fn add_policy(&mut self, name: String, policy: PolicyUpdate) {
        self.policies.insert(name, policy);
    }

    /// Get a policy
    pub fn get_policy(&self, name: &str) -> Option<&PolicyUpdate> {
        self.policies.get(name)
    }
}

/// Hot-reload manager for zero-downtime updates
pub struct HotReloadManager {
    /// Current version
    current_version: Arc<RwLock<u64>>,
    /// Version history
    versions: Arc<RwLock<HashMap<u64, ReloadVersion>>>,
    /// Active policies
    policies: Arc<RwLock<HashMap<String, PolicyUpdate>>>,
    /// Update queue (for batching)
    update_queue: Arc<RwLock<Vec<PolicyUpdate>>>,
    /// Maximum rollback versions to keep
    max_history: usize,
}

impl HotReloadManager {
    /// Create a new hot-reload manager
    pub fn new(max_history: usize) -> Self {
        let mut versions = HashMap::new();
        let initial_version = ReloadVersion::new(0, None);
        versions.insert(0, initial_version);

        HotReloadManager {
            current_version: Arc::new(RwLock::new(0)),
            versions: Arc::new(RwLock::new(versions)),
            policies: Arc::new(RwLock::new(HashMap::new())),
            update_queue: Arc::new(RwLock::new(Vec::new())),
            max_history,
        }
    }

    /// Queue a policy update
    pub fn queue_update(&self, update: PolicyUpdate) -> VerificationResult<()> {
        update.verify()?;
        self.update_queue.write().unwrap().push(update);
        Ok(())
    }

    /// Apply all queued updates
    pub fn apply_updates(&self) -> VerificationResult<()> {
        let queue = self.update_queue.write().unwrap().drain(..).collect::<Vec<_>>();
        let queue_len = queue.len();

        if queue.is_empty() {
            return Err(VerificationError::HotReloadError(
                "No updates in queue".to_string(),
            ));
        }

        // Create new version
        let current = *self.current_version.read().unwrap();
        let new_version = current + 1;
        let mut version_entry = ReloadVersion::new(new_version, Some(current));

        // Add updates to new version
        let mut policies = self.policies.write().unwrap();
        for update in queue {
            let name = format!("policy_{}", update.id);
            version_entry.add_policy(name.clone(), update.clone());
            policies.insert(name, update);
        }

        // Store new version
        let mut versions = self.versions.write().unwrap();
        versions.insert(new_version, version_entry);

        // Clean up old versions
        if versions.len() > self.max_history {
            let mut old_versions: Vec<_> = versions.keys().cloned().collect();
            old_versions.sort();
            for old_v in old_versions.iter().take(versions.len() - self.max_history) {
                versions.remove(old_v);
            }
        }

        // Update current version
        *self.current_version.write().unwrap() = new_version;

        tracing::info!("Applied {} updates, now at version {}", queue_len, new_version);
        Ok(())
    }

    /// Rollback to previous version
    pub fn rollback(&self) -> VerificationResult<()> {
        let versions = self.versions.read().unwrap();
        let current = *self.current_version.read().unwrap();

        let current_version = versions
            .get(&current)
            .ok_or_else(|| VerificationError::HotReloadError("Current version not found".to_string()))?;

        let previous_version = current_version
            .previous_version
            .ok_or_else(|| VerificationError::HotReloadError("No previous version to rollback to".to_string()))?;

        let previous = versions.get(&previous_version).ok_or_else(|| {
            VerificationError::HotReloadError("Previous version not found".to_string())
        })?;

        if !previous.policies.values().all(|p| p.is_rollback_safe) {
            return Err(VerificationError::HotReloadError(
                "Previous version contains non-rollback-safe policies".to_string(),
            ));
        }

        // Restore policies from previous version
        let mut policies = self.policies.write().unwrap();
        policies.clear();
        for (name, policy) in &previous.policies {
            policies.insert(name.clone(), policy.clone());
        }

        // Update current version
        drop(versions);
        *self.current_version.write().unwrap() = previous_version;

        tracing::info!("Rolled back to version {}", previous_version);
        Ok(())
    }

    /// Get current version number
    pub fn current_version(&self) -> u64 {
        *self.current_version.read().unwrap()
    }

    /// Get version history
    pub fn version_history(&self) -> Vec<ReloadVersion> {
        let versions = self.versions.read().unwrap();
        let mut history: Vec<_> = versions.values().cloned().collect();
        history.sort_by_key(|v| std::cmp::Reverse(v.version));
        history
    }

    /// Get active policy
    pub fn get_policy(&self, name: &str) -> Option<PolicyUpdate> {
        self.policies.read().ok()?.get(name).cloned()
    }

    /// Get all active policies
    pub fn get_all_policies(&self) -> HashMap<String, PolicyUpdate> {
        self.policies.read().map(|p| p.clone()).unwrap_or_default()
    }

    /// Check if version exists
    pub fn version_exists(&self, version: u64) -> bool {
        self.versions.read().map(|v| v.contains_key(&version)).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_update_creation() {
        let details = serde_json::json!({
            "threshold": 0.95,
            "enabled": true
        });
        let update = PolicyUpdate::new(1, details, "council");
        assert!(update.verify().is_ok());
    }

    #[test]
    fn test_policy_update_hash_verification() {
        let details = serde_json::json!({"test": "value"});
        let mut update = PolicyUpdate::new(1, details, "council");

        // Tamper with content hash
        update.content_hash = "wrong_hash".to_string();
        assert!(update.verify().is_err());
    }

    #[test]
    fn test_hot_reload_manager_creation() {
        let manager = HotReloadManager::new(10);
        assert_eq!(manager.current_version(), 0);
    }

    #[test]
    fn test_queue_and_apply_updates() {
        let manager = HotReloadManager::new(10);

        let details = serde_json::json!({"threshold": 0.95});
        let update = PolicyUpdate::new(1, details, "council");

        assert!(manager.queue_update(update.clone()).is_ok());
        assert!(manager.apply_updates().is_ok());

        assert_eq!(manager.current_version(), 1);
    }

    #[test]
    fn test_rollback() {
        let manager = HotReloadManager::new(10);

        let details1 = serde_json::json!({"version": 1});
        let update1 = PolicyUpdate::new(1, details1, "council");

        manager.queue_update(update1).unwrap();
        manager.apply_updates().unwrap();

        assert_eq!(manager.current_version(), 1);

        let details2 = serde_json::json!({"version": 2});
        let update2 = PolicyUpdate::new(2, details2, "council");

        manager.queue_update(update2).unwrap();
        manager.apply_updates().unwrap();

        assert_eq!(manager.current_version(), 2);

        assert!(manager.rollback().is_ok());
        assert_eq!(manager.current_version(), 1);
    }

    #[test]
    fn test_version_history() {
        let manager = HotReloadManager::new(10);

        let details = serde_json::json!({"test": "value"});
        let update = PolicyUpdate::new(1, details, "council");

        manager.queue_update(update).unwrap();
        manager.apply_updates().unwrap();

        let history = manager.version_history();
        assert!(history.len() >= 2); // Initial version + new version
    }
}
