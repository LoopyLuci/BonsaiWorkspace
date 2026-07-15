//! Bonsai Container Fabric CLI: parses a Blueprint from YAML, validates and
//! stores it, then emits and observes real lifecycle events over the
//! broadcast EventBus.

use chrono::Utc;
use container::{BcfConfig, BlueprintManager, Event, EventBus};

const BLUEPRINT_YAML: &str = r#"
name: web-app
version: "1.0.0"
containers:
  - id: web-1
    name: web
    image: bonsai/web:1.0.0
    replicas: 3
    resources:
      cpu_cores: 0.5
      cpu_priority: Normal
      memory_mib: 256
      memory_swap_mib: null
      gpu: null
    storage:
      volumes: []
    network:
      ports:
        - container_port: 8080
          service_port: 80
          protocol: tcp
      policy: default
      tls_enabled: false
    capability_tokens: []
    overlay_size_mib: null
    deadline: null
    period: null
    probes:
      liveness: null
      readiness: null
      startup: null
    env_vars: {}
    update_strategy: !Rolling
      max_surge: 1
      max_unavailable: 0
services:
  - name: web-svc
    selector: {}
    ports:
      - container_port: 8080
        service_port: 80
        protocol: tcp
    load_balancing: RoundRobin
    session_affinity: false
volumes: []
metadata: {}
"#;

#[tokio::main]
async fn main() -> container::Result<()> {
    let config = BcfConfig::default();
    println!("node_id={} namespace={} max_containers_per_node={}", config.node_id, config.namespace, config.max_containers_per_node);

    let blueprint = BlueprintManager::from_yaml(BLUEPRINT_YAML)?;
    println!(
        "parsed blueprint '{}' v{} with {} container(s), {} service(s)",
        blueprint.name,
        blueprint.version,
        blueprint.containers.len(),
        blueprint.services.len()
    );

    let manager = BlueprintManager::new();
    manager.store(blueprint.clone())?;
    println!("stored blueprints: {:?}", manager.list());

    let bus = EventBus::new();
    let mut receiver = bus.subscribe();

    bus.emit(Event::DeploymentStarted {
        deployment_id: "deploy-1".to_string(),
        timestamp: Utc::now(),
    })
    .await?;
    bus.emit(Event::ContainerStarted {
        container_id: "web-1-0".to_string(),
        image_hash: "sha256:deadbeef".to_string(),
        node_id: config.node_id.clone(),
        timestamp: Utc::now(),
    })
    .await?;
    bus.emit(Event::DeploymentSucceeded {
        deployment_id: "deploy-1".to_string(),
        timestamp: Utc::now(),
    })
    .await?;

    println!("\nevents observed:");
    for _ in 0..3 {
        let event = receiver.recv().await.expect("event bus closed unexpectedly");
        println!("  {:?}", event);
    }

    // Validation is real: an empty blueprint name is rejected.
    let mut invalid = blueprint;
    invalid.name = String::new();
    match invalid.validate() {
        Err(e) => println!("\ninvalid blueprint correctly rejected: {e}"),
        Ok(()) => println!("\nunexpected: invalid blueprint passed validation"),
    }

    Ok(())
}
