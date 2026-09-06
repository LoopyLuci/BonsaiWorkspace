//! CLI for container-performance-profiler.
//!
//! This crate is currently a thin scaffold: its only real logic is the
//! init() bootstrap and the shared Metadata type. This CLI exercises both
//! honestly rather than pretending a richer API exists.

use container_performance_profiler::Metadata;

#[tokio::main]
async fn main() -> container_performance_profiler::Result<()> {
    container_performance_profiler::init().await?;
    println!("container-performance-profiler initialized");

    let meta = Metadata::new();
    println!("metadata id: {}, version: {}, created_at: {}", meta.id, meta.version, meta.created_at);

    Ok(())
}
