use crate::{FailoverConfig, IntegrationError, IntegrationResult, ServiceEndpoint, ServiceId};
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct FailoverManager {
    config: Arc<FailoverConfig>,
    failure_counts: Arc<DashMap<String, AtomicU64>>,
    current_endpoint: Arc<parking_lot::Mutex<ServiceEndpoint>>,
}

impl FailoverManager {
    pub fn new(config: FailoverConfig) -> Self {
        let current = config.primary_endpoint.clone();
        Self {
            config: Arc::new(config),
            failure_counts: Arc::new(DashMap::new()),
            current_endpoint: Arc::new(parking_lot::Mutex::new(current)),
        }
    }

    pub async fn record_failure(&self, endpoint: &ServiceEndpoint) -> IntegrationResult<()> {
        let counter = self
            .failure_counts
            .entry(endpoint.service_id.0.clone())
            .or_insert_with(|| AtomicU64::new(0));

        let count = counter.fetch_add(1, Ordering::Relaxed) + 1;

        if self.config.enable_failover && count >= self.config.failover_threshold_failures {
            self.trigger_failover().await?;
        }

        Ok(())
    }

    pub async fn record_success(&self, endpoint: &ServiceEndpoint) -> IntegrationResult<()> {
        self.failure_counts
            .entry(endpoint.service_id.0.clone())
            .or_insert_with(|| AtomicU64::new(0))
            .store(0, Ordering::Relaxed);

        Ok(())
    }

    pub async fn trigger_failover(&self) -> IntegrationResult<()> {
        if self.config.backup_endpoints.is_empty() {
            return Err(IntegrationError::FailoverError("No backup endpoints available".to_string()));
        }

        let backup = self.config.backup_endpoints[0].clone();
        *self.current_endpoint.lock() = backup;
        Ok(())
    }

    pub async fn get_current_endpoint(&self) -> IntegrationResult<ServiceEndpoint> {
        Ok(self.current_endpoint.lock().clone())
    }

    pub async fn restore_to_primary(&self) -> IntegrationResult<()> {
        *self.current_endpoint.lock() = self.config.primary_endpoint.clone();
        self.failure_counts.clear();
        Ok(())
    }

    pub async fn get_failure_count(&self, endpoint: &ServiceId) -> IntegrationResult<u64> {
        Ok(self
            .failure_counts
            .get(&endpoint.0)
            .map(|entry| entry.load(Ordering::Relaxed))
            .unwrap_or(0))
    }

    pub async fn reset_failure_count(&self, endpoint: &ServiceId) -> IntegrationResult<()> {
        self.failure_counts.insert(endpoint.0.clone(), AtomicU64::new(0));
        Ok(())
    }

    pub fn is_failover_enabled(&self) -> bool {
        self.config.enable_failover
    }

    pub fn backup_count(&self) -> usize {
        self.config.backup_endpoints.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ServiceType;

    fn create_test_config() -> FailoverConfig {
        FailoverConfig {
            enable_failover: true,
            primary_endpoint: ServiceEndpoint {
                service_id: ServiceId("primary".to_string()),
                service_type: ServiceType::WebHosting,
                host: "primary.example.com".to_string(),
                port: 443,
                tls_enabled: true,
            },
            backup_endpoints: vec![ServiceEndpoint {
                service_id: ServiceId("backup1".to_string()),
                service_type: ServiceType::WebHosting,
                host: "backup1.example.com".to_string(),
                port: 443,
                tls_enabled: true,
            }],
            health_check_interval_secs: 30,
            failover_threshold_failures: 3,
        }
    }

    #[tokio::test]
    async fn test_record_failure() {
        let config = create_test_config();
        let manager = FailoverManager::new(config);
        let endpoint = ServiceEndpoint {
            service_id: ServiceId("test".to_string()),
            service_type: ServiceType::WebHosting,
            host: "test.example.com".to_string(),
            port: 443,
            tls_enabled: true,
        };

        manager.record_failure(&endpoint).await.unwrap();
        let count = manager.get_failure_count(&endpoint.service_id).await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_record_success() {
        let config = create_test_config();
        let manager = FailoverManager::new(config);
        let endpoint = ServiceEndpoint {
            service_id: ServiceId("test".to_string()),
            service_type: ServiceType::WebHosting,
            host: "test.example.com".to_string(),
            port: 443,
            tls_enabled: true,
        };

        manager.record_failure(&endpoint).await.unwrap();
        manager.record_success(&endpoint).await.unwrap();

        let count = manager.get_failure_count(&endpoint.service_id).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_trigger_failover() {
        let config = create_test_config();
        let manager = FailoverManager::new(config);

        manager.trigger_failover().await.unwrap();

        let current = manager.get_current_endpoint().await.unwrap();
        assert_eq!(current.service_id.0, "backup1");
    }

    #[tokio::test]
    async fn test_restore_to_primary() {
        let config = create_test_config();
        let manager = FailoverManager::new(config);

        manager.trigger_failover().await.unwrap();
        manager.restore_to_primary().await.unwrap();

        let current = manager.get_current_endpoint().await.unwrap();
        assert_eq!(current.service_id.0, "primary");
    }

    #[tokio::test]
    async fn test_get_current_endpoint() {
        let config = create_test_config();
        let manager = FailoverManager::new(config);

        let endpoint = manager.get_current_endpoint().await.unwrap();
        assert_eq!(endpoint.service_id.0, "primary");
    }

    #[tokio::test]
    async fn test_is_failover_enabled() {
        let config = create_test_config();
        let manager = FailoverManager::new(config);

        assert!(manager.is_failover_enabled());
    }

    #[tokio::test]
    async fn test_backup_count() {
        let config = create_test_config();
        let manager = FailoverManager::new(config);

        assert_eq!(manager.backup_count(), 1);
    }
}
