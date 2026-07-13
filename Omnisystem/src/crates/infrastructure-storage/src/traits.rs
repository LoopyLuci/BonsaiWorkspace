use crate::{
    Bucket, BucketName, FileContent, FileMetadata, FilePath, ListObjectsResponse,
    ObjectData, ObjectKey, ObjectMetadata, StorageResult, Volume, VolumeId, BlockData,
    BlockAddress, Snapshot, DirectoryListing, UploadPart,
};
use async_trait::async_trait;
use std::collections::HashMap;

// Object Storage Trait

#[async_trait]
pub trait ObjectStorage: Send + Sync {
    async fn create_bucket(&self, name: BucketName) -> StorageResult<Bucket>;

    async fn delete_bucket(&self, name: &BucketName) -> StorageResult<()>;

    async fn get_bucket(&self, name: &BucketName) -> StorageResult<Bucket>;

    async fn list_buckets(&self) -> StorageResult<Vec<Bucket>>;

    async fn put_object(
        &self,
        bucket: &BucketName,
        key: ObjectKey,
        data: Vec<u8>,
    ) -> StorageResult<ObjectMetadata>;

    async fn get_object(
        &self,
        bucket: &BucketName,
        key: &ObjectKey,
    ) -> StorageResult<ObjectData>;

    async fn delete_object(
        &self,
        bucket: &BucketName,
        key: &ObjectKey,
    ) -> StorageResult<()>;

    async fn list_objects(
        &self,
        bucket: &BucketName,
        prefix: Option<String>,
        limit: usize,
    ) -> StorageResult<ListObjectsResponse>;

    async fn copy_object(
        &self,
        source_bucket: &BucketName,
        source_key: &ObjectKey,
        dest_bucket: &BucketName,
        dest_key: ObjectKey,
    ) -> StorageResult<ObjectMetadata>;

    async fn head_object(
        &self,
        bucket: &BucketName,
        key: &ObjectKey,
    ) -> StorageResult<ObjectMetadata>;

    async fn get_object_range(
        &self,
        bucket: &BucketName,
        key: &ObjectKey,
        start: u64,
        end: u64,
    ) -> StorageResult<Vec<u8>>;
}

// Block Storage Trait

#[async_trait]
pub trait BlockStorage: Send + Sync {
    async fn create_volume(
        &self,
        name: String,
        size_bytes: u64,
    ) -> StorageResult<Volume>;

    async fn delete_volume(&self, volume_id: &VolumeId) -> StorageResult<()>;

    async fn get_volume(&self, volume_id: &VolumeId) -> StorageResult<Volume>;

    async fn list_volumes(&self) -> StorageResult<Vec<Volume>>;

    async fn write_block(
        &self,
        volume_id: &VolumeId,
        offset: u64,
        data: Vec<u8>,
    ) -> StorageResult<BlockAddress>;

    async fn read_block(
        &self,
        address: &BlockAddress,
    ) -> StorageResult<BlockData>;

    async fn delete_block(
        &self,
        address: &BlockAddress,
    ) -> StorageResult<()>;

    async fn trim_block(
        &self,
        address: &BlockAddress,
    ) -> StorageResult<()>;

    async fn create_snapshot(
        &self,
        volume_id: &VolumeId,
        description: Option<String>,
    ) -> StorageResult<Snapshot>;

    async fn restore_snapshot(
        &self,
        snapshot_id: &str,
    ) -> StorageResult<Volume>;

    async fn resize_volume(
        &self,
        volume_id: &VolumeId,
        new_size: u64,
    ) -> StorageResult<Volume>;
}

// File Storage Trait

#[async_trait]
pub trait FileStorage: Send + Sync {
    async fn create_file(
        &self,
        path: FilePath,
        data: Vec<u8>,
    ) -> StorageResult<FileMetadata>;

    async fn read_file(&self, path: &FilePath) -> StorageResult<FileContent>;

    async fn delete_file(&self, path: &FilePath) -> StorageResult<()>;

    async fn list_directory(&self, path: &FilePath) -> StorageResult<DirectoryListing>;

    async fn create_directory(&self, path: FilePath) -> StorageResult<FileMetadata>;

    async fn copy_file(
        &self,
        source: &FilePath,
        destination: FilePath,
    ) -> StorageResult<FileMetadata>;

    async fn move_file(
        &self,
        source: &FilePath,
        destination: FilePath,
    ) -> StorageResult<FileMetadata>;

    async fn set_permissions(
        &self,
        path: &FilePath,
        permissions: u32,
    ) -> StorageResult<FileMetadata>;

    async fn get_file_metadata(&self, path: &FilePath) -> StorageResult<FileMetadata>;

    async fn append_file(
        &self,
        path: &FilePath,
        data: Vec<u8>,
    ) -> StorageResult<FileMetadata>;

    async fn truncate_file(
        &self,
        path: &FilePath,
        size: u64,
    ) -> StorageResult<FileMetadata>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucket_name_equality() {
        let bucket1 = BucketName("test".to_string());
        let bucket2 = BucketName("test".to_string());
        assert_eq!(bucket1, bucket2);
    }

    #[test]
    fn test_object_key_equality() {
        let key1 = ObjectKey("path/to/object".to_string());
        let key2 = ObjectKey("path/to/object".to_string());
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_volume_id_uniqueness() {
        let vol1 = VolumeId(uuid::Uuid::new_v4());
        let vol2 = VolumeId(uuid::Uuid::new_v4());
        assert_ne!(vol1, vol2);
    }

    #[test]
    fn test_file_path_equality() {
        let path1 = FilePath("/data/file.txt".to_string());
        let path2 = FilePath("/data/file.txt".to_string());
        assert_eq!(path1, path2);
    }
}
