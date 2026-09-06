//! CLI for logging-aggregator.
//!
//! This crate is currently a thin scaffold: its only real logic is the
//! init() bootstrap and the shared Metadata type. This CLI exercises both
//! honestly rather than pretending a richer API exists.

use logging_aggregator::Metadata;

#[tokio::main]
async fn main() -> logging_aggregator::Result<()> {
    logging_aggregator::init().await?;
    println!("logging-aggregator initialized");

    let meta = Metadata::new();
    println!("metadata id: {}, version: {}, created_at: {}", meta.id, meta.version, meta.created_at);

    Ok(())
}
