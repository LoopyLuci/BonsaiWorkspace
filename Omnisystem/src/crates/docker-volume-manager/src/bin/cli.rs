//! CLI demo: initialize the docker-volume-manager module.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    docker_volume_manager::init().await?;
    println!("docker-volume-manager initialized");
    Ok(())
}
