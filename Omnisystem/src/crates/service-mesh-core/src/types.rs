use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeshService {
    pub service_id: Uuid,
    pub name: String,
    pub namespace: String,
    pub port: u16,
    pub protocol: Protocol,
    pub version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Protocol {
    HTTP,
    HTTPS,
    GRPC,
    TCP,
    UDP,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SidecarProxy {
    pub proxy_id: Uuid,
    pub service_id: Uuid,
    pub pod_ip: String,
    pub proxy_port: u16,
    pub admin_port: u16,
    pub status: ProxyStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum ProxyStatus {
    Initializing,
    Ready,
    Healthy,
    Unhealthy,
    Terminating,
}

/// A raw registered mesh endpoint (used by [`crate::mesh_manager::MeshManager`]).
///
/// Distinct from [`ServiceEndpoint`], which is the health/weight-tracked
/// endpoint shape used by the service registry and load balancer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeshEndpoint {
    pub endpoint_id: Uuid,
    pub service_id: Uuid,
    pub address: String,
    pub port: u16,
    pub weight: u32,
    pub healthy: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeshConfig {
    pub config_id: Uuid,
    pub name: String,
    pub mtls_enabled: bool,
    pub tracing_enabled: bool,
    pub service_registry_url: String,
}

/// Identifier for a registered service
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct ServiceId(pub String);

/// Identifier for a single service endpoint
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct EndpointId(pub String);

/// Health status of a service or endpoint
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// A health/weight-tracked endpoint, as used by the service registry and load balancer
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    pub id: EndpointId,
    pub address: String,
    pub port: u16,
    pub weight: u32,
    pub status: ServiceStatus,
    pub last_checked: DateTime<Utc>,
    pub failure_count: u32,
    pub success_count: u32,
}

/// A registered service instance with its known endpoints
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceInstance {
    pub service_id: ServiceId,
    pub endpoints: Vec<ServiceEndpoint>,
    pub status: ServiceStatus,
    pub metadata: HashMap<String, String>,
    pub updated_at: DateTime<Utc>,
}

/// Aggregated request metrics for a service, used by the load balancer
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RequestMetrics {
    pub request_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub total_latency_ms: u64,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
    pub p50_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
}

/// Token-bucket rate limiter configuration
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_second: u32,
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 100,
            burst_size: 20,
        }
    }
}

/// Result of a single health check
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub endpoint_id: EndpointId,
    pub healthy: bool,
    pub latency_ms: u64,
    pub checked_at: DateTime<Utc>,
}

/// Circuit breaker state machine states
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker thresholds
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout_secs: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_secs: 30,
        }
    }
}

/// Per-service circuit breaker state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitBreaker {
    pub service_id: ServiceId,
    pub state: CircuitBreakerState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_state_change: DateTime<Utc>,
    pub opened_at: Option<DateTime<Utc>>,
    pub config: CircuitBreakerConfig,
}
