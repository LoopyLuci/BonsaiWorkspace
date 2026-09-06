//! CLI for resource-monitor.
//!
//! This crate is currently a thin scaffold: its only real logic is the
//! init() bootstrap and the shared Metadata type. This CLI exercises both
//! honestly rather than pretending a richer API exists.

use resource_monitor::Metadata;

#[tokio::main]
async fn main() -> resource_monitor::Result<()> {
    resource_monitor::init().await?;
    println!("resource-monitor initialized");

    let meta = Metadata::new();
    println!("metadata id: {}, version: {}, created_at: {}", meta.id, meta.version, meta.created_at);

    Ok(())
}
