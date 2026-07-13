use async_trait::async_trait;
use crate::{Event, Result};

#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: Event) -> Result<()>;
}

pub struct SimpleHandler;

#[async_trait]
impl EventHandler for SimpleHandler {
    async fn handle(&self, event: Event) -> Result<()> {
        tracing::info!("Handling event: {}", event.event_type);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handler() {
        let handler = SimpleHandler;
        let event = Event::new("test".to_string(), serde_json::json!({}));
        assert!(handler.handle(event).await.is_ok());
    }
}
