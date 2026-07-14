//! CLI demo: publish an event and read it back from the bus.

use chrono::Utc;
use event_system::{Event, EventBus};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bus = EventBus::new();

    bus.publish(&Event {
        event_id: "e1".to_string(),
        event_type: "user.created".to_string(),
        source: "auth-service".to_string(),
        timestamp: Utc::now(),
        payload: "{\"user_id\":\"u1\"}".to_string(),
    })
    .await?;

    let events = bus.get_events("user.created").await?;
    println!("Events for user.created: {}", events.len());

    Ok(())
}
