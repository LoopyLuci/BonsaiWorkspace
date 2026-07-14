use crate::{HealthCheckEvent, HealthError, HealthMonitor, HealthResult, HealthStatus, ProbeConfig, ProbeType};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct HealthChecker {
    monitors: Arc<DashMap<String, HealthMonitor>>,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self {
            monitors: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_monitor(&self, service_id: &str, probe_config: ProbeConfig) -> HealthResult<()> {
        let monitor = HealthMonitor {
            service_id: service_id.to_string(),
            status: HealthStatus::Unknown,
            probe_config,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_check_time: Utc::now(),
            last_status_change: Utc::now(),
            check_history: Vec::new(),
        };

        self.monitors.insert(service_id.to_string(), monitor);
        Ok(())
    }

    pub async fn record_health_check(
        &self,
        service_id: &str,
        endpoint_id: &str,
        healthy: bool,
        duration_ms: u32,
        error_message: Option<String>,
    ) -> HealthResult<()> {
        if let Some(mut monitor) = self.monitors.get_mut(service_id) {
            let new_status = if healthy {
                HealthStatus::Healthy
            } else {
                HealthStatus::Unhealthy
            };

            if healthy {
                monitor.consecutive_successes += 1;
                monitor.consecutive_failures = 0;

                if monitor.consecutive_successes >= monitor.probe_config.success_threshold {
                    if monitor.status != HealthStatus::Healthy {
                        monitor.status = HealthStatus::Healthy;
                        monitor.last_status_change = Utc::now();
                    }
                }
            } else {
                monitor.consecutive_failures += 1;
                monitor.consecutive_successes = 0;

                if monitor.consecutive_failures >= monitor.probe_config.failure_threshold {
                    if monitor.status != HealthStatus::Unhealthy {
                        monitor.status = HealthStatus::Unhealthy;
                        monitor.last_status_change = Utc::now();
                    }
                }
            }

            let event = HealthCheckEvent {
                service_id: service_id.to_string(),
                endpoint_id: endpoint_id.to_string(),
                status: new_status,
                timestamp: Utc::now(),
                duration_ms,
                error_message,
            };

            monitor.check_history.push(event);

            if monitor.check_history.len() > 100 {
                monitor.check_history.remove(0);
            }

            monitor.last_check_time = Utc::now();
        } else {
            return Err(HealthError::MonitorNotFound(service_id.to_string()));
        }

        Ok(())
    }

    pub async fn get_health_status(&self, service_id: &str) -> HealthResult<HealthStatus> {
        self.monitors
            .get(service_id)
            .map(|monitor| monitor.status)
            .ok_or_else(|| HealthError::MonitorNotFound(service_id.to_string()))
    }

    pub async fn get_monitor(&self, service_id: &str) -> HealthResult<HealthMonitor> {
        self.monitors
            .get(service_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| HealthError::MonitorNotFound(service_id.to_string()))
    }

    pub async fn check_http_endpoint(&self, address: &str, port: u16, path: &str) -> HealthResult<u32> {
        let start = std::time::Instant::now();
        let url = format!("http://{}:{}{}", address, port, path);

        let response = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            async {
                reqwest::Client::new()
                    .get(&url)
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await
            },
        )
        .await;

        let duration_ms = start.elapsed().as_millis() as u32;

        match response {
            Ok(Ok(resp)) if resp.status().is_success() => Ok(duration_ms),
            Ok(Ok(resp)) => Err(HealthError::HttpProbeFailure(resp.status().as_u16())),
            _ => Err(HealthError::ProbeTimeout),
        }
    }

    pub async fn check_tcp_endpoint(&self, address: &str, port: u16) -> HealthResult<u32> {
        let start = std::time::Instant::now();

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            tokio::net::TcpStream::connect(format!("{}:{}", address, port)),
        )
        .await;

        let duration_ms = start.elapsed().as_millis() as u32;

        match result {
            Ok(Ok(_)) => Ok(duration_ms),
            _ => Err(HealthError::TcpProbeFailure),
        }
    }

    pub async fn execute_probe(&self, service_id: &str, probe_type: ProbeType) -> HealthResult<u32> {
        let monitor = self
            .monitors
            .get(service_id)
            .ok_or_else(|| HealthError::MonitorNotFound(service_id.to_string()))?;

        match probe_type {
            ProbeType::Http => {
                self.check_http_endpoint("127.0.0.1", monitor.probe_config.port, &monitor.probe_config.path)
                    .await
            }
            ProbeType::Tcp => {
                self.check_tcp_endpoint("127.0.0.1", monitor.probe_config.port)
                    .await
            }
            ProbeType::Exec => Err(HealthError::ExecProbeFailure("Exec probes not supported yet".to_string())),
            ProbeType::Grpc => Err(HealthError::ExecProbeFailure("gRPC probes not supported yet".to_string())),
        }
    }

    pub fn monitor_count(&self) -> usize {
        self.monitors.len()
    }

    pub async fn get_all_monitors(&self) -> HealthResult<Vec<HealthMonitor>> {
        Ok(self.monitors.iter().map(|entry| entry.value().clone()).collect())
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
    async fn test_register_monitor() {
        let checker = HealthChecker::new();
        let config = ProbeConfig::default();

        checker.register_monitor("api-service", config).await.unwrap();
        assert_eq!(checker.monitor_count(), 1);
    }

    #[tokio::test]
    async fn test_record_health_check() {
        let checker = HealthChecker::new();
        let config = ProbeConfig::default();

        checker.register_monitor("api-service", config).await.unwrap();
        checker
            .record_health_check("api-service", "endpoint-1", true, 50, None)
            .await
            .unwrap();

        let status = checker.get_health_status("api-service").await.unwrap();
        assert_eq!(status, HealthStatus::Unknown);
    }

    #[tokio::test]
    async fn test_status_transitions_to_healthy() {
        let checker = HealthChecker::new();
        let mut config = ProbeConfig::default();
        config.success_threshold = 2;

        checker.register_monitor("api-service", config).await.unwrap();

        checker
            .record_health_check("api-service", "endpoint-1", true, 50, None)
            .await
            .unwrap();
        assert_eq!(
            checker.get_health_status("api-service").await.unwrap(),
            HealthStatus::Unknown
        );

        checker
            .record_health_check("api-service", "endpoint-1", true, 50, None)
            .await
            .unwrap();
        assert_eq!(
            checker.get_health_status("api-service").await.unwrap(),
            HealthStatus::Healthy
        );
    }

    #[tokio::test]
    async fn test_status_transitions_to_unhealthy() {
        let checker = HealthChecker::new();
        let mut config = ProbeConfig::default();
        config.success_threshold = 1;
        config.failure_threshold = 2;

        checker.register_monitor("api-service", config).await.unwrap();

        checker
            .record_health_check("api-service", "endpoint-1", true, 50, None)
            .await
            .unwrap();

        for _ in 0..2 {
            checker
                .record_health_check("api-service", "endpoint-1", false, 100, Some("Connection failed".to_string()))
                .await
                .unwrap();
        }

        assert_eq!(
            checker.get_health_status("api-service").await.unwrap(),
            HealthStatus::Unhealthy
        );
    }

    #[tokio::test]
    async fn test_check_monitor() {
        let checker = HealthChecker::new();
        let config = ProbeConfig::default();

        checker.register_monitor("api-service", config).await.unwrap();
        let monitor = checker.get_monitor("api-service").await.unwrap();

        assert_eq!(monitor.service_id, "api-service");
        assert_eq!(monitor.status, HealthStatus::Unknown);
    }

    #[tokio::test]
    async fn test_check_history_retention() {
        let checker = HealthChecker::new();
        let config = ProbeConfig::default();

        checker.register_monitor("api-service", config).await.unwrap();

        for i in 0..150 {
            checker
                .record_health_check("api-service", "endpoint-1", i % 2 == 0, 50 + i as u32, None)
                .await
                .unwrap();
        }

        let monitor = checker.get_monitor("api-service").await.unwrap();
        assert_eq!(monitor.check_history.len(), 100);
    }

    #[tokio::test]
    async fn test_get_all_monitors() {
        let checker = HealthChecker::new();
        let config = ProbeConfig::default();

        checker.register_monitor("service-1", config.clone()).await.unwrap();
        checker.register_monitor("service-2", config.clone()).await.unwrap();
        checker.register_monitor("service-3", config).await.unwrap();

        let monitors = checker.get_all_monitors().await.unwrap();
        assert_eq!(monitors.len(), 3);
    }
}
