use crate::{CircuitBreakerError, CircuitBreakerResult};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct HealthCheck {
    pub service_id: String,
    pub healthy: bool,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub check_interval_ms: u64,
}

pub struct HealthCheckManager {
    checks: Arc<DashMap<String, HealthCheck>>,
}

impl HealthCheckManager {
    pub fn new() -> Self {
        Self {
            checks: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_health_check(
        &self,
        service_id: &str,
        check_interval_ms: u64,
    ) -> CircuitBreakerResult<HealthCheck> {
        let check = HealthCheck {
            service_id: service_id.to_string(),
            healthy: true,
            last_check: Utc::now(),
            check_interval_ms,
        };

        self.checks.insert(service_id.to_string(), check.clone());
        Ok(check)
    }

    pub async fn mark_healthy(&self, service_id: &str) -> CircuitBreakerResult<()> {
        if let Some(mut check) = self.checks.get_mut(service_id) {
            check.healthy = true;
            check.last_check = Utc::now();
            Ok(())
        } else {
            Err(CircuitBreakerError::HealthCheckFailed)
        }
    }

    pub async fn mark_unhealthy(&self, service_id: &str) -> CircuitBreakerResult<()> {
        if let Some(mut check) = self.checks.get_mut(service_id) {
            check.healthy = false;
            check.last_check = Utc::now();
            Ok(())
        } else {
            Err(CircuitBreakerError::HealthCheckFailed)
        }
    }

    pub async fn is_healthy(&self, service_id: &str) -> CircuitBreakerResult<bool> {
        if let Some(check) = self.checks.get(service_id) {
            let elapsed = Utc::now()
                .signed_duration_since(check.last_check)
                .num_milliseconds() as u64;

            if !check.healthy && elapsed > check.check_interval_ms {
                Ok(false)
            } else {
                Ok(check.healthy)
            }
        } else {
            Err(CircuitBreakerError::HealthCheckFailed)
        }
    }

    pub async fn get_check(&self, service_id: &str) -> CircuitBreakerResult<HealthCheck> {
        self.checks
            .get(service_id)
            .map(|entry| entry.clone())
            .ok_or(CircuitBreakerError::HealthCheckFailed)
    }

    pub fn check_count(&self) -> usize {
        self.checks.len()
    }
}

impl Default for HealthCheckManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_health_check() {
        let manager = HealthCheckManager::new();
        let check = manager.register_health_check("service-1", 5000).await.unwrap();

        assert_eq!(check.service_id, "service-1");
        assert!(check.healthy);
    }

    #[tokio::test]
    async fn test_mark_healthy() {
        let manager = HealthCheckManager::new();
        manager.register_health_check("service-1", 5000).await.unwrap();
        manager.mark_unhealthy("service-1").await.unwrap();

        manager.mark_healthy("service-1").await.unwrap();
        let check = manager.get_check("service-1").await.unwrap();

        assert!(check.healthy);
    }

    #[tokio::test]
    async fn test_mark_unhealthy() {
        let manager = HealthCheckManager::new();
        manager.register_health_check("service-1", 5000).await.unwrap();

        manager.mark_unhealthy("service-1").await.unwrap();
        let check = manager.get_check("service-1").await.unwrap();

        assert!(!check.healthy);
    }

    #[tokio::test]
    async fn test_is_healthy() {
        let manager = HealthCheckManager::new();
        manager.register_health_check("service-1", 5000).await.unwrap();

        let is_healthy = manager.is_healthy("service-1").await.unwrap();
        assert!(is_healthy);
    }

    #[tokio::test]
    async fn test_is_unhealthy() {
        let manager = HealthCheckManager::new();
        manager.register_health_check("service-1", 5000).await.unwrap();
        manager.mark_unhealthy("service-1").await.unwrap();

        let is_healthy = manager.is_healthy("service-1").await.unwrap();
        assert!(!is_healthy);
    }

    #[tokio::test]
    async fn test_health_check_not_found() {
        let manager = HealthCheckManager::new();
        let result = manager.is_healthy("nonexistent").await;

        assert!(result.is_err());
    }
}
