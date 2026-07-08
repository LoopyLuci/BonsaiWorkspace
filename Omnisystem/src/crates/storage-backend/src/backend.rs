use crate::{StorageEntry, MemTable, SSTable, CompressionInfo, CompactionTask, CompactionStatus, WriteAheadLog, StorageError, StorageResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct StorageBackend {
    memtable: Arc<DashMap<String, StorageEntry>>,
    sstables: Arc<DashMap<Uuid, SSTable>>,
    compressions: Arc<DashMap<Uuid, CompressionInfo>>,
    compactions: Arc<DashMap<Uuid, CompactionTask>>,
    wal: Arc<DashMap<Uuid, WriteAheadLog>>,
}

impl StorageBackend {
    pub fn new() -> Self {
        Self {
            memtable: Arc::new(DashMap::new()),
            sstables: Arc::new(DashMap::new()),
            compressions: Arc::new(DashMap::new()),
            compactions: Arc::new(DashMap::new()),
            wal: Arc::new(DashMap::new()),
        }
    }

    pub async fn put(&self, key: &str, value: &[u8]) -> StorageResult<StorageEntry> {
        let entry = StorageEntry {
            entry_id: Uuid::new_v4(),
            key: key.to_string(),
            value: value.to_vec(),
            timestamp: Utc::now(),
            size_bytes: value.len() as u64,
        };

        self.memtable.insert(key.to_string(), entry.clone());
        Ok(entry)
    }

    pub async fn get(&self, key: &str) -> StorageResult<StorageEntry> {
        self.memtable
            .get(key)
            .map(|e| e.value().clone())
            .ok_or(StorageError::KeyNotFound)
    }

    pub async fn create_sstable(&self, level: u32, key_min: &str, key_max: &str) -> StorageResult<SSTable> {
        let sstable = SSTable {
            sstable_id: Uuid::new_v4(),
            level,
            key_min: key_min.to_string(),
            key_max: key_max.to_string(),
            entry_count: self.memtable.len() as u64,
            size_bytes: 1024000,
            created_at: Utc::now(),
        };

        self.sstables.insert(sstable.sstable_id, sstable.clone());
        Ok(sstable)
    }

    pub async fn compress_data(&self, original_size: u64, compressed_size: u64) -> StorageResult<CompressionInfo> {
        let ratio = (compressed_size as f32) / (original_size as f32);

        let info = CompressionInfo {
            compression_id: Uuid::new_v4(),
            original_size,
            compressed_size,
            compression_ratio: ratio,
            algorithm: "snappy".to_string(),
        };

        self.compressions.insert(info.compression_id, info.clone());
        Ok(info)
    }

    pub async fn start_compaction(&self, level: u32, sstable_ids: Vec<Uuid>) -> StorageResult<CompactionTask> {
        let task = CompactionTask {
            task_id: Uuid::new_v4(),
            level,
            sstables: sstable_ids,
            status: CompactionStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
        };

        self.compactions.insert(task.task_id, task.clone());
        Ok(task)
    }

    pub async fn log_write_ahead(&self, sequence_number: u64, operation: &str) -> StorageResult<WriteAheadLog> {
        let log = WriteAheadLog {
            log_id: Uuid::new_v4(),
            sequence_number,
            operation: operation.to_string(),
            timestamp: Utc::now(),
            synced_to_disk: false,
        };

        self.wal.insert(log.log_id, log.clone());
        Ok(log)
    }

    pub fn entry_count(&self) -> usize {
        self.memtable.len()
    }
}

impl Default for StorageBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_put() {
        let backend = StorageBackend::new();
        let entry = backend.put("key1", b"value1").await.unwrap();

        assert_eq!(entry.key, "key1");
        assert_eq!(backend.entry_count(), 1);
    }

    #[tokio::test]
    async fn test_get() {
        let backend = StorageBackend::new();
        backend.put("key2", b"value2").await.unwrap();

        let entry = backend.get("key2").await.unwrap();
        assert_eq!(entry.value, b"value2");
    }

    #[tokio::test]
    async fn test_create_sstable() {
        let backend = StorageBackend::new();
        backend.put("a", b"data_a").await.unwrap();
        backend.put("z", b"data_z").await.unwrap();

        let sstable = backend.create_sstable(0, "a", "z").await.unwrap();
        assert_eq!(sstable.level, 0);
        assert_eq!(sstable.key_min, "a");
    }

    #[tokio::test]
    async fn test_compress_data() {
        let backend = StorageBackend::new();
        let compression = backend.compress_data(1000, 600).await.unwrap();

        assert_eq!(compression.original_size, 1000);
        assert_eq!(compression.compressed_size, 600);
        assert!(compression.compression_ratio < 1.0);
    }
}
