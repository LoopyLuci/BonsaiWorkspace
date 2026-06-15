use crate::{
    BlockAddress, BlockData, BlockStorage, Snapshot, StorageError, StorageResult, Volume,
    VolumeId, VolumeType,
};
use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

pub struct InMemoryBlockStorage {
    volumes: Arc<DashMap<String, Volume>>,
    blocks: Arc<DashMap<String, BlockData>>,
    snapshots: Arc<DashMap<String, Snapshot>>,
    total_blocks: Arc<AtomicU64>,
}

impl InMemoryBlockStorage {
    pub fn new() -> Self {
        Self {
            volumes: Arc::new(DashMap::new()),
            blocks: Arc::new(DashMap::new()),
            snapshots: Arc::new(DashMap::new()),
            total_blocks: Arc::new(AtomicU64::new(0)),
        }
    }

    fn block_key(volume_id: &VolumeId, offset: u64) -> String {
        format!("{}:{}", volume_id.0, offset)
    }

    fn calculate_checksum(data: &[u8]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

impl Default for InMemoryBlockStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BlockStorage for InMemoryBlockStorage {
    async fn create_volume(
        &self,
        name: String,
        size_bytes: u64,
    ) -> StorageResult<Volume> {
        let volume_id = VolumeId(Uuid::new_v4());
        let volume = Volume {
            id: volume_id.clone(),
            name,
            size_bytes,
            used_bytes: 0,
            volume_type: VolumeType::SSD,
            created_at: Utc::now(),
            replication_factor: 1,
            tags: Default::default(),
        };

        self.volumes.insert(volume_id.0.to_string(), volume.clone());
        Ok(volume)
    }

    async fn delete_volume(&self, volume_id: &VolumeId) -> StorageResult<()> {
        if !self.volumes.contains_key(&volume_id.0.to_string()) {
            return Err(StorageError::BlockNotFound("Volume not found".to_string()));
        }

        self.volumes.remove(&volume_id.0.to_string());
        Ok(())
    }

    async fn get_volume(&self, volume_id: &VolumeId) -> StorageResult<Volume> {
        self.volumes
            .get(&volume_id.0.to_string())
            .map(|entry| entry.clone())
            .ok_or_else(|| StorageError::BlockNotFound("Volume not found".to_string()))
    }

    async fn list_volumes(&self) -> StorageResult<Vec<Volume>> {
        Ok(self
            .volumes
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    async fn write_block(
        &self,
        volume_id: &VolumeId,
        offset: u64,
        data: Vec<u8>,
    ) -> StorageResult<BlockAddress> {
        let mut volume = self
            .volumes
            .get_mut(&volume_id.0.to_string())
            .ok_or_else(|| StorageError::BlockNotFound("Volume not found".to_string()))?;

        let data_len = data.len() as u64;
        if volume.used_bytes + data_len > volume.size_bytes {
            return Err(StorageError::InsufficientSpace(
                "Not enough space in volume".to_string(),
            ));
        }

        let checksum = Self::calculate_checksum(&data);
        let address = BlockAddress {
            volume_id: volume_id.clone(),
            offset,
            length: data_len,
        };

        volume.used_bytes += data_len;

        drop(volume);

        self.blocks.insert(
            Self::block_key(volume_id, offset),
            BlockData {
                address: address.clone(),
                data,
                checksum,
            },
        );

        self.total_blocks.fetch_add(1, Ordering::Relaxed);
        Ok(address)
    }

    async fn read_block(
        &self,
        address: &BlockAddress,
    ) -> StorageResult<BlockData> {
        self.blocks
            .get(&Self::block_key(&address.volume_id, address.offset))
            .map(|entry| entry.clone())
            .ok_or_else(|| StorageError::BlockNotFound("Block not found".to_string()))
    }

    async fn delete_block(
        &self,
        address: &BlockAddress,
    ) -> StorageResult<()> {
        let key = Self::block_key(&address.volume_id, address.offset);
        if let Some((_, block)) = self.blocks.remove(&key) {
            if let Ok(mut volume) = self
                .volumes
                .get_mut(&address.volume_id.0.to_string())
                .ok_or_else(|| StorageError::BlockNotFound("Volume not found".to_string()))
            {
                volume.used_bytes = volume.used_bytes.saturating_sub(block.address.length);
            }
        }
        Ok(())
    }

    async fn trim_block(
        &self,
        address: &BlockAddress,
    ) -> StorageResult<()> {
        // In-memory storage: trimming is a no-op but updates metadata
        if self
            .blocks
            .get(&Self::block_key(&address.volume_id, address.offset))
            .is_some()
        {
            Ok(())
        } else {
            Err(StorageError::BlockNotFound("Block not found".to_string()))
        }
    }

    async fn create_snapshot(
        &self,
        volume_id: &VolumeId,
        description: Option<String>,
    ) -> StorageResult<Snapshot> {
        let volume = self.get_volume(volume_id).await?;

        let snapshot = Snapshot {
            id: Uuid::new_v4().to_string(),
            volume_id: volume_id.clone(),
            timestamp: Utc::now(),
            size_bytes: volume.used_bytes,
            description,
        };

        self.snapshots
            .insert(snapshot.id.clone(), snapshot.clone());
        Ok(snapshot)
    }

    async fn restore_snapshot(
        &self,
        snapshot_id: &str,
    ) -> StorageResult<Volume> {
        let snapshot = self
            .snapshots
            .get(snapshot_id)
            .ok_or_else(|| StorageError::BlockNotFound("Snapshot not found".to_string()))?;

        let original_volume = self.get_volume(&snapshot.volume_id).await?;

        let restored = Volume {
            id: VolumeId(Uuid::new_v4()),
            name: format!("{}-restored", original_volume.name),
            size_bytes: original_volume.size_bytes,
            used_bytes: snapshot.size_bytes,
            volume_type: original_volume.volume_type,
            created_at: Utc::now(),
            replication_factor: original_volume.replication_factor,
            tags: original_volume.tags,
        };

        self.volumes
            .insert(restored.id.0.to_string(), restored.clone());
        Ok(restored)
    }

    async fn resize_volume(
        &self,
        volume_id: &VolumeId,
        new_size: u64,
    ) -> StorageResult<Volume> {
        if new_size == 0 {
            return Err(StorageError::InvalidBlockSize("Size must be > 0".to_string()));
        }

        let mut volume = self
            .volumes
            .get_mut(&volume_id.0.to_string())
            .ok_or_else(|| StorageError::BlockNotFound("Volume not found".to_string()))?;

        if new_size < volume.used_bytes {
            return Err(StorageError::InvalidBlockSize(
                "New size cannot be less than used space".to_string(),
            ));
        }

        volume.size_bytes = new_size;
        Ok(volume.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_volume() {
        let storage = InMemoryBlockStorage::new();
        let volume = storage
            .create_volume("data".to_string(), 1024 * 1024)
            .await
            .unwrap();
        assert_eq!(volume.size_bytes, 1024 * 1024);
        assert_eq!(volume.used_bytes, 0);
    }

    #[tokio::test]
    async fn test_write_and_read_block() {
        let storage = InMemoryBlockStorage::new();
        let volume = storage
            .create_volume("disk".to_string(), 1024 * 1024)
            .await
            .unwrap();

        let data = b"test block data".to_vec();
        let address = storage
            .write_block(&volume.id, 0, data.clone())
            .await
            .unwrap();

        let block = storage.read_block(&address).await.unwrap();
        assert_eq!(block.data, data);
    }

    #[tokio::test]
    async fn test_insufficient_space() {
        let storage = InMemoryBlockStorage::new();
        let volume = storage
            .create_volume("small".to_string(), 100)
            .await
            .unwrap();

        let data = vec![0u8; 150];
        let result = storage.write_block(&volume.id, 0, data).await;
        assert!(matches!(result, Err(StorageError::InsufficientSpace(_))));
    }

    #[tokio::test]
    async fn test_delete_block() {
        let storage = InMemoryBlockStorage::new();
        let volume = storage
            .create_volume("disk".to_string(), 1024 * 1024)
            .await
            .unwrap();

        let address = storage
            .write_block(&volume.id, 0, b"data".to_vec())
            .await
            .unwrap();

        storage.delete_block(&address).await.unwrap();

        let result = storage.read_block(&address).await;
        assert!(matches!(result, Err(StorageError::BlockNotFound(_))));
    }

    #[tokio::test]
    async fn test_create_snapshot() {
        let storage = InMemoryBlockStorage::new();
        let volume = storage
            .create_volume("disk".to_string(), 1024 * 1024)
            .await
            .unwrap();

        let snapshot = storage
            .create_snapshot(&volume.id, Some("backup".to_string()))
            .await
            .unwrap();

        assert_eq!(snapshot.volume_id, volume.id);
        assert_eq!(snapshot.description, Some("backup".to_string()));
    }

    #[tokio::test]
    async fn test_resize_volume() {
        let storage = InMemoryBlockStorage::new();
        let volume = storage
            .create_volume("disk".to_string(), 1024)
            .await
            .unwrap();

        let resized = storage
            .resize_volume(&volume.id, 2048)
            .await
            .unwrap();

        assert_eq!(resized.size_bytes, 2048);
    }

    #[tokio::test]
    async fn test_resize_volume_invalid() {
        let storage = InMemoryBlockStorage::new();
        let volume = storage
            .create_volume("disk".to_string(), 1024)
            .await
            .unwrap();

        let result = storage.resize_volume(&volume.id, 0).await;
        assert!(matches!(result, Err(StorageError::InvalidBlockSize(_))));
    }
}
