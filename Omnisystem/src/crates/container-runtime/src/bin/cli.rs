//! CLI demo: create an image, spin up a container from it, and start it.

use container_runtime::{ContainerConfig, ContainerRuntime, ImageManager};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let images = ImageManager::new();
    let runtime = ContainerRuntime::new();

    let image = images.create_image("nginx".to_string(), "latest".to_string()).await?;
    println!("Created image: {}", image.id.0);

    let container_id = runtime
        .create_container(
            &image.id,
            &ContainerConfig {
                image: "nginx:latest".to_string(),
                cmd: vec!["nginx".to_string(), "-g".to_string(), "daemon off;".to_string()],
                env: HashMap::new(),
                working_dir: "/".to_string(),
                ports: vec![],
                volumes: vec![],
                cpu_limit_millicores: 500,
                memory_limit_bytes: 256 * 1024 * 1024,
            },
        )
        .await?;
    println!("Created container: {}", container_id.0);

    runtime.start_container(&container_id).await?;
    let container = runtime.get_container(&container_id).await?;
    println!("Container state: {:?}", container.state);

    Ok(())
}
