use crate::{DnsError, DnsResult, HealthChecker, HealthStatus, ServerHealth, ZoneId};
use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct DefaultHealthChecker {
    server_health: Arc<DashMap<String, ServerHealth>>,
}

impl DefaultHealthChecker {
    pub fn new() -> Self {
        Self {
            server_health: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_server(&self, server: String) -> DnsResult<()> {
        self.server_health.insert(
            server.clone(),
            ServerHealth {
                server,
                status: HealthStatus::Unknown,
                last_check: None,
                check_count: 0,
            },
        );
        Ok(())
    }

    pub fn server_count(&self) -> usize {
        self.server_health.len()
    }

    pub async fn get_all_servers(&self) -> DnsResult<Vec<ServerHealth>> {
        Ok(self
            .server_health
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }
}

impl Default for DefaultHealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HealthChecker for DefaultHealthChecker {
    async fn check_health(&self, server: &str) -> DnsResult<HealthStatus> {
        if let Some(mut entry) = self.server_health.get_mut(server) {
            entry.value_mut().check_count += 1;
            entry.value_mut().last_check = Some(Utc::now());
            Ok(entry.value().status.clone())
        } else {
            Err(DnsError::NoHealthyServers(server.to_string()))
        }
    }

    async fn get_server_health(&self, server: &str) -> DnsResult<HealthStatus> {
        self.server_health
            .get(server)
            .map(|entry| entry.status.clone())
            .ok_or_else(|| DnsError::NoHealthyServers(server.to_string()))
    }

    async fn mark_healthy(&self, server: &str) -> DnsResult<()> {
        if let Some(mut entry) = self.server_health.get_mut(server) {
            entry.value_mut().status = HealthStatus::Healthy;
            entry.value_mut().last_check = Some(Utc::now());
            Ok(())
        } else {
            Err(DnsError::NoHealthyServers(server.to_string()))
        }
    }

    async fn mark_unhealthy(&self, server: &str) -> DnsResult<()> {
        if let Some(mut entry) = self.server_health.get_mut(server) {
            entry.value_mut().status = HealthStatus::Unhealthy;
            entry.value_mut().last_check = Some(Utc::now());
            Ok(())
        } else {
            Err(DnsError::NoHealthyServers(server.to_string()))
        }
    }

    async fn get_healthy_servers(&self, _zone_id: &ZoneId) -> DnsResult<Vec<String>> {
        Ok(self
            .server_health
            .iter()
            .filter(|entry| entry.status == HealthStatus::Healthy)
            .map(|entry| entry.server.clone())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_server() {
        let checker = DefaultHealthChecker::new();
        checker.register_server("server1".to_string()).await.unwrap();
        assert_eq!(checker.server_count(), 1);
    }

    #[tokio::test]
    async fn test_mark_healthy() {
        let checker = DefaultHealthChecker::new();
        checker.register_server("server1".to_string()).await.unwrap();

        checker.mark_healthy("server1").await.unwrap();
        let status = checker.get_server_health("server1").await.unwrap();
        assert_eq!(status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_mark_unhealthy() {
        let checker = DefaultHealthChecker::new();
        checker.register_server("server1".to_string()).await.unwrap();

        checker.mark_unhealthy("server1").await.unwrap();
        let status = checker.get_server_health("server1").await.unwrap();
        assert_eq!(status, HealthStatus::Unhealthy);
    }

    #[tokio::test]
    async fn test_check_health_increments_counter() {
        let checker = DefaultHealthChecker::new();
        checker.register_server("server1".to_string()).await.unwrap();
        checker.mark_healthy("server1").await.unwrap();

        checker.check_health("server1").await.unwrap();
        let servers = checker.get_all_servers().await.unwrap();
        assert_eq!(servers[0].check_count, 1);
    }

    #[tokio::test]
    async fn test_get_healthy_servers() {
        let checker = DefaultHealthChecker::new();
        checker.register_server("server1".to_string()).await.unwrap();
        checker.register_server("server2".to_string()).await.unwrap();

        checker.mark_healthy("server1").await.unwrap();
        checker.mark_unhealthy("server2").await.unwrap();

        let zone_id = ZoneId(uuid::Uuid::new_v4());
        let healthy = checker.get_healthy_servers(&zone_id).await.unwrap();
        assert_eq!(healthy.len(), 1);
        assert_eq!(healthy[0], "server1");
    }
}
