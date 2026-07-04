use crate::{BandwidthControl, BandwidthStats, FtpError, FtpResult, SessionId};
use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Debug)]
struct SessionBandwidth {
    #[allow(dead_code)]
    session_id: SessionId,
    upload_bytes: Arc<AtomicU64>,
    download_bytes: Arc<AtomicU64>,
    last_update: Arc<parking_lot::Mutex<chrono::DateTime<Utc>>>,
}

pub struct BandwidthManager {
    sessions: Arc<DashMap<String, SessionBandwidth>>,
}

impl BandwidthManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    async fn ensure_session(&self, session_id: &SessionId) -> FtpResult<()> {
        if !self.sessions.contains_key(&session_id.0.to_string()) {
            self.sessions.insert(
                session_id.0.to_string(),
                SessionBandwidth {
                    session_id: session_id.clone(),
                    upload_bytes: Arc::new(AtomicU64::new(0)),
                    download_bytes: Arc::new(AtomicU64::new(0)),
                    last_update: Arc::new(parking_lot::Mutex::new(Utc::now())),
                },
            );
        }
        Ok(())
    }
}

impl Default for BandwidthManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BandwidthControl for BandwidthManager {
    async fn record_upload(&self, session_id: &SessionId, bytes: u64) -> FtpResult<()> {
        self.ensure_session(session_id).await?;

        if let Some(entry) = self.sessions.get(&session_id.0.to_string()) {
            entry.upload_bytes.fetch_add(bytes, Ordering::Relaxed);
            *entry.last_update.lock() = Utc::now();
        }

        Ok(())
    }

    async fn record_download(&self, session_id: &SessionId, bytes: u64) -> FtpResult<()> {
        self.ensure_session(session_id).await?;

        if let Some(entry) = self.sessions.get(&session_id.0.to_string()) {
            entry.download_bytes.fetch_add(bytes, Ordering::Relaxed);
            *entry.last_update.lock() = Utc::now();
        }

        Ok(())
    }

    async fn check_bandwidth_limit(
        &self,
        session_id: &SessionId,
        bytes: u64,
    ) -> FtpResult<bool> {
        self.ensure_session(session_id).await?;

        if let Some(entry) = self.sessions.get(&session_id.0.to_string()) {
            let current = entry.upload_bytes.load(Ordering::Relaxed)
                + entry.download_bytes.load(Ordering::Relaxed);
            Ok(current + bytes <= 1_000_000_000)
        } else {
            Ok(true)
        }
    }

    async fn get_bandwidth_stats(&self, session_id: &SessionId) -> FtpResult<BandwidthStats> {
        self.ensure_session(session_id).await?;

        if let Some(entry) = self.sessions.get(&session_id.0.to_string()) {
            let upload_bytes = entry.upload_bytes.load(Ordering::Relaxed);
            let download_bytes = entry.download_bytes.load(Ordering::Relaxed);
            let last_update = *entry.last_update.lock();

            let duration = (Utc::now() - last_update).num_seconds() as f64;
            let duration = if duration > 0.0 { duration } else { 1.0 };

            let average_upload_rate = upload_bytes as f64 / duration;
            let average_download_rate = download_bytes as f64 / duration;

            Ok(BandwidthStats {
                session_id: session_id.clone(),
                upload_bytes,
                download_bytes,
                current_upload_rate: average_upload_rate,
                current_download_rate: average_download_rate,
                average_upload_rate,
                average_download_rate,
            })
        } else {
            Err(FtpError::SessionNotFound(session_id.0.to_string()))
        }
    }

    async fn reset_bandwidth(&self, session_id: &SessionId) -> FtpResult<()> {
        self.sessions.remove(&session_id.0.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_record_upload() {
        let manager = BandwidthManager::new();
        let session_id = SessionId(Uuid::new_v4());

        manager.record_upload(&session_id, 1024).await.unwrap();
        assert_eq!(manager.session_count(), 1);
    }

    #[tokio::test]
    async fn test_record_download() {
        let manager = BandwidthManager::new();
        let session_id = SessionId(Uuid::new_v4());

        manager.record_download(&session_id, 2048).await.unwrap();
        let stats = manager.get_bandwidth_stats(&session_id).await.unwrap();
        assert_eq!(stats.download_bytes, 2048);
    }

    #[tokio::test]
    async fn test_bandwidth_stats() {
        let manager = BandwidthManager::new();
        let session_id = SessionId(Uuid::new_v4());

        manager.record_upload(&session_id, 1000).await.unwrap();
        manager.record_download(&session_id, 2000).await.unwrap();

        let stats = manager.get_bandwidth_stats(&session_id).await.unwrap();
        assert_eq!(stats.upload_bytes, 1000);
        assert_eq!(stats.download_bytes, 2000);
    }

    #[tokio::test]
    async fn test_check_bandwidth_limit() {
        let manager = BandwidthManager::new();
        let session_id = SessionId(Uuid::new_v4());

        let allowed = manager.check_bandwidth_limit(&session_id, 500).await.unwrap();
        assert!(allowed);
    }

    #[tokio::test]
    async fn test_reset_bandwidth() {
        let manager = BandwidthManager::new();
        let session_id = SessionId(Uuid::new_v4());

        manager.record_upload(&session_id, 1024).await.unwrap();
        assert_eq!(manager.session_count(), 1);

        manager.reset_bandwidth(&session_id).await.unwrap();
        assert_eq!(manager.session_count(), 0);
    }
}
