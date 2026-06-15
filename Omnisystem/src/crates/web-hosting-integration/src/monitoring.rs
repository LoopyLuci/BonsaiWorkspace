use crate::{HealthStatus, IntegrationResult, PerformanceMetrics, ServiceHealth, ServiceId};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct HealthMonitor {
    service_health: Arc<DashMap<String, ServiceHealth>>,
    metrics: Arc<DashMap<String, PerformanceMetrics>>,
    request_counts: Arc<DashMap<String, AtomicU64>>,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            service_health: Arc::new(DashMap::new()),
            metrics: Arc::new(DashMap::new()),
            request_counts: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_service(&self, service_id: &ServiceId) -> IntegrationResult<()> {
        self.service_health.insert(
            service_id.0.clone(),
            ServiceHealth {
                service_id: service_id.clone(),
                status: HealthStatus::Unknown,
                last_check: Utc::now(),
                response_time_ms: 0.0,
                error_count: 0,
                success_count: 0,
            },
        );

        self.request_counts
            .insert(service_id.0.clone(), AtomicU64::new(0));

        Ok(())
    }

    pub async fn update_health(
        &self,
        service_id: &ServiceId,
        status: HealthStatus,
        response_time_ms: f64,
    ) -> IntegrationResult<()> {
        if let Some(mut entry) = self.service_health.get_mut(&service_id.0) {
            entry.status = status.clone();
            entry.response_time_ms = response_time_ms;
            entry.last_check = Utc::now();

            match status {
                HealthStatus::Healthy => {
                    entry.success_count += 1;
                }
                HealthStatus::Unhealthy => {
                    entry.error_count += 1;
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub async fn get_service_health(&self, service_id: &ServiceId) -> IntegrationResult<ServiceHealth> {
        self.service_health
            .get(&service_id.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| crate::IntegrationError::ServiceUnavailable(service_id.0.clone()))
    }

    pub async fn get_all_health(&self) -> IntegrationResult<Vec<ServiceHealth>> {
        Ok(self
            .service_health
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    pub async fn record_request(&self, service_id: &ServiceId) -> IntegrationResult<()> {
        let counter = self
            .request_counts
            .entry(service_id.0.clone())
            .or_insert_with(|| AtomicU64::new(0));

        counter.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub async fn record_metrics(
        &self,
        service_id: &ServiceId,
        avg_response_time: f64,
        p95_response_time: f64,
        p99_response_time: f64,
        error_rate: f64,
        throughput: f64,
    ) -> IntegrationResult<()> {
        let metrics = PerformanceMetrics {
            service_id: service_id.clone(),
            request_count: self
                .request_counts
                .get(&service_id.0)
                .map(|entry| entry.load(Ordering::Relaxed))
                .unwrap_or(0),
            average_response_time_ms: avg_response_time,
            p95_response_time_ms: p95_response_time,
            p99_response_time_ms: p99_response_time,
            error_rate,
            throughput_requests_per_sec: throughput,
        };

        self.metrics.insert(service_id.0.clone(), metrics);
        Ok(())
    }

    pub async fn get_metrics(&self, service_id: &ServiceId) -> IntegrationResult<PerformanceMetrics> {
        self.metrics
            .get(&service_id.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| crate::IntegrationError::Internal("Metrics not found".to_string()))
    }

    pub fn service_count(&self) -> usize {
        self.service_health.len()
    }

    pub async fn get_healthy_services(&self) -> IntegrationResult<Vec<ServiceHealth>> {
        let healthy: Vec<ServiceHealth> = self
            .service_health
            .iter()
            .filter(|entry| entry.value().status == HealthStatus::Healthy)
            .map(|entry| entry.value().clone())
            .collect();

        Ok(healthy)
    }

    pub async fn get_unhealthy_services(&self) -> IntegrationResult<Vec<ServiceHealth>> {
        let unhealthy: Vec<ServiceHealth> = self
            .service_health
            .iter()
            .filter(|entry| entry.value().status == HealthStatus::Unhealthy)
            .map(|entry| entry.value().clone())
            .collect();

        Ok(unhealthy)
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_service() {
        let monitor = HealthMonitor::new();
        let service_id = ServiceId("service1".to_string());

        monitor.register_service(&service_id).await.unwrap();
        assert_eq!(monitor.service_count(), 1);
    }

    #[tokio::test]
    async fn test_update_health() {
        let monitor = HealthMonitor::new();
        let service_id = ServiceId("service1".to_string());

        monitor.register_service(&service_id).await.unwrap();
        monitor
            .update_health(&service_id, HealthStatus::Healthy, 10.5)
            .await
            .unwrap();

        let health = monitor.get_service_health(&service_id).await.unwrap();
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.response_time_ms, 10.5);
    }

    #[tokio::test]
    async fn test_record_request() {
        let monitor = HealthMonitor::new();
        let service_id = ServiceId("service1".to_string());

        monitor.register_service(&service_id).await.unwrap();
        monitor.record_request(&service_id).await.unwrap();
        monitor.record_request(&service_id).await.unwrap();

        let health = monitor.get_service_health(&service_id).await.unwrap();
        assert_eq!(health.service_id, service_id);
    }

    #[tokio::test]
    async fn test_record_metrics() {
        let monitor = HealthMonitor::new();
        let service_id = ServiceId("service1".to_string());

        monitor.register_service(&service_id).await.unwrap();
        monitor
            .record_metrics(&service_id, 25.0, 50.0, 100.0, 0.01, 1000.0)
            .await
            .unwrap();

        let metrics = monitor.get_metrics(&service_id).await.unwrap();
        assert_eq!(metrics.average_response_time_ms, 25.0);
        assert_eq!(metrics.p95_response_time_ms, 50.0);
    }

    #[tokio::test]
    async fn test_get_healthy_services() {
        let monitor = HealthMonitor::new();
        let service1 = ServiceId("service1".to_string());
        let service2 = ServiceId("service2".to_string());

        monitor.register_service(&service1).await.unwrap();
        monitor.register_service(&service2).await.unwrap();

        monitor
            .update_health(&service1, HealthStatus::Healthy, 10.0)
            .await
            .unwrap();

        monitor
            .update_health(&service2, HealthStatus::Unhealthy, 50.0)
            .await
            .unwrap();

        let healthy = monitor.get_healthy_services().await.unwrap();
        assert_eq!(healthy.len(), 1);
    }

    #[tokio::test]
    async fn test_get_unhealthy_services() {
        let monitor = HealthMonitor::new();
        let service_id = ServiceId("service1".to_string());

        monitor.register_service(&service_id).await.unwrap();
        monitor
            .update_health(&service_id, HealthStatus::Unhealthy, 100.0)
            .await
            .unwrap();

        let unhealthy = monitor.get_unhealthy_services().await.unwrap();
        assert_eq!(unhealthy.len(), 1);
    }
}
