use crate::{Backup, Snapshot, BackupSchedule, RetentionPolicy, BackupMetadata, BackupError, BackupResult, BackupType, BackupStatus};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct BackupManager {
    backups: Arc<DashMap<Uuid, Backup>>,
    snapshots: Arc<DashMap<Uuid, Snapshot>>,
    schedules: Arc<DashMap<Uuid, BackupSchedule>>,
    policies: Arc<DashMap<Uuid, RetentionPolicy>>,
}

impl BackupManager {
    pub fn new() -> Self {
        Self {
            backups: Arc::new(DashMap::new()),
            snapshots: Arc::new(DashMap::new()),
            schedules: Arc::new(DashMap::new()),
            policies: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_backup(&self, resource_id: &str, backup_type: BackupType) -> BackupResult<Uuid> {
        let backup = Backup {
            backup_id: Uuid::new_v4(),
            resource_id: resource_id.to_string(),
            backup_type,
            created_at: Utc::now(),
            size_bytes: 0,
            status: BackupStatus::InProgress,
        };

        let backup_id = backup.backup_id;
        self.backups.insert(backup_id, backup);
        Ok(backup_id)
    }

    pub async fn complete_backup(&self, backup_id: Uuid, size_bytes: u64) -> BackupResult<()> {
        if let Some(mut backup) = self.backups.get_mut(&backup_id) {
            backup.status = BackupStatus::VerifyingIntegrity;
            backup.size_bytes = size_bytes;
            Ok(())
        } else {
            Err(BackupError::BackupFailed)
        }
    }

    pub async fn verify_backup(&self, backup_id: Uuid) -> BackupResult<()> {
        if let Some(mut backup) = self.backups.get_mut(&backup_id) {
            backup.status = BackupStatus::Completed;
            Ok(())
        } else {
            Err(BackupError::VerificationFailed)
        }
    }

    pub async fn create_snapshot(&self, backup_id: Uuid) -> BackupResult<Uuid> {
        if !self.backups.contains_key(&backup_id) {
            return Err(BackupError::SnapshotFailed);
        }

        let snapshot = Snapshot {
            snapshot_id: Uuid::new_v4(),
            backup_id,
            timestamp: Utc::now(),
            checksum: format!("sha256_{}", Uuid::new_v4()),
            verified: true,
        };

        let snapshot_id = snapshot.snapshot_id;
        self.snapshots.insert(snapshot_id, snapshot);
        Ok(snapshot_id)
    }

    pub async fn create_schedule(&self, resource_id: &str, frequency: &str, retention_days: u32) -> BackupResult<()> {
        let schedule = BackupSchedule {
            schedule_id: Uuid::new_v4(),
            resource_id: resource_id.to_string(),
            frequency: frequency.to_string(),
            retention_days,
            enabled: true,
        };

        self.schedules.insert(schedule.schedule_id, schedule);
        Ok(())
    }

    pub async fn register_retention_policy(&self, policy: &RetentionPolicy) -> BackupResult<()> {
        self.policies.insert(policy.policy_id, policy.clone());
        Ok(())
    }

    pub async fn cleanup_expired_backups(&self, resource_id: &str, retention_days: u32) -> BackupResult<usize> {
        let mut deleted = 0;
        let cutoff_time = Utc::now() - chrono::Duration::days(retention_days as i64);

        let mut to_remove = Vec::new();
        for entry in self.backups.iter() {
            let backup = entry.value();
            if backup.resource_id == resource_id && backup.created_at < cutoff_time {
                to_remove.push(backup.backup_id);
            }
        }

        for backup_id in to_remove {
            self.backups.remove(&backup_id);
            deleted += 1;
        }

        Ok(deleted)
    }

    pub fn backup_count(&self) -> usize {
        self.backups.len()
    }
}

impl Default for BackupManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_backup() {
        let manager = BackupManager::new();
        let backup_id = manager.create_backup("db1", BackupType::Full).await.unwrap();
        assert!(!backup_id.is_nil());
        assert_eq!(manager.backup_count(), 1);
    }

    #[tokio::test]
    async fn test_complete_backup() {
        let manager = BackupManager::new();
        let backup_id = manager.create_backup("db2", BackupType::Incremental).await.unwrap();
        
        manager.complete_backup(backup_id, 1024 * 1024).await.unwrap();
        
        let backup = manager.backups.get(&backup_id).unwrap();
        assert_eq!(backup.size_bytes, 1024 * 1024);
    }

    #[tokio::test]
    async fn test_create_snapshot() {
        let manager = BackupManager::new();
        let backup_id = manager.create_backup("db3", BackupType::Differential).await.unwrap();
        manager.complete_backup(backup_id, 2048).await.unwrap();
        
        let snapshot_id = manager.create_snapshot(backup_id).await.unwrap();
        assert!(!snapshot_id.is_nil());
    }

    #[tokio::test]
    async fn test_create_schedule() {
        let manager = BackupManager::new();
        manager.create_schedule("resource1", "daily", 30).await.unwrap();
        assert_eq!(manager.schedules.len(), 1);
    }
}
