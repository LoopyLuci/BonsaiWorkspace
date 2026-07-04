use async_trait::async_trait;
use crate::{
    CircuitBreaker, EndpointId, HealthCheckResult, MeshResult, RequestMetrics, ServiceEndpoint,
    ServiceId, ServiceInstance,
};

#[async_trait]
pub trait ServiceRegistry: Send + Sync {
    async fn register_service(&self, service: &ServiceInstance) -> MeshResult<()>;

    async fn deregister_service(&self, service_id: &ServiceId) -> MeshResult<()>;

    async fn get_service(&self, service_id: &ServiceId) -> MeshResult<ServiceInstance>;

    async fn list_services(&self) -> MeshResult<Vec<ServiceInstance>>;

    async fn update_endpoint_status(
        &self,
        service_id: &ServiceId,
        endpoint_id: &EndpointId,
        healthy: bool,
    ) -> MeshResult<()>;
}

#[async_trait]
pub trait LoadBalancer: Send + Sync {
    async fn select_endpoint(
        &self,
        service_id: &ServiceId,
    ) -> MeshResult<ServiceEndpoint>;

    async fn update_metrics(&self, service_id: &ServiceId, metrics: &RequestMetrics) -> MeshResult<()>;

    async fn get_metrics(&self, service_id: &ServiceId) -> MeshResult<RequestMetrics>;
}

#[async_trait]
pub trait CircuitBreakerManager: Send + Sync {
    async fn check_circuit(&self, service_id: &ServiceId) -> MeshResult<bool>;

    async fn record_success(&self, service_id: &ServiceId) -> MeshResult<()>;

    async fn record_failure(&self, service_id: &ServiceId) -> MeshResult<()>;

    async fn get_circuit_state(&self, service_id: &ServiceId) -> MeshResult<CircuitBreaker>;

    async fn reset_circuit(&self, service_id: &ServiceId) -> MeshResult<()>;
}

#[async_trait]
pub trait HealthChecker: Send + Sync {
    async fn check_health(&self, endpoint: &ServiceEndpoint) -> MeshResult<HealthCheckResult>;

    async fn check_service_health(&self, service_id: &ServiceId) -> MeshResult<Vec<HealthCheckResult>>;

    async fn start_periodic_checks(&self, service_id: &ServiceId, interval_secs: u64) -> MeshResult<()>;

    async fn stop_periodic_checks(&self, service_id: &ServiceId) -> MeshResult<()>;
}

#[async_trait]
pub trait RateLimiter: Send + Sync {
    async fn check_rate_limit(&self, service_id: &ServiceId) -> MeshResult<bool>;

    async fn acquire_token(&self, service_id: &ServiceId, count: u32) -> MeshResult<bool>;

    async fn reset_limits(&self, service_id: &ServiceId) -> MeshResult<()>;
}
