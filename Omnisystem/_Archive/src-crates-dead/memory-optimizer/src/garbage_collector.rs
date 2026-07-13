use crate::{MemoryBlock, GcStatistics, MemoryError, MemoryResult};
use dashmap::DashMap;
use std::sync::Arc;

pub struct GarbageCollector {
    blocks: Arc<DashMap<u64, MemoryBlock>>,
    stats: Arc<GcStatistics>,
}

impl GarbageCollector {
    pub fn new(heap_size_bytes: u64) -> Self {
        Self {
            blocks: Arc::new(DashMap::new()),
            stats: Arc::new(GcStatistics {
                total_collections: 0,
                total_freed_bytes: 0,
                last_collection_time_ms: 0,
                heap_size_bytes,
                heap_used_bytes: 0,
            }),
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
        Ok(())
    }

    pub async fn deallocate(&self, block_id: u64) -> MemoryResult<()> {
        if let Some(block) = self.blocks.remove(&block_id) {
            let (_, removed_block) = block;
            // In real implementation, would update heap stats
            Ok(())
        } else {
            Err(MemoryError::InvalidPointer)
        }
    }

    pub async fn collect(&self) -> MemoryResult<u64> {
        let mut freed_bytes = 0u64;

        for entry in self.blocks.iter() {
            let block = entry.value();
            if block.allocated_at.is_some() && block.size_bytes > 0 {
                freed_bytes += block.size_bytes;
            }
        }

        Ok(freed_bytes)
    }

    pub async fn get_statistics(&self) -> MemoryResult<GcStatistics> {
        Ok(self.stats.as_ref().clone())
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
    }

    #[tokio::test]
    async fn test_deallocate() {
        let gc = GarbageCollector::new(1024 * 1024);
        gc.allocate(1, 4096).await.unwrap();

        let result = gc.deallocate(1).await;
        assert!(result.is_ok());
        assert_eq!(gc.block_count(), 0);
    }

    #[tokio::test]
    async fn test_collect() {
        let gc = GarbageCollector::new(1024 * 1024);
        gc.allocate(1, 4096).await.unwrap();
        gc.allocate(2, 8192).await.unwrap();

        let freed = gc.collect().await.unwrap();
        assert!(freed > 0);
    }
}
