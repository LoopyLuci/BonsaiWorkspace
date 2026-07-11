//! CLI

use orchestration_engine::{PodManager, PodSpec};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = PodManager::new();
    let spec = PodSpec {
        name: "example-pod".to_string(),
        image: "example:latest".to_string(),
        replicas: 1,
        labels: HashMap::new(),
        cpu_request_millicores: 100,
        memory_request_bytes: 128 * 1024 * 1024,
        cpu_limit_millicores: 500,
        memory_limit_bytes: 512 * 1024 * 1024,
        ports: vec![],
    };

    let pod_id = manager.create_pod(&spec).await?;
    println!("Created pod: {}", pod_id.0);
    println!("Total pods: {}", manager.pod_count());

    Ok(())
}
