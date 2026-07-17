use crate::{StorageEntry, MemTable, SSTable, CompressionInfo, CompactionTask, CompactionStatus, WriteAheadLog, StorageError, StorageResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct StorageBackend {
    memtable: Arc<DashMap<String, StorageEntry>>,
    memtable_snapshots: Arc<DashMap<Uuid, MemTable>>,
    sstables: Arc<DashMap<Uuid, SSTable>>,
    compressions: Arc<DashMap<Uuid, CompressionInfo>>,
    compactions: Arc<DashMap<Uuid, CompactionTask>>,
    wal: Arc<DashMap<Uuid, WriteAheadLog>>,
}

impl StorageBackend {
    pub fn new() -> Self {
        Self {
            memtable: Arc::new(DashMap::new()),
            memtable_snapshots: Arc::new(DashMap::new()),
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

    /// Snapshot the current memtable into an immutable `MemTable` record
    /// (the type existed but was never actually constructed by anything).
    pub async fn flush_memtable(&self) -> StorageResult<MemTable> {
        let size_bytes: u64 = self.memtable.iter().map(|e| e.value().size_bytes).sum();
        let snapshot = MemTable {
            memtable_id: Uuid::new_v4(),
            created_at: Utc::now(),
            entry_count: self.memtable.len() as u64,
            size_bytes,
            is_immutable: true,
        };

        self.memtable_snapshots.insert(snapshot.memtable_id, snapshot.clone());
        Ok(snapshot)
    }

    /// Flush the entries whose key falls within `[key_min, key_max]`
    /// (inclusive) into a new SSTable, computing its real entry count and
    /// byte size from those entries -- the original implementation used
    /// the *entire* memtable's length regardless of the given range and a
    /// hardcoded 1024000-byte size regardless of what was actually
    /// written.
    pub async fn create_sstable(&self, level: u32, key_min: &str, key_max: &str) -> StorageResult<SSTable> {
        let matching: Vec<StorageEntry> = self
            .memtable
            .iter()
            .filter(|e| e.key().as_str() >= key_min && e.key().as_str() <= key_max)
            .map(|e| e.value().clone())
            .collect();

        let entry_count = matching.len() as u64;
        let size_bytes = matching.iter().map(|e| e.size_bytes).sum();

        let sstable = SSTable {
            sstable_id: Uuid::new_v4(),
            level,
            key_min: key_min.to_string(),
            key_max: key_max.to_string(),
            entry_count,
            size_bytes,
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

    /// Mark a running compaction task finished. Previously there was no
    /// way to transition a CompactionTask out of `Running` at all, so
    /// every compaction stayed permanently "in progress" and
    /// `completed_at` could never be set.
    pub async fn complete_compaction(&self, task_id: Uuid) -> StorageResult<()> {
        let mut task = self.compactions.get_mut(&task_id).ok_or(StorageError::Other("compaction task not found".to_string()))?;
        task.status = CompactionStatus::Completed;
        task.completed_at = Some(Utc::now());
        Ok(())
    }

    pub async fn fail_compaction(&self, task_id: Uuid) -> StorageResult<()> {
        let mut task = self.compactions.get_mut(&task_id).ok_or(StorageError::Other("compaction task not found".to_string()))?;
        task.status = CompactionStatus::Failed;
        task.completed_at = Some(Utc::now());
        Ok(())
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

    /// Mark a WAL entry as durably synced. `synced_to_disk` was
    /// previously write-only: every log entry was created `false` and
    /// nothing ever flipped it, so there was no way to tell a durable
    /// write from a merely-buffered one.
    pub async fn mark_wal_synced(&self, log_id: Uuid) -> StorageResult<()> {
        let mut log = self.wal.get_mut(&log_id).ok_or(StorageError::Other("WAL entry not found".to_string()))?;
        log.synced_to_disk = true;
        Ok(())
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
    async fn test_get_missing_key_fails() {
        let backend = StorageBackend::new();
        let result = backend.get("missing").await;
        assert!(matches!(result, Err(StorageError::KeyNotFound)));
    }

    #[tokio::test]
    async fn test_create_sstable_computes_real_size_and_count_for_range() {
        let backend = StorageBackend::new();
        backend.put("a", b"data_a").await.unwrap(); // 6 bytes, in range
        backend.put("m", b"middle").await.unwrap(); // 6 bytes, in range
        backend.put("z", b"data_z").await.unwrap(); // 6 bytes, out of ["a","n"] range

        let sstable = backend.create_sstable(0, "a", "n").await.unwrap();
        assert_eq!(sstable.level, 0);
        assert_eq!(sstable.key_min, "a");
        // Only "a" and "m" fall in ["a", "n"]; "z" must be excluded, and
        // the old hardcoded-1024000/full-memtable-length behavior would
        // have gotten both of these wrong.
        assert_eq!(sstable.entry_count, 2);
        assert_eq!(sstable.size_bytes, 12);
    }

    #[tokio::test]
    async fn test_flush_memtable_snapshot_reflects_real_contents() {
        let backend = StorageBackend::new();
        backend.put("x", b"12345").await.unwrap();
        backend.put("y", b"67890").await.unwrap();

        let snapshot = backend.flush_memtable().await.unwrap();
        assert_eq!(snapshot.entry_count, 2);
        assert_eq!(snapshot.size_bytes, 10);
        assert!(snapshot.is_immutable);
    }

    #[tokio::test]
    async fn test_compress_data() {
        let backend = StorageBackend::new();
        let compression = backend.compress_data(1000, 600).await.unwrap();

        assert_eq!(compression.original_size, 1000);
        assert_eq!(compression.compressed_size, 600);
        assert!(compression.compression_ratio < 1.0);
    }

    #[tokio::test]
    async fn test_compaction_lifecycle() {
        let backend = StorageBackend::new();
        let task = backend.start_compaction(1, vec![Uuid::new_v4()]).await.unwrap();
        assert_eq!(task.status, CompactionStatus::Running);
        assert!(task.completed_at.is_none());

        backend.complete_compaction(task.task_id).await.unwrap();
        let stored = backend.compactions.get(&task.task_id).unwrap();
        assert_eq!(stored.status, CompactionStatus::Completed);
        assert!(stored.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_wal_sync_tracking() {
        let backend = StorageBackend::new();
        let log = backend.log_write_ahead(1, "PUT key1").await.unwrap();
        assert!(!log.synced_to_disk);

        backend.mark_wal_synced(log.log_id).await.unwrap();
        let stored = backend.wal.get(&log.log_id).unwrap();
        assert!(stored.synced_to_disk);
    }
}
