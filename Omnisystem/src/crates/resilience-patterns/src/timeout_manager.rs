use crate::{AdaptiveTimeoutConfig, ResilienceError, ResilienceResult, TimeoutPolicy};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;

pub struct TimeoutManager {
    configs: Arc<DashMap<String, AdaptiveTimeoutConfig>>,
    policy: TimeoutPolicy,
}

impl TimeoutManager {
    pub fn new(policy: TimeoutPolicy) -> Self {
        Self {
            configs: Arc::new(DashMap::new()),
            policy,
        }
    }

    pub async fn get_timeout(&self, service_id: &str) -> ResilienceResult<Duration> {
        if !self.policy.adaptive {
            return Ok(Duration::from_millis(self.policy.initial_timeout_ms));
        }

        if let Some(config) = self.configs.get(service_id) {
            Ok(Duration::from_millis(config.current_timeout_ms))
        } else {
            Ok(Duration::from_millis(self.policy.initial_timeout_ms))
        }
    }

    pub async fn update_timeout(
        &self,
        service_id: &str,
        p99_latency_ms: u64,
    ) -> ResilienceResult<()> {
        if !self.policy.adaptive {
            return Ok(());
        }

        let mut new_timeout = (p99_latency_ms as f64 * 1.5) as u64;
        new_timeout = new_timeout.max(self.policy.initial_timeout_ms);
        new_timeout = new_timeout.min(self.policy.max_timeout_ms);

        self.configs
            .entry(service_id.to_string())
            .or_insert_with(|| AdaptiveTimeoutConfig {
                service_id: service_id.to_string(),
                current_timeout_ms: new_timeout,
                p99_latency_ms,
                success_count: 0,
                last_updated: Utc::now(),
            })
            .current_timeout_ms = new_timeout;

        Ok(())
    }

    pub async fn record_success(&self, service_id: &str) -> ResilienceResult<()> {
        if let Some(mut config) = self.configs.get_mut(service_id) {
            config.success_count += 1;
        }
        Ok(())
    }

    pub async fn get_timeout_config(&self, service_id: &str) -> ResilienceResult<AdaptiveTimeoutConfig> {
        self.configs
            .get(service_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| ResilienceError::Internal("Config not found".to_string()))
    }

    pub fn config_count(&self) -> usize {
        self.configs.len()
    }
}

impl Default for TimeoutManager {
    fn default() -> Self {
        Self::new(TimeoutPolicy::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_default_timeout() {
        let manager = TimeoutManager::default();
        let timeout = manager.get_timeout("service-1").await.unwrap();
        assert_eq!(timeout.as_millis(), 5000);
    }

    #[tokio::test]
    async fn test_adaptive_timeout_disabled() {
        let policy = TimeoutPolicy {
            adaptive: false,
            ..Default::default()
        };
        let manager = TimeoutManager::new(policy);

        manager.update_timeout("service-1", 10000).await.unwrap();
        let timeout = manager.get_timeout("service-1").await.unwrap();
        assert_eq!(timeout.as_millis(), 5000);
    }

    #[tokio::test]
    async fn test_update_adaptive_timeout() {
        let policy = TimeoutPolicy {
            adaptive: true,
            ..Default::default()
        };
        let manager = TimeoutManager::new(policy);

        manager.update_timeout("service-1", 3000).await.unwrap();
        let timeout = manager.get_timeout("service-1").await.unwrap();
        assert!(timeout.as_millis() > 3000);
    }

    #[tokio::test]
    async fn test_timeout_max_limit() {
        let policy = TimeoutPolicy {
            adaptive: true,
            max_timeout_ms: 10000,
            ..Default::default()
        };
        let manager = TimeoutManager::new(policy);

        manager.update_timeout("service-1", 50000).await.unwrap();
        let timeout = manager.get_timeout("service-1").await.unwrap();
        assert!(timeout.as_millis() <= 10000);
    }

    #[tokio::test]
    async fn test_record_success() {
        let manager = TimeoutManager::default();
        manager.update_timeout("service-1", 3000).await.unwrap();
        manager.record_success("service-1").await.unwrap();

        let config = manager.get_timeout_config("service-1").await.unwrap();
        assert_eq!(config.success_count, 1);
    }
}
