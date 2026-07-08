use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Backup {
    pub backup_id: Uuid,
    pub resource_id: String,
    pub backup_type: BackupType,
    pub created_at: DateTime<Utc>,
    pub size_bytes: u64,
    pub status: BackupStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Copy, Hash)]
pub enum BackupType {
    Full,
    Incremental,
    Differential,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Copy, Hash)]
pub enum BackupStatus {
    Scheduled,
    InProgress,
    Completed,
    VerifyingIntegrity,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub snapshot_id: Uuid,
    pub backup_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub checksum: String,
    pub verified: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackupSchedule {
    pub schedule_id: Uuid,
    pub resource_id: String,
    pub frequency: String,
    pub retention_days: u32,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub policy_id: Uuid,
    pub name: String,
    pub retention_days: u32,
    pub min_backups: u32,
    pub archive_after_days: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub metadata_id: Uuid,
    pub backup_id: Uuid,
    pub resource_type: String,
    pub data_items: u64,
    pub completion_time_seconds: u32,
}
