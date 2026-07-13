use crate::{FtpError, FtpResult, QuotaControl, QuotaUsage, UserId};
use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct QuotaManager {
    quotas: Arc<DashMap<String, QuotaUsage>>,
}

impl QuotaManager {
    pub fn new() -> Self {
        Self {
            quotas: Arc::new(DashMap::new()),
        }
    }

    pub fn quota_count(&self) -> usize {
        self.quotas.len()
    }

    async fn ensure_user(&self, user_id: &UserId) -> FtpResult<()> {
        if !self.quotas.contains_key(&user_id.0) {
            self.quotas.insert(
                user_id.0.clone(),
                QuotaUsage {
                    user_id: user_id.clone(),
                    current_storage_bytes: 0,
                    upload_bytes_today: 0,
                    download_bytes_today: 0,
                    file_count: 0,
                    last_reset: Utc::now(),
                },
            );
        }
        Ok(())
    }

    fn should_reset_daily(&self, last_reset: chrono::DateTime<chrono::Utc>) -> bool {
        let now = Utc::now();
        (now - last_reset).num_hours() >= 24
    }
}

impl Default for QuotaManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl QuotaControl for QuotaManager {
    async fn check_upload_quota(&self, user_id: &UserId, bytes: u64) -> FtpResult<bool> {
        self.ensure_user(user_id).await?;

        if let Some(entry) = self.quotas.get(&user_id.0) {
            let quota = entry.upload_bytes_today + bytes;
            Ok(quota <= 100_000_000)
        } else {
            Ok(true)
        }
    }

    async fn check_download_quota(&self, user_id: &UserId, bytes: u64) -> FtpResult<bool> {
        self.ensure_user(user_id).await?;

        if let Some(entry) = self.quotas.get(&user_id.0) {
            let quota = entry.download_bytes_today + bytes;
            Ok(quota <= 100_000_000)
        } else {
            Ok(true)
        }
    }

    async fn check_storage_quota(&self, user_id: &UserId, bytes: u64) -> FtpResult<bool> {
        self.ensure_user(user_id).await?;

        if let Some(entry) = self.quotas.get(&user_id.0) {
            let quota = entry.current_storage_bytes + bytes;
            Ok(quota <= 1_000_000_000)
        } else {
            Ok(true)
        }
    }

    async fn record_upload(&self, user_id: &UserId, bytes: u64) -> FtpResult<()> {
        self.ensure_user(user_id).await?;

        if let Some(mut entry) = self.quotas.get_mut(&user_id.0) {
            if self.should_reset_daily(entry.last_reset) {
                entry.upload_bytes_today = 0;
                entry.last_reset = Utc::now();
            }
            entry.upload_bytes_today += bytes;
        }

        Ok(())
    }

    async fn record_download(&self, user_id: &UserId, bytes: u64) -> FtpResult<()> {
        self.ensure_user(user_id).await?;

        if let Some(mut entry) = self.quotas.get_mut(&user_id.0) {
            if self.should_reset_daily(entry.last_reset) {
                entry.download_bytes_today = 0;
                entry.last_reset = Utc::now();
            }
            entry.download_bytes_today += bytes;
        }

        Ok(())
    }

    async fn record_storage_usage(&self, user_id: &UserId, bytes: u64) -> FtpResult<()> {
        self.ensure_user(user_id).await?;

        if let Some(mut entry) = self.quotas.get_mut(&user_id.0) {
            entry.current_storage_bytes += bytes;
        }

        Ok(())
    }

    async fn get_quota_usage(&self, user_id: &UserId) -> FtpResult<QuotaUsage> {
        self.ensure_user(user_id).await?;

        self.quotas
            .get(&user_id.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| FtpError::ConfigurationError("Quota not found".to_string()))
    }

    async fn reset_daily_quotas(&self, user_id: &UserId) -> FtpResult<()> {
        self.ensure_user(user_id).await?;

        if let Some(mut entry) = self.quotas.get_mut(&user_id.0) {
            entry.upload_bytes_today = 0;
            entry.download_bytes_today = 0;
            entry.last_reset = Utc::now();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_upload_quota() {
        let manager = QuotaManager::new();
        let user_id = UserId("user1".to_string());

        let allowed = manager.check_upload_quota(&user_id, 50_000_000).await.unwrap();
        assert!(allowed);
    }

    #[tokio::test]
    async fn test_check_download_quota() {
        let manager = QuotaManager::new();
        let user_id = UserId("user1".to_string());

        let allowed = manager.check_download_quota(&user_id, 50_000_000).await.unwrap();
        assert!(allowed);
    }

    #[tokio::test]
    async fn test_check_storage_quota() {
        let manager = QuotaManager::new();
        let user_id = UserId("user1".to_string());

        let allowed = manager.check_storage_quota(&user_id, 500_000_000).await.unwrap();
        assert!(allowed);
    }

    #[tokio::test]
    async fn test_record_upload() {
        let manager = QuotaManager::new();
        let user_id = UserId("user1".to_string());

        manager.record_upload(&user_id, 10_000_000).await.unwrap();

        let usage = manager.get_quota_usage(&user_id).await.unwrap();
        assert_eq!(usage.upload_bytes_today, 10_000_000);
    }

    #[tokio::test]
    async fn test_record_storage() {
        let manager = QuotaManager::new();
        let user_id = UserId("user1".to_string());

        manager.record_storage_usage(&user_id, 100_000).await.unwrap();

        let usage = manager.get_quota_usage(&user_id).await.unwrap();
        assert_eq!(usage.current_storage_bytes, 100_000);
    }

    #[tokio::test]
    async fn test_reset_daily_quotas() {
        let manager = QuotaManager::new();
        let user_id = UserId("user1".to_string());

        manager.record_upload(&user_id, 10_000_000).await.unwrap();
        manager.reset_daily_quotas(&user_id).await.unwrap();

        let usage = manager.get_quota_usage(&user_id).await.unwrap();
        assert_eq!(usage.upload_bytes_today, 0);
    }
}
