use crate::{DistributedQuota, RateLimitError, RateLimitResult};
use chrono::{Duration, Utc};
use dashmap::DashMap;
use std::sync::Arc;

pub struct QuotaManager {
    quotas: Arc<DashMap<String, DistributedQuota>>,
}

impl QuotaManager {
    pub fn new() -> Self {
        Self {
            quotas: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_quota(
        &self,
        quota_id: &str,
        service_id: &str,
        total_allowance: u64,
    ) -> RateLimitResult<DistributedQuota> {
        let reset_at = Utc::now() + Duration::hours(1);
        let quota = DistributedQuota {
            quota_id: quota_id.to_string(),
            service_id: service_id.to_string(),
            total_allowance,
            used_allowance: 0,
            reset_at,
        };

        self.quotas.insert(quota_id.to_string(), quota.clone());
        Ok(quota)
    }

    pub async fn consume_quota(&self, quota_id: &str, amount: u64) -> RateLimitResult<u64> {
        if let Some(mut quota) = self.quotas.get_mut(quota_id) {
            self.check_and_reset_quota(&mut quota);

            if quota.used_allowance + amount > quota.total_allowance {
                return Err(RateLimitError::QuotaExceeded);
            }

            quota.used_allowance += amount;
            Ok(quota.total_allowance - quota.used_allowance)
        } else {
            Err(RateLimitError::BucketNotFound)
        }
    }

    pub async fn get_remaining_quota(&self, quota_id: &str) -> RateLimitResult<u64> {
        if let Some(quota) = self.quotas.get(quota_id) {
            Ok(quota.total_allowance - quota.used_allowance)
        } else {
            Err(RateLimitError::BucketNotFound)
        }
    }

    pub async fn reset_quota(&self, quota_id: &str) -> RateLimitResult<()> {
        if let Some(mut quota) = self.quotas.get_mut(quota_id) {
            quota.used_allowance = 0;
            quota.reset_at = Utc::now() + Duration::hours(1);
            Ok(())
        } else {
            Err(RateLimitError::BucketNotFound)
        }
    }

    fn check_and_reset_quota(&self, quota: &mut DistributedQuota) {
        if Utc::now() > quota.reset_at {
            quota.used_allowance = 0;
            quota.reset_at = Utc::now() + Duration::hours(1);
        }
    }

    pub async fn get_quota(&self, quota_id: &str) -> RateLimitResult<DistributedQuota> {
        self.quotas
            .get(quota_id)
            .map(|entry| entry.clone())
            .ok_or(RateLimitError::BucketNotFound)
    }

    pub fn quota_count(&self) -> usize {
        self.quotas.len()
    }
}

impl Default for QuotaManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_quota() {
        let manager = QuotaManager::new();
        let quota = manager.create_quota("quota-1", "service-1", 1000).await.unwrap();

        assert_eq!(quota.quota_id, "quota-1");
        assert_eq!(quota.total_allowance, 1000);
        assert_eq!(quota.used_allowance, 0);
    }

    #[tokio::test]
    async fn test_consume_quota() {
        let manager = QuotaManager::new();
        manager.create_quota("quota-1", "service-1", 1000).await.unwrap();

        let remaining = manager.consume_quota("quota-1", 300).await.unwrap();
        assert_eq!(remaining, 700);
    }

    #[tokio::test]
    async fn test_consume_quota_exceeds() {
        let manager = QuotaManager::new();
        manager.create_quota("quota-1", "service-1", 100).await.unwrap();

        let result = manager.consume_quota("quota-1", 150).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_remaining_quota() {
        let manager = QuotaManager::new();
        manager.create_quota("quota-1", "service-1", 1000).await.unwrap();
        manager.consume_quota("quota-1", 300).await.unwrap();

        let remaining = manager.get_remaining_quota("quota-1").await.unwrap();
        assert_eq!(remaining, 700);
    }

    #[tokio::test]
    async fn test_reset_quota() {
        let manager = QuotaManager::new();
        manager.create_quota("quota-1", "service-1", 1000).await.unwrap();
        manager.consume_quota("quota-1", 300).await.unwrap();

        manager.reset_quota("quota-1").await.unwrap();
        let quota = manager.get_quota("quota-1").await.unwrap();

        assert_eq!(quota.used_allowance, 0);
    }

    #[tokio::test]
    async fn test_quota_not_found() {
        let manager = QuotaManager::new();
        let result = manager.consume_quota("nonexistent", 100).await;

        assert!(result.is_err());
    }
}
