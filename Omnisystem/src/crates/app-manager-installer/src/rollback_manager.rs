use crate::{Result, InstallerError};
use app_manager_core::AppId;
use app_manager_core::AppSnapshot;
use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub struct RollbackManager {
    snapshots: Arc<DashMap<AppId, AppSnapshot>>,
}

impl RollbackManager {
    pub fn new() -> Self {
        RollbackManager {
            snapshots: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_snapshot(&self, app_id: &AppId, path: &PathBuf) -> Result<()> {
        tracing::info!("Creating snapshot for {}", app_id);

        let mut snapshot = AppSnapshot::new(app_id.clone(), app_manager_core::Version::new(1, 0, 0));

        if path.exists() {
            let app_data = tokio::fs::read(path)
                .await
                .map_err(|e| InstallerError::IoError(e))?;

            snapshot.save_app_data(app_data);
        }

        self.snapshots.insert(app_id.clone(), snapshot);

        tracing::info!("Snapshot created for {}", app_id);

        Ok(())
    }

    pub async fn restore_snapshot(&self, app_id: &AppId, path: &PathBuf) -> Result<()> {
        tracing::info!("Restoring snapshot for {}", app_id);

        let snapshot = self.snapshots
            .get(app_id)
            .ok_or_else(|| InstallerError::RollbackFailed("No snapshot found".to_string()))?;

        if !snapshot.app_data.is_empty() {
            tokio::fs::write(path, &snapshot.app_data)
                .await
                .map_err(|e| InstallerError::IoError(e))?;
        }

        tracing::info!("Snapshot restored for {}", app_id);

        Ok(())
    }

    pub fn has_snapshot(&self, app_id: &AppId) -> bool {
        self.snapshots.contains_key(app_id)
    }

    pub fn remove_snapshot(&self, app_id: &AppId) -> Result<()> {
        self.snapshots.remove(app_id)
            .ok_or_else(|| InstallerError::RollbackFailed("Snapshot not found".to_string()))?;

        Ok(())
    }

    pub fn clear_all_snapshots(&self) {
        self.snapshots.clear();
    }

    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }
}

impl Default for RollbackManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rollback_manager_creation() {
        let manager = RollbackManager::new();
        assert_eq!(manager.snapshot_count(), 0);
    }

    #[tokio::test]
    async fn test_create_snapshot() {
        let manager = RollbackManager::new();
        let app_id = AppId::new("test-app").unwrap();
        let path = PathBuf::from("/tmp/test");

        manager.create_snapshot(&app_id, &path).await.ok();
        assert_eq!(manager.snapshot_count(), 1);
    }

    #[tokio::test]
    async fn test_has_snapshot() {
        let manager = RollbackManager::new();
        let app_id = AppId::new("test-app").unwrap();
        let path = PathBuf::from("/tmp/test");

        manager.create_snapshot(&app_id, &path).await.ok();
        assert!(manager.has_snapshot(&app_id));
    }

    #[tokio::test]
    async fn test_remove_snapshot() {
        let manager = RollbackManager::new();
        let app_id = AppId::new("test-app").unwrap();
        let path = PathBuf::from("/tmp/test");

        manager.create_snapshot(&app_id, &path).await.ok();
        manager.remove_snapshot(&app_id).unwrap();
        assert!(!manager.has_snapshot(&app_id));
    }

    #[test]
    fn test_clear_all_snapshots() {
        let manager = RollbackManager::new();
        let app_id = AppId::new("test-app").unwrap();

        // Manually insert a snapshot (bypassing the async create)
        manager.snapshots.insert(app_id.clone(), AppSnapshot::new(app_id, app_manager_core::Version::new(1, 0, 0)));
        assert_eq!(manager.snapshot_count(), 1);

        manager.clear_all_snapshots();
        assert_eq!(manager.snapshot_count(), 0);
    }
}
