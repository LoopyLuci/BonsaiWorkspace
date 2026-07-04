use crate::{
    Bucket, BucketName, ListObjectsResponse, ObjectData, ObjectKey, ObjectMetadata,
    ObjectStorage, StorageClass, StorageError, StorageResult,
};
use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct InMemoryObjectStorage {
    buckets: Arc<DashMap<String, Bucket>>,
    objects: Arc<DashMap<String, ObjectData>>,
}

impl InMemoryObjectStorage {
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(DashMap::new()),
            objects: Arc::new(DashMap::new()),
        }
    }

    fn object_key(bucket: &BucketName, key: &ObjectKey) -> String {
        format!("{}:{}", bucket.0, key.0)
    }

    fn calculate_checksum(data: &[u8]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

impl Default for InMemoryObjectStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ObjectStorage for InMemoryObjectStorage {
    async fn create_bucket(&self, name: BucketName) -> StorageResult<Bucket> {
        if self.buckets.contains_key(&name.0) {
            return Err(StorageError::BucketAlreadyExists(name.0.clone()));
        }

        let bucket = Bucket {
            name: name.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: Default::default(),
            versioning_enabled: false,
            storage_class: StorageClass::Standard,
        };

        self.buckets.insert(name.0, bucket.clone());
        Ok(bucket)
    }

    async fn delete_bucket(&self, name: &BucketName) -> StorageResult<()> {
        if !self.buckets.contains_key(&name.0) {
            return Err(StorageError::BucketNotFound(name.0.clone()));
        }

        self.buckets.remove(&name.0);
        Ok(())
    }

    async fn get_bucket(&self, name: &BucketName) -> StorageResult<Bucket> {
        self.buckets
            .get(&name.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| StorageError::BucketNotFound(name.0.clone()))
    }

    async fn list_buckets(&self) -> StorageResult<Vec<Bucket>> {
        Ok(self
            .buckets
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    async fn put_object(
        &self,
        bucket: &BucketName,
        key: ObjectKey,
        data: Vec<u8>,
    ) -> StorageResult<ObjectMetadata> {
        if !self.buckets.contains_key(&bucket.0) {
            return Err(StorageError::BucketNotFound(bucket.0.clone()));
        }

        let size = data.len() as u64;
        let checksum = Self::calculate_checksum(&data);
        let now = Utc::now();

        let metadata = ObjectMetadata {
            key: key.clone(),
            bucket: bucket.clone(),
            size,
            content_type: "application/octet-stream".to_string(),
            checksum: checksum.clone(),
            created_at: now,
            updated_at: now,
            version_id: None,
            tags: Default::default(),
        };

        let obj_key = Self::object_key(bucket, &key);
        self.objects.insert(
            obj_key,
            ObjectData {
                metadata: metadata.clone(),
                data,
            },
        );

        Ok(metadata)
    }

    async fn get_object(
        &self,
        bucket: &BucketName,
        key: &ObjectKey,
    ) -> StorageResult<ObjectData> {
        let obj_key = Self::object_key(bucket, key);
        self.objects
            .get(&obj_key)
            .map(|entry| entry.clone())
            .ok_or_else(|| StorageError::ObjectNotFound(key.0.clone()))
    }

    async fn delete_object(
        &self,
        bucket: &BucketName,
        key: &ObjectKey,
    ) -> StorageResult<()> {
        let obj_key = Self::object_key(bucket, key);
        self.objects.remove(&obj_key);
        Ok(())
    }

    async fn list_objects(
        &self,
        bucket: &BucketName,
        prefix: Option<String>,
        limit: usize,
    ) -> StorageResult<ListObjectsResponse> {
        if !self.buckets.contains_key(&bucket.0) {
            return Err(StorageError::BucketNotFound(bucket.0.clone()));
        }

        let bucket_prefix = format!("{}:", bucket.0);
        let mut objects: Vec<_> = self
            .objects
            .iter()
            .filter(|entry| entry.key().starts_with(&bucket_prefix))
            .filter(|entry| {
                if let Some(ref p) = prefix {
                    entry.value().metadata.key.0.starts_with(p)
                } else {
                    true
                }
            })
            .map(|entry| entry.value().metadata.clone())
            .collect();

        objects.sort_by(|a, b| a.key.0.cmp(&b.key.0));

        let truncated = objects.len() > limit;
        objects.truncate(limit);

        Ok(ListObjectsResponse {
            bucket: bucket.clone(),
            objects,
            truncated,
            next_token: if truncated { Some("next".to_string()) } else { None },
        })
    }

    async fn copy_object(
        &self,
        source_bucket: &BucketName,
        source_key: &ObjectKey,
        dest_bucket: &BucketName,
        dest_key: ObjectKey,
    ) -> StorageResult<ObjectMetadata> {
        let source_data = self.get_object(source_bucket, source_key).await?;
        self.put_object(dest_bucket, dest_key, source_data.data)
            .await
    }

    async fn head_object(
        &self,
        bucket: &BucketName,
        key: &ObjectKey,
    ) -> StorageResult<ObjectMetadata> {
        self.get_object(bucket, key)
            .await
            .map(|obj| obj.metadata)
    }

    async fn get_object_range(
        &self,
        bucket: &BucketName,
        key: &ObjectKey,
        start: u64,
        end: u64,
    ) -> StorageResult<Vec<u8>> {
        let obj = self.get_object(bucket, key).await?;

        if start >= obj.data.len() as u64 {
            return Err(StorageError::InvalidOffset(
                "Start offset exceeds object size".to_string(),
            ));
        }

        let end = (end as usize).min(obj.data.len());
        Ok(obj.data[start as usize..end].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_bucket() {
        let storage = InMemoryObjectStorage::new();
        let name = BucketName("test-bucket".to_string());
        let bucket = storage.create_bucket(name.clone()).await.unwrap();
        assert_eq!(bucket.name, name);
    }

    #[tokio::test]
    async fn test_duplicate_bucket() {
        let storage = InMemoryObjectStorage::new();
        let name = BucketName("test".to_string());
        storage.create_bucket(name.clone()).await.unwrap();
        let result = storage.create_bucket(name).await;
        assert!(matches!(result, Err(StorageError::BucketAlreadyExists(_))));
    }

    #[tokio::test]
    async fn test_put_and_get_object() {
        let storage = InMemoryObjectStorage::new();
        let bucket = BucketName("data".to_string());
        storage.create_bucket(bucket.clone()).await.unwrap();

        let key = ObjectKey("file.txt".to_string());
        let data = b"Hello, World!".to_vec();
        storage
            .put_object(&bucket, key.clone(), data.clone())
            .await
            .unwrap();

        let retrieved = storage.get_object(&bucket, &key).await.unwrap();
        assert_eq!(retrieved.data, data);
    }

    #[tokio::test]
    async fn test_delete_object() {
        let storage = InMemoryObjectStorage::new();
        let bucket = BucketName("data".to_string());
        storage.create_bucket(bucket.clone()).await.unwrap();

        let key = ObjectKey("file.txt".to_string());
        storage
            .put_object(&bucket, key.clone(), b"test".to_vec())
            .await
            .unwrap();

        storage.delete_object(&bucket, &key).await.unwrap();
        let result = storage.get_object(&bucket, &key).await;
        assert!(matches!(result, Err(StorageError::ObjectNotFound(_))));
    }

    #[tokio::test]
    async fn test_list_objects() {
        let storage = InMemoryObjectStorage::new();
        let bucket = BucketName("files".to_string());
        storage.create_bucket(bucket.clone()).await.unwrap();

        storage
            .put_object(&bucket, ObjectKey("a.txt".to_string()), b"1".to_vec())
            .await
            .unwrap();
        storage
            .put_object(&bucket, ObjectKey("b.txt".to_string()), b"2".to_vec())
            .await
            .unwrap();

        let listing = storage.list_objects(&bucket, None, 10).await.unwrap();
        assert_eq!(listing.objects.len(), 2);
    }

    #[tokio::test]
    async fn test_copy_object() {
        let storage = InMemoryObjectStorage::new();
        let bucket = BucketName("bucket".to_string());
        storage.create_bucket(bucket.clone()).await.unwrap();

        let source_key = ObjectKey("original.txt".to_string());
        let data = b"Original".to_vec();
        storage
            .put_object(&bucket, source_key.clone(), data.clone())
            .await
            .unwrap();

        let dest_key = ObjectKey("copy.txt".to_string());
        storage
            .copy_object(&bucket, &source_key, &bucket, dest_key.clone())
            .await
            .unwrap();

        let copied = storage.get_object(&bucket, &dest_key).await.unwrap();
        assert_eq!(copied.data, data);
    }

    #[tokio::test]
    async fn test_get_object_range() {
        let storage = InMemoryObjectStorage::new();
        let bucket = BucketName("data".to_string());
        storage.create_bucket(bucket.clone()).await.unwrap();

        let key = ObjectKey("file.bin".to_string());
        let data = b"0123456789".to_vec();
        storage
            .put_object(&bucket, key.clone(), data)
            .await
            .unwrap();

        let range = storage.get_object_range(&bucket, &key, 2, 5).await.unwrap();
        assert_eq!(range, b"234");
    }
}
