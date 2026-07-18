use crate::{Backup, Snapshot, BackupSchedule, RetentionPolicy, BackupError, BackupResult, BackupType, BackupStatus};
use dashmap::DashMap;
use sha2::{Digest, Sha256};
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
        let backup = self.backups.get(&backup_id).ok_or(BackupError::SnapshotFailed)?;

        // Real SHA-256 digest over the backup's actual recorded metadata,
        // rather than a random UUID mislabeled as a "sha256_" checksum.
        let mut hasher = Sha256::new();
        hasher.update(backup.backup_id.as_bytes());
        hasher.update(backup.resource_id.as_bytes());
        hasher.update(format!("{:?}", backup.backup_type).as_bytes());
        hasher.update(backup.size_bytes.to_le_bytes());
        hasher.update(backup.created_at.timestamp_nanos_opt().unwrap_or(0).to_le_bytes());
        let checksum = format!("{:x}", hasher.finalize());

        // A snapshot can only be considered verified if the underlying
        // backup has actually finished (or is at least being verified);
        // an in-progress backup has nothing trustworthy to snapshot yet.
        let verified = matches!(backup.status, BackupStatus::Completed | BackupStatus::VerifyingIntegrity);

        let snapshot = Snapshot {
            snapshot_id: Uuid::new_v4(),
            backup_id,
            timestamp: Utc::now(),
            checksum,
            verified,
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

        let snapshot = manager.snapshots.get(&snapshot_id).unwrap();
        // Real SHA-256 hex digests are 64 hex characters.
        assert_eq!(snapshot.checksum.len(), 64);
        assert!(snapshot.checksum.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(snapshot.verified, "backup was VerifyingIntegrity so snapshot should be verified");
    }

    #[tokio::test]
    async fn test_create_snapshot_checksum_is_deterministic_and_distinguishes_backups() {
        let manager = BackupManager::new();
        let backup_id = manager.create_backup("db4", BackupType::Full).await.unwrap();
        manager.complete_backup(backup_id, 4096).await.unwrap();

        let snap1 = manager.create_snapshot(backup_id).await.unwrap();
        let snap2 = manager.create_snapshot(backup_id).await.unwrap();
        let checksum1 = manager.snapshots.get(&snap1).unwrap().checksum.clone();
        let checksum2 = manager.snapshots.get(&snap2).unwrap().checksum.clone();
        // Same backup metadata -> same checksum, unlike the old random-UUID stub.
        assert_eq!(checksum1, checksum2);

        let other_backup_id = manager.create_backup("db5", BackupType::Full).await.unwrap();
        manager.complete_backup(other_backup_id, 4096).await.unwrap();
        let snap3 = manager.create_snapshot(other_backup_id).await.unwrap();
        let checksum3 = manager.snapshots.get(&snap3).unwrap().checksum.clone();
        assert_ne!(checksum1, checksum3);
    }

    #[tokio::test]
    async fn test_create_snapshot_of_in_progress_backup_is_unverified() {
        let manager = BackupManager::new();
        // create_backup leaves status as InProgress; complete_backup/verify_backup not called.
        let backup_id = manager.create_backup("db6", BackupType::Full).await.unwrap();

        let snapshot_id = manager.create_snapshot(backup_id).await.unwrap();
        let snapshot = manager.snapshots.get(&snapshot_id).unwrap();
        assert!(!snapshot.verified);
    }

    #[tokio::test]
    async fn test_create_snapshot_unknown_backup_fails() {
        let manager = BackupManager::new();
        let result = manager.create_snapshot(Uuid::new_v4()).await;
        assert!(matches!(result, Err(BackupError::SnapshotFailed)));
    }

    #[tokio::test]
    async fn test_create_schedule() {
        let manager = BackupManager::new();
        manager.create_schedule("resource1", "daily", 30).await.unwrap();
        assert_eq!(manager.schedules.len(), 1);
    }

    #[tokio::test]
    async fn test_cleanup_expired_backups() {
        let manager = BackupManager::new();
        let backup_id = manager.create_backup("db7", BackupType::Full).await.unwrap();

        // Not expired yet with a long retention window.
        let deleted = manager.cleanup_expired_backups("db7", 30).await.unwrap();
        assert_eq!(deleted, 0);
        assert_eq!(manager.backup_count(), 1);

        // A retention window of 0 days means "older than right now", which
        // this backup (created moments ago) should still narrowly survive
        // or be swept depending on timing, so instead force an expired
        // manual backdate through direct map access for a deterministic test.
        {
            let mut backup = manager.backups.get_mut(&backup_id).unwrap();
            backup.created_at = Utc::now() - chrono::Duration::days(60);
        }
        let deleted = manager.cleanup_expired_backups("db7", 30).await.unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(manager.backup_count(), 0);
    }
}
