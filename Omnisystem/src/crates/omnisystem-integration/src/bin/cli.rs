//! CLI demo for omnisystem-integration: registers a module, a service,
//! publishes an event, and runs a health check.

use omnisystem_integration::{EventBus, HealthCheck, ModuleOrchestrator, ServiceRegistry};
use omnisystem_integration::event_bus::Event;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orch = ModuleOrchestrator::new();
    orch.register_module("search".to_string()).await?;
    orch.start_module("search").await?;
    println!("Modules running: {}", orch.module_count());

    let registry = ServiceRegistry::new();
    registry.register("search".to_string(), "1.0.0".to_string())?;
    println!("Services: {:?}", registry.list_services());

    let bus = EventBus::new();
    bus.subscribe("module_started", "monitor".to_string());
    bus.publish(Event {
        event_id: "evt-1".to_string(),
        source_system: "search".to_string(),
        event_type: "module_started".to_string(),
        payload: "{}".to_string(),
    });
    println!(
        "Subscribers to module_started: {:?}",
        bus.get_subscribers("module_started")
    );

    let health = HealthCheck::check(orch.module_count(), orch.module_count());
    println!("Health: {} ({}/{})", health.status, health.modules_healthy, health.modules_total);

    Ok(())
}
