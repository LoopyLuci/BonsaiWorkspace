//! Service discovery CLI: registers a couple of demo instances, runs the
//! round-robin load balancer over them, and prints the resulting registry.

use service_discovery::{LoadBalancer, LoadBalancingPolicy, ServiceInstance, ServiceRegistryImpl, ServiceStatus};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = ServiceRegistryImpl::new();
    let balancer = LoadBalancer::new();

    let policy = LoadBalancingPolicy {
        policy_type: "round_robin".to_string(),
        weight_map: HashMap::new(),
    };
    balancer.register_policy("api", &policy).await?;

    for (id, host) in [("i1", "10.0.0.1"), ("i2", "10.0.0.2")] {
        let instance = ServiceInstance {
            instance_id: id.to_string(),
            service_name: "api".to_string(),
            host: host.to_string(),
            port: 8080,
            status: ServiceStatus::Healthy,
            registered_at: chrono::Utc::now(),
            last_heartbeat: chrono::Utc::now(),
            metadata: HashMap::new(),
        };
        registry.register(&instance).await?;
    }

    let instances = registry.get_instances("api").await?;
    let selected = balancer.select_instance("api", &instances).await?;
    println!("Registered {} instance(s) for 'api'", instances.len());
    println!("Load balancer selected: {} ({})", selected.instance_id, selected.host);

    let snapshot = registry.get_registry("api").await?;
    println!(
        "Registry snapshot: {}/{} healthy",
        snapshot.healthy_instances, snapshot.total_instances
    );

    Ok(())
}
