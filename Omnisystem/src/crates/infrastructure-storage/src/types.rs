use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// Object Storage Types

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BucketName(pub String);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ObjectKey(pub String);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bucket {
    pub name: BucketName,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
    pub versioning_enabled: bool,
    pub storage_class: StorageClass,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum StorageClass {
    Standard,
    InfrequentAccess,
    Glacier,
    DeepArchive,
}

impl Default for StorageClass {
    fn default() -> Self {
        StorageClass::Standard
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjectMetadata {
    pub key: ObjectKey,
    pub bucket: BucketName,
    pub size: u64,
    pub content_type: String,
    pub checksum: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version_id: Option<String>,
    pub tags: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjectData {
    pub metadata: ObjectMetadata,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UploadPart {
    pub part_number: u32,
    pub size: u64,
    pub checksum: String,
    pub etag: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListObjectsResponse {
    pub bucket: BucketName,
    pub objects: Vec<ObjectMetadata>,
    pub truncated: bool,
    pub next_token: Option<String>,
}

// Block Storage Types

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct VolumeId(pub Uuid);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Copy)]
pub enum VolumeType {
    SSD,
    HDD,
    NVMe,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Volume {
    pub id: VolumeId,
    pub name: String,
    pub size_bytes: u64,
    pub used_bytes: u64,
    pub volume_type: VolumeType,
    pub created_at: DateTime<Utc>,
    pub replication_factor: u32,
    pub tags: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockAddress {
    pub volume_id: VolumeId,
    pub offset: u64,
    pub length: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockData {
    pub address: BlockAddress,
    pub data: Vec<u8>,
    pub checksum: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub volume_id: VolumeId,
    pub timestamp: DateTime<Utc>,
    pub size_bytes: u64,
    pub description: Option<String>,
}

// File Storage Types

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct FilePath(pub String);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Copy)]
pub enum FilePermission {
    Owner,
    Group,
    Other,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: FilePath,
    pub size: u64,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub is_directory: bool,
    pub permissions: u32,
    pub owner: String,
    pub group: String,
    pub checksum: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileContent {
    pub metadata: FileMetadata,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirectoryListing {
    pub path: FilePath,
    pub entries: Vec<FileMetadata>,
    pub total_size: u64,
}

// Replication and Backup Types

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplicationConfig {
    pub factor: u32,
    pub destinations: Vec<String>,
    pub consistency_level: ConsistencyLevel,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConsistencyLevel {
    Strong,
    Eventual,
    Sequential,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackupPolicy {
    pub enabled: bool,
    pub schedule: String,
    pub retention_days: u32,
    pub destination: String,
    pub encryption: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucket_creation() {
        let bucket = Bucket {
            name: BucketName("test-bucket".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: HashMap::new(),
            versioning_enabled: false,
            storage_class: StorageClass::Standard,
        };
        assert_eq!(bucket.name.0, "test-bucket");
    }

    #[test]
    fn test_volume_creation() {
        let volume = Volume {
            id: VolumeId(Uuid::new_v4()),
            name: "data-volume".to_string(),
            size_bytes: 1024 * 1024 * 1024, // 1GB
            used_bytes: 512 * 1024 * 1024,  // 512MB
            volume_type: VolumeType::SSD,
            created_at: Utc::now(),
            replication_factor: 3,
            tags: HashMap::new(),
        };
        assert_eq!(volume.size_bytes, 1024 * 1024 * 1024);
        assert_eq!(volume.used_bytes, 512 * 1024 * 1024);
    }

    #[test]
    fn test_file_metadata() {
        let meta = FileMetadata {
            path: FilePath("/data/file.txt".to_string()),
            size: 1024,
            created_at: Utc::now(),
            modified_at: Utc::now(),
            is_directory: false,
            permissions: 0o644,
            owner: "user".to_string(),
            group: "group".to_string(),
            checksum: Some("abc123".to_string()),
        };
        assert_eq!(meta.size, 1024);
        assert!(!meta.is_directory);
    }

    #[test]
    fn test_storage_class_default() {
        assert_eq!(StorageClass::default(), StorageClass::Standard);
    }

    #[test]
    fn test_consistency_level() {
        assert_ne!(ConsistencyLevel::Strong, ConsistencyLevel::Eventual);
        assert_eq!(ConsistencyLevel::Strong, ConsistencyLevel::Strong);
    }
}
