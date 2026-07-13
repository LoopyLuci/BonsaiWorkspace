//! CLI

use infrastructure_core::ServiceInstance;

fn main() {
    let instance = ServiceInstance::new(
        infrastructure_core::ServiceId("example-service".to_string()),
        "localhost".to_string(),
        8080,
    );
    println!("Service instance: {} ({})", instance.service_id.0, instance.address());
    println!("Healthy: {}", instance.is_healthy());
}
