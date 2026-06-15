use crate::{
    HealthCheckResult, InfraResult, LoadBalancerPolicy,
    RegistrySnapshot, ServiceDefinition, ServiceId, ServiceInstance,
};
use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait ServiceRegistry: Send + Sync {
    async fn register_service(
        &self,
        definition: ServiceDefinition,
    ) -> InfraResult<()>;

    async fn deregister_service(&self, service_id: &ServiceId) -> InfraResult<()>;

    async fn register_instance(&self, instance: ServiceInstance) -> InfraResult<()>;

    async fn deregister_instance(
        &self,
        service_id: &ServiceId,
        instance_id: &str,
    ) -> InfraResult<()>;

    async fn get_instances(&self, service_id: &ServiceId) -> InfraResult<Vec<ServiceInstance>>;

    async fn get_healthy_instances(
        &self,
        service_id: &ServiceId,
    ) -> InfraResult<Vec<ServiceInstance>>;

    async fn get_service(&self, service_id: &ServiceId) -> InfraResult<ServiceDefinition>;

    async fn update_health_status(
        &self,
        result: HealthCheckResult,
    ) -> InfraResult<()>;

    async fn snapshot(&self) -> InfraResult<RegistrySnapshot>;

    async fn list_services(&self) -> InfraResult<Vec<ServiceDefinition>>;
}

#[async_trait]
pub trait HealthChecker: Send + Sync {
    async fn check_http(
        &self,
        host: &str,
        port: u16,
        path: &str,
        timeout_secs: u64,
    ) -> InfraResult<HealthCheckResult>;

    async fn check_tcp(
        &self,
        host: &str,
        port: u16,
        timeout_secs: u64,
    ) -> InfraResult<HealthCheckResult>;
}

#[async_trait]
pub trait LoadBalancer: Send + Sync {
    async fn select_instance(&self, service_id: &ServiceId) -> InfraResult<ServiceInstance>;

    async fn select_instances(
        &self,
        service_id: &ServiceId,
        count: usize,
    ) -> InfraResult<Vec<ServiceInstance>>;

    async fn get_policy(&self, service_id: &ServiceId) -> InfraResult<LoadBalancerPolicy>;

    async fn update_policy(
        &self,
        service_id: &ServiceId,
        policy: LoadBalancerPolicy,
    ) -> InfraResult<()>;
}

#[async_trait]
pub trait Metrics: Send + Sync {
    async fn record_request(
        &self,
        service_id: &ServiceId,
        duration_ms: u64,
        success: bool,
    ) -> InfraResult<()>;

    async fn record_custom_metric(
        &self,
        name: &str,
        value: f64,
        tags: HashMap<String, String>,
    ) -> InfraResult<()>;

    async fn get_service_metrics(
        &self,
        service_id: &ServiceId,
        time_window_secs: u64,
    ) -> InfraResult<ServiceMetrics>;
}

#[derive(Clone, Debug)]
pub struct ServiceMetrics {
    pub service_id: ServiceId,
    pub request_count: u64,
    pub error_count: u64,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub p95_latency_ms: f64,
}

#[async_trait]
pub trait ServiceMesh: Send + Sync {
    async fn add_route(
        &self,
        source: &ServiceId,
        destination: &ServiceId,
        weight: u32,
    ) -> InfraResult<()>;

    async fn remove_route(
        &self,
        source: &ServiceId,
        destination: &ServiceId,
    ) -> InfraResult<()>;

    async fn get_routes(&self, service_id: &ServiceId) -> InfraResult<Vec<MeshRoute>>;

    async fn enable_circuit_breaker(
        &self,
        service_id: &ServiceId,
        config: CircuitBreakerConfig,
    ) -> InfraResult<()>;

    async fn get_circuit_breaker_status(
        &self,
        service_id: &ServiceId,
    ) -> InfraResult<CircuitBreakerStatus>;
}

#[derive(Clone, Debug)]
pub struct MeshRoute {
    pub source: ServiceId,
    pub destination: ServiceId,
    pub weight: u32,
}

#[derive(Clone, Debug)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout_secs: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CircuitBreakerStatus {
    Closed,
    Open,
    HalfOpen,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_metrics_creation() {
        let metrics = ServiceMetrics {
            service_id: ServiceId("svc".to_string()),
            request_count: 1000,
            error_count: 5,
            success_rate: 0.995,
            avg_latency_ms: 45.5,
            p99_latency_ms: 250.0,
            p95_latency_ms: 150.0,
        };

        assert_eq!(metrics.request_count, 1000);
        assert_eq!(metrics.error_count, 5);
    }

    #[test]
    fn test_mesh_route_creation() {
        let route = MeshRoute {
            source: ServiceId("api".to_string()),
            destination: ServiceId("db".to_string()),
            weight: 100,
        };

        assert_eq!(route.weight, 100);
    }

    #[test]
    fn test_circuit_breaker_status() {
        assert_ne!(CircuitBreakerStatus::Closed, CircuitBreakerStatus::Open);
        assert_eq!(CircuitBreakerStatus::Closed, CircuitBreakerStatus::Closed);
    }
}
