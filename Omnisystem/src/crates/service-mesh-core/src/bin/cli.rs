//! Service Mesh Core CLI - exercises registry, load balancing, circuit breaking, and rate limiting

use chrono::Utc;
use service_mesh_core::{
    CircuitBreakerManager, EndpointId, LoadBalancer, RateLimitConfig, RateLimiter, ServiceEndpoint,
    ServiceId, ServiceInstance, ServiceStatus,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = service_mesh_core::ServiceRegistry::new();
    let service_id = ServiceId("api-service".to_string());

    let service = ServiceInstance {
        service_id: service_id.clone(),
        endpoints: vec![ServiceEndpoint {
            id: EndpointId("ep-1".to_string()),
            address: "10.0.0.1".to_string(),
            port: 8080,
            weight: 100,
            status: ServiceStatus::Healthy,
            last_checked: Utc::now(),
            failure_count: 0,
            success_count: 0,
        }],
        status: ServiceStatus::Healthy,
        metadata: HashMap::new(),
        updated_at: Utc::now(),
    };

    registry.register_service(&service).await?;
    println!("registered services: {}", registry.service_count());

    let endpoints = registry.get_healthy_endpoints(&service_id).await?;
    let lb = LoadBalancer::new();
    let endpoint = lb.select_round_robin(&endpoints)?;
    println!("selected endpoint: {}:{}", endpoint.address, endpoint.port);

    let breakers = CircuitBreakerManager::new();
    breakers.check_circuit(&service_id).await?;
    println!("circuit breakers tracked: {}", breakers.breaker_count());

    let limiter = RateLimiter::new(RateLimitConfig::default());
    let allowed = limiter.check_rate_limit(&service_id).await?;
    println!("request allowed: {}", allowed);

    Ok(())
}
