use crate::{ResilienceError, ResilienceResult, RetryPolicy, ResilienceMetrics};
use dashmap::DashMap;
use std::sync::Arc;

pub struct RetryManager {
    policy: RetryPolicy,
    metrics: Arc<DashMap<String, ResilienceMetrics>>,
}

impl RetryManager {
    pub fn new(policy: RetryPolicy) -> Self {
        Self {
            policy,
            metrics: Arc::new(DashMap::new()),
        }
    }

    pub fn calculate_backoff(&self, attempt: u32) -> ResilienceResult<std::time::Duration> {
        if attempt > self.policy.max_attempts {
            return Err(ResilienceError::RetriesExhausted);
        }

        let mut backoff = self.policy.initial_backoff_ms as f64
            * self.policy.backoff_multiplier.powi((attempt - 1) as i32);
        backoff = backoff.min(self.policy.max_backoff_ms as f64);

        if self.policy.jitter_enabled {
            let jitter = (backoff * 0.1) as u64;
            let random_jitter = (uuid::Uuid::new_v4().as_bytes()[0] as u64) % jitter;
            backoff = (backoff as u64 + random_jitter) as f64;
            backoff = backoff.min(self.policy.max_backoff_ms as f64);
        }

        Ok(std::time::Duration::from_millis(backoff as u64))
    }

    pub async fn record_retry(
        &self,
        service_id: &str,
        success: bool,
    ) -> ResilienceResult<()> {
        let mut metrics = self
            .metrics
            .entry(service_id.to_string())
            .or_insert_with(|| ResilienceMetrics {
                service_id: service_id.to_string(),
                retry_count: 0,
                success_after_retry: 0,
                failure_after_retry: 0,
                hedge_count: 0,
                hedge_success_count: 0,
                avg_timeout_used_ms: 0.0,
            });

        metrics.retry_count += 1;
        if success {
            metrics.success_after_retry += 1;
        } else {
            metrics.failure_after_retry += 1;
        }

        Ok(())
    }

    pub async fn get_metrics(&self, service_id: &str) -> ResilienceResult<ResilienceMetrics> {
        self.metrics
            .get(service_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| ResilienceError::Internal("Metrics not found".to_string()))
    }

    pub fn metrics_count(&self) -> usize {
        self.metrics.len()
    }

    pub fn is_retryable(&self, attempt: u32) -> bool {
        attempt < self.policy.max_attempts
    }
}

impl Default for RetryManager {
    fn default() -> Self {
        Self::new(RetryPolicy::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_backoff() {
        let manager = RetryManager::default();

        let backoff1 = manager.calculate_backoff(1).unwrap();
        let backoff2 = manager.calculate_backoff(2).unwrap();
        let backoff3 = manager.calculate_backoff(3).unwrap();

        assert!(backoff1.as_millis() < backoff2.as_millis());
        assert!(backoff2.as_millis() < backoff3.as_millis());
    }

    #[test]
    fn test_backoff_max_limit() {
        let policy = RetryPolicy {
            max_backoff_ms: 5000,
            max_attempts: 10,
            ..Default::default()
        };
        let manager = RetryManager::new(policy);

        let backoff = manager.calculate_backoff(10).unwrap();
        assert!(backoff.as_millis() <= 5000);
    }

    #[test]
    fn test_retries_exhausted() {
        let manager = RetryManager::default();
        let result = manager.calculate_backoff(10);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_record_retry_success() {
        let manager = RetryManager::default();

        manager.record_retry("service-1", true).await.unwrap();
        manager.record_retry("service-1", true).await.unwrap();

        let metrics = manager.get_metrics("service-1").await.unwrap();
        assert_eq!(metrics.retry_count, 2);
        assert_eq!(metrics.success_after_retry, 2);
    }

    #[tokio::test]
    async fn test_record_retry_failure() {
        let manager = RetryManager::default();

        manager.record_retry("service-1", false).await.unwrap();
        manager.record_retry("service-1", true).await.unwrap();

        let metrics = manager.get_metrics("service-1").await.unwrap();
        assert_eq!(metrics.failure_after_retry, 1);
        assert_eq!(metrics.success_after_retry, 1);
    }

    #[test]
    fn test_is_retryable() {
        let manager = RetryManager::default();

        assert!(manager.is_retryable(1));
        assert!(manager.is_retryable(2));
        assert!(!manager.is_retryable(10));
    }

    #[test]
    fn test_jitter_variation() {
        let policy = RetryPolicy {
            jitter_enabled: true,
            max_attempts: 5,
            ..Default::default()
        };
        let manager = RetryManager::new(policy);

        let durations: Vec<_> = (0..5)
            .map(|i| manager.calculate_backoff(i + 1).unwrap().as_millis())
            .collect();

        // With jitter, durations should vary
        let unique_durations: std::collections::HashSet<_> = durations.iter().cloned().collect();
        assert!(unique_durations.len() > 1);
    }
}
