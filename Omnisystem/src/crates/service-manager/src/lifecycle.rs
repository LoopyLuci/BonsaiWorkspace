//! Service lifecycle state machine

use crate::error::{Result, SLMError};
use crate::kernel_adapter::KernelAdapter;
use crate::types::*;
use log::{debug, info, warn};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Manages service lifecycle transitions
pub struct LifecycleManager {
    kernel: KernelAdapter,
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub fn new(kernel: KernelAdapter) -> Self {
        Self { kernel }
    }

    /// Spawn a new service instance
    pub async fn spawn_service(&self, manifest: ServiceManifest) -> Result<ServiceInstance> {
        info!("Spawning service: {}", manifest.name);

        // Create vault
        let vault_id = self
            .kernel
            .create_vault(&manifest.binary_hash)
            .await
            .map_err(|e| SLMError::VaultCreationFailed(e.to_string()))?;

        let instance = ServiceInstance {
            instance_id: Uuid::new_v4(),
            manifest,
            state: ServiceState::Running,
            vault_id: Some(vault_id),
            latest_snapshot: None,
            snapshots: Vec::new(),
            last_access_timestamp: current_timestamp(),
            consecutive_failures: 0,
            resource_usage: ResourceUsage::default(),
        };

        info!(
            "Service spawned: {} (vault: {}, instance: {})",
            instance.manifest.name, vault_id, instance.instance_id
        );

        Ok(instance)
    }

    /// Pause and snapshot a service
    pub async fn pause_and_snapshot(&self, instance: &mut ServiceInstance) -> Result<()> {
        if !instance.state.is_running() {
            return Err(SLMError::ServiceNotRunning(
                instance.manifest.name.clone(),
            ));
        }

        debug!("Pausing service: {}", instance.manifest.name);

        let vault_id = instance
            .vault_id
            .ok_or_else(|| SLMError::ServiceNotRunning(instance.manifest.name.clone()))?;

        // Transition to PAUSING
        instance.state = ServiceState::Pausing;

        // Take snapshot
        let snapshot_hash = self
            .kernel
            .snapshot_vault(vault_id)
            .await
            .map_err(|e| SLMError::SnapshotFailed(e.to_string()))?;

        let snapshot_data = self
            .kernel
            .get_snapshot_data(&snapshot_hash)
            .await
            .map_err(|e| SLMError::SnapshotFailed(e.to_string()))?;

        let snapshot = Snapshot {
            hash: snapshot_hash.clone(),
            size_bytes: snapshot_data.len() as u64,
            created_at: chrono::Utc::now(),
            archived: false,
        };

        // Update instance
        instance.latest_snapshot = Some(snapshot.clone());
        instance.snapshots.push(snapshot);
        instance.state = ServiceState::Paused;

        // Destroy vault
        self.kernel
            .destroy_vault(vault_id)
            .await
            .map_err(|e| SLMError::SnapshotFailed(e.to_string()))?;

        instance.vault_id = None;

        info!(
            "Service paused and snapshotted: {} (hash: {})",
            instance.manifest.name, snapshot_hash
        );

        Ok(())
    }

    /// Restore a service from snapshot
    pub async fn restore_from_snapshot(&self, instance: &mut ServiceInstance) -> Result<()> {
        if !instance.state.can_restore() {
            return Err(SLMError::ServiceNotRunning(
                instance.manifest.name.clone(),
            ));
        }

        let snapshot = instance.latest_snapshot.as_ref().ok_or_else(|| {
            SLMError::RestoreFailed("No snapshot available".to_string())
        })?;

        debug!("Restoring service: {} from snapshot", instance.manifest.name);

        instance.state = ServiceState::Restoring;

        // Verify snapshot integrity
        self.kernel
            .verify_snapshot(&snapshot.hash)
            .await
            .map_err(|e| SLMError::RestoreFailed(e.to_string()))?;

        // Restore vault
        let vault_id = self
            .kernel
            .restore_vault(&snapshot.hash)
            .await
            .map_err(|e| SLMError::RestoreFailed(e.to_string()))?;

        instance.vault_id = Some(vault_id);
        instance.state = ServiceState::Running;
        instance.last_access_timestamp = current_timestamp();
        instance.consecutive_failures = 0;

        info!(
            "Service restored: {} (vault: {}, snapshot: {})",
            instance.manifest.name, vault_id, snapshot.hash
        );

        Ok(())
    }

    /// Archive old snapshots (move to cold storage)
    pub async fn archive_old_snapshots(&self, instance: &mut ServiceInstance) -> Result<()> {
        let max_snapshots = instance.manifest.quota.max_snapshots as usize;

        if instance.snapshots.len() > max_snapshots {
            debug!(
                "Archiving old snapshots for service: {}",
                instance.manifest.name
            );

            // Keep only latest N snapshots
            let num_to_remove = instance.snapshots.len() - max_snapshots;
            for _ in 0..num_to_remove {
                if let Some(mut old) = instance.snapshots.first().cloned() {
                    old.archived = true;
                    instance.snapshots.remove(0);
                }
            }
        }

        Ok(())
    }

    /// Mark service as failed
    pub fn mark_failed(&self, instance: &mut ServiceInstance, error: &str) {
        warn!(
            "Service failed: {} (error: {})",
            instance.manifest.name, error
        );
        instance.state = ServiceState::Failed;
        instance.consecutive_failures += 1;
        instance.vault_id = None;
    }

    /// Check if service should be restarted from last snapshot
    pub fn should_restart(&self, instance: &ServiceInstance) -> bool {
        instance.consecutive_failures < 3 && instance.latest_snapshot.is_some()
    }

    /// Get time since last access (seconds)
    pub fn time_since_last_access(&self, instance: &ServiceInstance) -> u64 {
        let now = current_timestamp();
        now.saturating_sub(instance.last_access_timestamp)
    }

    /// Update last access timestamp
    pub fn touch_service(&self, instance: &mut ServiceInstance) {
        instance.last_access_timestamp = current_timestamp();
        instance.consecutive_failures = 0;  // Reset failures on successful access
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new(KernelAdapter::new())
    }
}

/// Get current Unix timestamp in seconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> ServiceManifest {
        ServiceManifest {
            name: "test-service".to_string(),
            version: "1.0.0".to_string(),
            binary_hash: "binary123".to_string(),
            capabilities_required: vec![],
            quota: ResourceQuota {
                memory_mb: 256,
                cpu_cores: 1.0,
                cpu_percent_max: 50,
                iops_limit: 1000,
                max_snapshots: 5,
                max_snapshot_size_mb: 256,
            },
            idle_timeout_secs: 300,
            archive_after_hours: 24,
            heartbeat_interval_secs: 10,
            heartbeat_timeout_secs: 5,
            signature: "sig123".to_string(),
        }
    }

    #[tokio::test]
    async fn test_spawn_service() {
        let manager = LifecycleManager::default();
        let manifest = sample_manifest();

        let instance = manager.spawn_service(manifest).await.unwrap();
        assert!(instance.state.is_running());
        assert!(instance.vault_id.is_some());
    }

    #[tokio::test]
    async fn test_pause_and_snapshot() {
        let manager = LifecycleManager::default();
        let manifest = sample_manifest();

        let mut instance = manager.spawn_service(manifest).await.unwrap();
        assert!(instance.state.is_running());

        assert!(manager.pause_and_snapshot(&mut instance).await.is_ok());
        assert!(instance.state.is_paused());
        assert!(instance.vault_id.is_none());
        assert!(instance.latest_snapshot.is_some());
    }

    #[tokio::test]
    async fn test_restore_from_snapshot() {
        let manager = LifecycleManager::default();
        let manifest = sample_manifest();

        let mut instance = manager.spawn_service(manifest).await.unwrap();
        manager.pause_and_snapshot(&mut instance).await.unwrap();

        assert!(instance.state.is_paused());
        assert!(manager.restore_from_snapshot(&mut instance).await.is_ok());
        assert!(instance.state.is_running());
        assert!(instance.vault_id.is_some());
    }

    #[tokio::test]
    async fn test_pause_without_running_service() {
        let manager = LifecycleManager::default();
        let manifest = sample_manifest();
        let mut instance = manager.spawn_service(manifest).await.unwrap();

        manager.pause_and_snapshot(&mut instance).await.unwrap();

        let result = manager.pause_and_snapshot(&mut instance).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_touch_service() {
        let manager = LifecycleManager::default();
        let manifest = sample_manifest();
        let mut instance = ServiceInstance {
            instance_id: Uuid::new_v4(),
            manifest,
            state: ServiceState::Running,
            vault_id: Some(1),
            latest_snapshot: None,
            snapshots: Vec::new(),
            last_access_timestamp: 0,
            consecutive_failures: 5,
            resource_usage: ResourceUsage::default(),
        };

        manager.touch_service(&mut instance);
        assert!(instance.last_access_timestamp > 0);
        assert_eq!(instance.consecutive_failures, 0);
    }

    #[tokio::test]
    async fn test_archive_old_snapshots() {
        let manager = LifecycleManager::default();
        let manifest = sample_manifest();
        let mut instance = ServiceInstance {
            instance_id: Uuid::new_v4(),
            manifest,
            state: ServiceState::Paused,
            vault_id: None,
            latest_snapshot: None,
            last_access_timestamp: current_timestamp(),
            consecutive_failures: 0,
            snapshots: vec![
                Snapshot {
                    hash: "snap1".to_string(),
                    size_bytes: 1024,
                    created_at: chrono::Utc::now(),
                    archived: false,
                },
                Snapshot {
                    hash: "snap2".to_string(),
                    size_bytes: 1024,
                    created_at: chrono::Utc::now(),
                    archived: false,
                },
                Snapshot {
                    hash: "snap3".to_string(),
                    size_bytes: 1024,
                    created_at: chrono::Utc::now(),
                    archived: false,
                },
                Snapshot {
                    hash: "snap4".to_string(),
                    size_bytes: 1024,
                    created_at: chrono::Utc::now(),
                    archived: false,
                },
                Snapshot {
                    hash: "snap5".to_string(),
                    size_bytes: 1024,
                    created_at: chrono::Utc::now(),
                    archived: false,
                },
                Snapshot {
                    hash: "snap6".to_string(),
                    size_bytes: 1024,
                    created_at: chrono::Utc::now(),
                    archived: false,
                },
            ],
            resource_usage: ResourceUsage::default(),
        };

        manager.archive_old_snapshots(&mut instance).await.unwrap();
        assert!(instance.snapshots.len() <= 5);
    }
}
