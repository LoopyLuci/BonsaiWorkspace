//! CLI for omnisystem-event-bus-integration — exercises the crate's real Enterprise processing API.

use omnisystem_event_bus_integration::Enterprise;

#[tokio::main]
async fn main() -> omnisystem_event_bus_integration::Result<()> {
    let module = Enterprise::new();
    let input = std::env::args().nth(1).unwrap_or_else(|| "sample payload".to_string());

    let processed = module.process(&input).await?;
    println!("processed: {processed}");

    Ok(())
}
