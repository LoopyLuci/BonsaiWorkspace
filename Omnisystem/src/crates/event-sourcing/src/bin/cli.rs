//! CLI

use event_sourcing::EventSourcingEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = EventSourcingEngine::new();
    let aggregate_id = uuid::Uuid::new_v4();

    let event = engine.append_event(aggregate_id, "Created", b"data").await?;
    println!("Appended event: {} ({})", event.event_id, event.event_type);

    println!("Total events: {}", engine.event_count());
    Ok(())
}
