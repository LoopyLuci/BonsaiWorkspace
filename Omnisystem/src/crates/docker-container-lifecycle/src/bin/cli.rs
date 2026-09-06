//! CLI for docker-container-lifecycle.
//!
//! This crate is currently a thin scaffold: its only real logic is the
//! init() bootstrap and the shared Metadata type. This CLI exercises both
//! honestly rather than pretending a richer API exists.

use docker_container_lifecycle::Metadata;

#[tokio::main]
async fn main() -> docker_container_lifecycle::Result<()> {
    docker_container_lifecycle::init().await?;
    println!("docker-container-lifecycle initialized");

    let meta = Metadata::new();
    println!("metadata id: {}, version: {}, created_at: {}", meta.id, meta.version, meta.created_at);

    Ok(())
}
