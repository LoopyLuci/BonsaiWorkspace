//! CLI for docker-network-manager.
//!
//! This crate is currently a thin scaffold: its only real logic is the
//! init() bootstrap and the shared Metadata type. This CLI exercises both
//! honestly rather than pretending a richer API exists.

use docker_network_manager::Metadata;

#[tokio::main]
async fn main() -> docker_network_manager::Result<()> {
    docker_network_manager::init().await?;
    println!("docker-network-manager initialized");

    let meta = Metadata::new();
    println!("metadata id: {}, version: {}, created_at: {}", meta.id, meta.version, meta.created_at);

    Ok(())
}
