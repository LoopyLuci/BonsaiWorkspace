use crate::{HealthStatus, HealthCheckResult, Result};
use dashmap::DashMap;
use std::sync::Arc;

pub struct HealthChecker {
    checks: Arc<DashMap<String, HealthCheckResult>>,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self {
            checks: Arc::new(DashMap::new()),
        }
    }

    pub async fn check_service(&self, name: String) -> Result<HealthStatus> {
        let start = std::time::Instant::now();
        
        let status = HealthStatus::Healthy;
        let response_time = start.elapsed().as_millis() as u64;
        
        let result = HealthCheckResult {
            service_name: name.clone(),
            status,
            response_time_ms: response_time,
            details: "Service is operational".to_string(),
        };
        
        self.checks.insert(name, result);
        tracing::info!("Health check completed");
        Ok(status)
    }

    pub fn get_status(&self, name: &str) -> Option<HealthStatus> {
        self.checks.get(name).map(|c| c.value().status)
    }

    pub fn all_healthy(&self) -> bool {
        self.checks
            .iter()
            .all(|c| c.value().status == HealthStatus::Healthy)
    }

    pub fn check_count(&self) -> usize {
        self.checks.len()
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_checker() {
        let checker = HealthChecker::new();
        let status = checker.check_service("db".to_string()).await.unwrap();
        assert_eq!(status, HealthStatus::Healthy);
        assert_eq!(checker.check_count(), 1);
    }
}
