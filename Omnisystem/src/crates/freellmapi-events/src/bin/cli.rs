//! Demo CLI: registers a webhook for a tenant, logs an event, and reads it back
//! through the real EventService API.

use freellmapi_events::{EventRecord, EventService, EventType};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = EventService::new().await?;
    service
        .register_webhook("demo-tenant", "https://example.com/webhook")
        .await?;

    let record = EventRecord {
        id: uuid::Uuid::new_v4().to_string(),
        event_type: EventType::RequestCompleted,
        tenant_id: "demo-tenant".to_string(),
        data: serde_json::json!({"model": "gpt-4"}),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };
    service.log_event(&record).await?;

    let events = service.get_events("demo-tenant", 10).await?;
    println!("Logged {} event(s) for demo-tenant", events.len());

    let webhooks = service.get_webhooks("demo-tenant").await?;
    println!("Registered webhooks: {:?}", webhooks);

    Ok(())
}
