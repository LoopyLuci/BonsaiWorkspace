//! CLI

use chrono::Utc;
use infrastructure_core::{HealthStatus, LoadBalancerPolicy, ServiceDefinition, ServiceId, ServiceInstance};
use infrastructure_loadbalancer::DefaultLoadBalancer;
use infrastructure_registry::InMemoryRegistry;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = Arc::new(InMemoryRegistry::new());
    let svc_id = ServiceId("api-service".to_string());

    let def = ServiceDefinition {
        id: svc_id.clone(),
        name: "API Service".to_string(),
        protocol: "http".to_string(),
        port: 8080,
        tags: vec![],
        health_check: Default::default(),
        load_balancer_policy: LoadBalancerPolicy::RoundRobin,
        created_at: Utc::now(),
    };
    registry.register_service(def).await?;

    let mut instance = ServiceInstance::new(svc_id.clone(), "api-0".to_string(), 8080);
    instance.health_status = HealthStatus::Healthy;
    registry.register_instance(instance).await?;

    let lb = DefaultLoadBalancer::new(registry.clone());
    let selected = lb.select_instance(&svc_id).await?;
    println!("Selected instance: {} on port {}", selected.id, selected.port);

    Ok(())
}
