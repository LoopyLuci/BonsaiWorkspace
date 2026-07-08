use crate::RateLimitResult;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct SlidingWindowLimiter {
    windows: Arc<DashMap<String, Vec<i64>>>,
}

impl SlidingWindowLimiter {
    pub fn new() -> Self {
        Self {
            windows: Arc::new(DashMap::new()),
        }
    }

    pub async fn allow_request(&self, window_id: &str, limit: u64, window_size_ms: u64) -> RateLimitResult<bool> {
        let now = Utc::now().timestamp_millis();
        let cutoff = now - window_size_ms as i64;

        let mut entry = self
            .windows
            .entry(window_id.to_string())
            .or_insert_with(Vec::new);

        entry.retain(|&timestamp| timestamp > cutoff);

        if entry.len() < limit as usize {
            entry.push(now);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn get_current_count(&self, window_id: &str, window_size_ms: u64) -> RateLimitResult<u64> {
        let now = Utc::now().timestamp_millis();
        let cutoff = now - window_size_ms as i64;

        if let Some(mut entry) = self.windows.get_mut(window_id) {
            entry.retain(|&timestamp| timestamp > cutoff);
            Ok(entry.len() as u64)
        } else {
            Ok(0)
        }
    }

    pub async fn reset_window(&self, window_id: &str) -> RateLimitResult<()> {
        if let Some(mut entry) = self.windows.get_mut(window_id) {
            entry.clear();
            Ok(())
        } else {
            self.windows.insert(window_id.to_string(), Vec::new());
            Ok(())
        }
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }
}

impl Default for SlidingWindowLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_allow_request_within_limit() {
        let limiter = SlidingWindowLimiter::new();
        let allowed = limiter.allow_request("window-1", 5, 1000).await.unwrap();

        assert!(allowed);
    }

    #[tokio::test]
    async fn test_allow_request_exceeds_limit() {
        let limiter = SlidingWindowLimiter::new();

        for _ in 0..5 {
            limiter.allow_request("window-1", 5, 1000).await.unwrap();
        }

        let allowed = limiter.allow_request("window-1", 5, 1000).await.unwrap();
        assert!(!allowed);
    }

    #[tokio::test]
    async fn test_get_current_count() {
        let limiter = SlidingWindowLimiter::new();

        for _ in 0..3 {
            limiter.allow_request("window-1", 10, 1000).await.unwrap();
        }

        let count = limiter.get_current_count("window-1", 1000).await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_reset_window() {
        let limiter = SlidingWindowLimiter::new();

        for _ in 0..3 {
            limiter.allow_request("window-1", 10, 1000).await.unwrap();
        }

        limiter.reset_window("window-1").await.unwrap();
        let count = limiter.get_current_count("window-1", 1000).await.unwrap();

        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_window_expiry() {
        let limiter = SlidingWindowLimiter::new();
        limiter.allow_request("window-1", 10, 1).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        let count = limiter.get_current_count("window-1", 1).await.unwrap();

        assert_eq!(count, 0);
    }
}
