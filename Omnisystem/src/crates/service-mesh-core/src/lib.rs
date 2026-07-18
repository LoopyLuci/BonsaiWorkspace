//! Service Mesh Core
//!
//! Building blocks for a service mesh control plane: service discovery
//! ([`mesh_manager`], [`service_registry`]), client-side load balancing
//! ([`load_balancer`]), resiliency ([`circuit_breaker`]), and traffic
//! shaping ([`rate_limiter`]). Abstract contracts for each of these live in
//! [`traits`] so alternative implementations can be swapped in.

pub mod circuit_breaker;
pub mod error;
pub mod load_balancer;
pub mod mesh_manager;
pub mod rate_limiter;
pub mod service_registry;
pub mod traits;
pub mod types;

pub use circuit_breaker::CircuitBreakerManager;
pub use error::{MeshError, MeshResult};
pub use load_balancer::LoadBalancer;
pub use mesh_manager::MeshManager;
pub use rate_limiter::RateLimiter;
pub use service_registry::ServiceRegistry;
pub use types::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState, EndpointId, HealthCheckResult,
    MeshConfig, MeshEndpoint, MeshService, ProxyStatus, Protocol, RateLimitConfig,
    RequestMetrics, ServiceEndpoint, ServiceId, ServiceInstance, ServiceStatus, SidecarProxy,
};
