use crate::{CoordinationError, CoordinationResult, DistributedLock};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct LockManager {
    locks: Arc<DashMap<String, DistributedLock>>,
}

impl LockManager {
    pub fn new() -> Self {
        Self {
            locks: Arc::new(DashMap::new()),
        }
    }

    pub async fn acquire_lock(
        &self,
        lock_id: &str,
        resource_id: &str,
        owner: &str,
        ttl_ms: u64,
    ) -> CoordinationResult<DistributedLock> {
        if self.locks.contains_key(resource_id) {
            return Err(CoordinationError::LockAcquisitionFailed);
        }

        let lock = DistributedLock {
            lock_id: lock_id.to_string(),
            resource_id: resource_id.to_string(),
            owner: owner.to_string(),
            acquired_at: Utc::now(),
            ttl_ms,
        };

        self.locks.insert(resource_id.to_string(), lock.clone());
        Ok(lock)
    }

    pub async fn release_lock(&self, resource_id: &str, owner: &str) -> CoordinationResult<()> {
        if let Some((_, lock)) = self.locks.remove(resource_id) {
            if lock.owner != owner {
                return Err(CoordinationError::LockAcquisitionFailed);
            }
            Ok(())
        } else {
            Err(CoordinationError::LockAcquisitionFailed)
        }
    }

    pub async fn check_lock_expired(&self, resource_id: &str) -> CoordinationResult<bool> {
        if let Some(lock) = self.locks.get(resource_id) {
            let elapsed = Utc::now()
                .signed_duration_since(lock.acquired_at)
                .num_milliseconds() as u64;
            Ok(elapsed > lock.ttl_ms)
        } else {
            Err(CoordinationError::LockAcquisitionFailed)
        }
    }

    pub async fn extend_lock_ttl(
        &self,
        resource_id: &str,
        additional_ms: u64,
    ) -> CoordinationResult<()> {
        if let Some(mut lock) = self.locks.get_mut(resource_id) {
            lock.ttl_ms += additional_ms;
            Ok(())
        } else {
            Err(CoordinationError::LockAcquisitionFailed)
        }
    }

    pub async fn get_lock(&self, resource_id: &str) -> CoordinationResult<DistributedLock> {
        self.locks
            .get(resource_id)
            .map(|entry| entry.clone())
            .ok_or(CoordinationError::LockAcquisitionFailed)
    }

    pub fn active_locks(&self) -> usize {
        self.locks.len()
    }
}

impl Default for LockManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_acquire_lock() {
        let manager = LockManager::new();
        let lock = manager.acquire_lock("lock-1", "resource-1", "service-1", 5000).await.unwrap();

        assert_eq!(lock.lock_id, "lock-1");
        assert_eq!(lock.resource_id, "resource-1");
        assert_eq!(lock.owner, "service-1");
    }

    #[tokio::test]
    async fn test_acquire_lock_conflict() {
        let manager = LockManager::new();
        manager.acquire_lock("lock-1", "resource-1", "service-1", 5000).await.unwrap();

        let result = manager.acquire_lock("lock-2", "resource-1", "service-2", 5000).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_release_lock() {
        let manager = LockManager::new();
        manager.acquire_lock("lock-1", "resource-1", "service-1", 5000).await.unwrap();

        manager.release_lock("resource-1", "service-1").await.unwrap();
        assert_eq!(manager.active_locks(), 0);
    }

    #[tokio::test]
    async fn test_release_lock_wrong_owner() {
        let manager = LockManager::new();
        manager.acquire_lock("lock-1", "resource-1", "service-1", 5000).await.unwrap();

        let result = manager.release_lock("resource-1", "service-2").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_extend_lock_ttl() {
        let manager = LockManager::new();
        manager.acquire_lock("lock-1", "resource-1", "service-1", 5000).await.unwrap();

        manager.extend_lock_ttl("resource-1", 2000).await.unwrap();
        let lock = manager.get_lock("resource-1").await.unwrap();

        assert_eq!(lock.ttl_ms, 7000);
    }

    #[tokio::test]
    async fn test_get_lock() {
        let manager = LockManager::new();
        manager.acquire_lock("lock-1", "resource-1", "service-1", 5000).await.unwrap();

        let lock = manager.get_lock("resource-1").await.unwrap();
        assert_eq!(lock.lock_id, "lock-1");
    }

    #[tokio::test]
    async fn test_check_lock_expired() {
        let manager = LockManager::new();
        manager.acquire_lock("lock-1", "resource-1", "service-1", 1).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        let is_expired = manager.check_lock_expired("resource-1").await.unwrap();
        assert!(is_expired);
    }
}
