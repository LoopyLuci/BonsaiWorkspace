//! CLI for omnidocker-state-manager.
//!
//! This crate is currently a thin scaffold: its only real logic is the
//! init() bootstrap and the shared Metadata type. This CLI exercises both
//! honestly rather than pretending a richer API exists.

use omnidocker_state_manager::Metadata;

#[tokio::main]
async fn main() -> omnidocker_state_manager::Result<()> {
    omnidocker_state_manager::init().await?;
    println!("omnidocker-state-manager initialized");

    let meta = Metadata::new();
    println!("metadata id: {}, version: {}, created_at: {}", meta.id, meta.version, meta.created_at);

    Ok(())
}
