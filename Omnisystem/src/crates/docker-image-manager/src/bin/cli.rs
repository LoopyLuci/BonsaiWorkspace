//! CLI for docker-image-manager.
//!
//! This crate is currently a thin scaffold: its only real logic is the
//! init() bootstrap and the shared Metadata type. This CLI exercises both
//! honestly rather than pretending a richer API exists.

use docker_image_manager::Metadata;

#[tokio::main]
async fn main() -> docker_image_manager::Result<()> {
    docker_image_manager::init().await?;
    println!("docker-image-manager initialized");

    let meta = Metadata::new();
    println!("metadata id: {}, version: {}, created_at: {}", meta.id, meta.version, meta.created_at);

    Ok(())
}
