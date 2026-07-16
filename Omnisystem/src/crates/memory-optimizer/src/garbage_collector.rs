use crate::{GcStatistics, MemoryBlock, MemoryError, MemoryResult};
use dashmap::DashMap;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct GarbageCollector {
    blocks: Arc<DashMap<u64, MemoryBlock>>,
    stats: Arc<Mutex<GcStatistics>>,
}

impl GarbageCollector {
    pub fn new(heap_size_bytes: u64) -> Self {
        Self {
            blocks: Arc::new(DashMap::new()),
            stats: Arc::new(Mutex::new(GcStatistics {
                total_collections: 0,
                total_freed_bytes: 0,
                last_collection_time_ms: 0,
                heap_size_bytes,
                heap_used_bytes: 0,
            })),
        }
    }

    pub async fn allocate(&self, block_id: u64, size_bytes: u64) -> MemoryResult<()> {
        let block = MemoryBlock {
            block_id,
            size_bytes,
            allocated: true,
            allocated_at: Some(chrono::Utc::now()),
        };

        self.blocks.insert(block_id, block);
        self.stats.lock().heap_used_bytes += size_bytes;
        Ok(())
    }

    pub async fn deallocate(&self, block_id: u64) -> MemoryResult<()> {
        if let Some((_, removed_block)) = self.blocks.remove(&block_id) {
            let mut stats = self.stats.lock();
            stats.heap_used_bytes = stats.heap_used_bytes.saturating_sub(removed_block.size_bytes);
            Ok(())
        } else {
            Err(MemoryError::InvalidPointer)
        }
    }

    /// Sweep every block still tracked (i.e. not yet explicitly
    /// deallocated), actually removing it and returning the real number
    /// of bytes reclaimed. Also updates `total_collections`,
    /// `total_freed_bytes`, `heap_used_bytes`, and
    /// `last_collection_time_ms` in the real statistics -- previously
    /// collect() only *summed* currently-allocated block sizes without
    /// removing anything, and get_statistics() always returned the same
    /// zeroed struct captured at construction time.
    pub async fn collect(&self) -> MemoryResult<u64> {
        let start = std::time::Instant::now();

        let ids: Vec<u64> = self.blocks.iter().map(|entry| *entry.key()).collect();
        let mut freed_bytes = 0u64;
        for id in ids {
            if let Some((_, block)) = self.blocks.remove(&id) {
                freed_bytes += block.size_bytes;
            }
        }

        let mut stats = self.stats.lock();
        stats.total_collections += 1;
        stats.total_freed_bytes += freed_bytes;
        stats.heap_used_bytes = stats.heap_used_bytes.saturating_sub(freed_bytes);
        stats.last_collection_time_ms = start.elapsed().as_millis() as u64;

        Ok(freed_bytes)
    }

    pub async fn get_statistics(&self) -> MemoryResult<GcStatistics> {
        Ok(self.stats.lock().clone())
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }
}

impl Default for GarbageCollector {
    fn default() -> Self {
        Self::new(1024 * 1024 * 1024) // 1GB
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_allocate() {
        let gc = GarbageCollector::new(1024 * 1024);
        let result = gc.allocate(1, 4096).await;
        assert!(result.is_ok());
        assert_eq!(gc.block_count(), 1);
        assert_eq!(gc.get_statistics().await.unwrap().heap_used_bytes, 4096);
    }

    #[tokio::test]
    async fn test_deallocate() {
        let gc = GarbageCollector::new(1024 * 1024);
        gc.allocate(1, 4096).await.unwrap();

        let result = gc.deallocate(1).await;
        assert!(result.is_ok());
        assert_eq!(gc.block_count(), 0);
        assert_eq!(gc.get_statistics().await.unwrap().heap_used_bytes, 0);
    }

    #[tokio::test]
    async fn test_deallocate_missing_block_errors() {
        let gc = GarbageCollector::new(1024 * 1024);
        assert_eq!(gc.deallocate(999).await, Err(MemoryError::InvalidPointer));
    }

    #[tokio::test]
    async fn test_collect_actually_reclaims_blocks() {
        let gc = GarbageCollector::new(1024 * 1024);
        gc.allocate(1, 4096).await.unwrap();
        gc.allocate(2, 8192).await.unwrap();

        let freed = gc.collect().await.unwrap();
        assert_eq!(freed, 4096 + 8192);
        // collect() must actually remove the blocks, not just sum them.
        assert_eq!(gc.block_count(), 0);
    }

    #[tokio::test]
    async fn test_collect_updates_real_statistics() {
        let gc = GarbageCollector::new(1024 * 1024);
        gc.allocate(1, 4096).await.unwrap();

        gc.collect().await.unwrap();
        let stats = gc.get_statistics().await.unwrap();

        assert_eq!(stats.total_collections, 1);
        assert_eq!(stats.total_freed_bytes, 4096);
        assert_eq!(stats.heap_used_bytes, 0);
    }
}
