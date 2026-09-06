//! CLI for notification-dispatcher.
//!
//! This crate is currently a thin scaffold: its only real logic is the
//! init() bootstrap and the shared Metadata type. This CLI exercises both
//! honestly rather than pretending a richer API exists.

use notification_dispatcher::Metadata;

#[tokio::main]
async fn main() -> notification_dispatcher::Result<()> {
    notification_dispatcher::init().await?;
    println!("notification-dispatcher initialized");

    let meta = Metadata::new();
    println!("metadata id: {}, version: {}, created_at: {}", meta.id, meta.version, meta.created_at);

    Ok(())
}
