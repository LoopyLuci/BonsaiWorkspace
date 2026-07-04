use crate::{MemoryPool, MemoryError, MemoryResult};
use dashmap::DashMap;
use std::sync::Arc;

pub struct MemoryPoolManager {
    pools: Arc<DashMap<u64, MemoryPool>>,
}

impl MemoryPoolManager {
    pub fn new() -> Self {
        Self {
            pools: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_pool(
        &self,
        pool_id: u64,
        block_size: u64,
        block_count: u64,
    ) -> MemoryResult<()> {
        let pool = MemoryPool {
            pool_id,
            block_size,
            block_count,
            free_blocks: block_count,
        };

        self.pools.insert(pool_id, pool);
        Ok(())
    }

    pub async fn allocate_from_pool(&self, pool_id: u64) -> MemoryResult<u64> {
        if let Some(mut pool) = self.pools.get_mut(&pool_id) {
            if pool.free_blocks > 0 {
                pool.free_blocks -= 1;
                Ok(pool.block_size)
            } else {
                Err(MemoryError::PoolExhausted)
            }
        } else {
            Err(MemoryError::AllocationFailed)
        }
    }

    pub async fn deallocate_to_pool(&self, pool_id: u64, block_size: u64) -> MemoryResult<()> {
        if let Some(mut pool) = self.pools.get_mut(&pool_id) {
            if pool.free_blocks < pool.block_count {
                pool.free_blocks += 1;
                Ok(())
            } else {
                Err(MemoryError::AllocationFailed)
            }
        } else {
            Err(MemoryError::AllocationFailed)
        }
    }

    pub async fn get_pool_status(&self, pool_id: u64) -> MemoryResult<MemoryPool> {
        self.pools
            .get(&pool_id)
            .map(|p| p.clone())
            .ok_or(MemoryError::AllocationFailed)
    }

    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }
}

impl Default for MemoryPoolManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_pool() {
        let manager = MemoryPoolManager::new();
        let result = manager.create_pool(1, 4096, 256).await;
        assert!(result.is_ok());
        assert_eq!(manager.pool_count(), 1);
    }

    #[tokio::test]
    async fn test_allocate_from_pool() {
        let manager = MemoryPoolManager::new();
        manager.create_pool(1, 4096, 10).await.unwrap();

        let result = manager.allocate_from_pool(1).await;
        assert!(result.is_ok());

        let pool = manager.get_pool_status(1).await.unwrap();
        assert_eq!(pool.free_blocks, 9);
    }

    #[tokio::test]
    async fn test_deallocate_to_pool() {
        let manager = MemoryPoolManager::new();
        manager.create_pool(1, 4096, 10).await.unwrap();

        manager.allocate_from_pool(1).await.unwrap();
        let result = manager.deallocate_to_pool(1, 4096).await;
        assert!(result.is_ok());

        let pool = manager.get_pool_status(1).await.unwrap();
        assert_eq!(pool.free_blocks, 10);
    }
}
