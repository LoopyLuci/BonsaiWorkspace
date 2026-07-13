use crate::{Event, Result};
use dashmap::DashMap;
use std::sync::Arc;

pub struct EventBus {
    subscribers: Arc<DashMap<String, Vec<Box<dyn Fn(Event) + Send + Sync>>>>,
    event_log: Arc<std::sync::Mutex<Vec<Event>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(DashMap::new()),
            event_log: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub async fn publish(&self, event: Event) -> Result<()> {
        let mut log = self.event_log.lock().unwrap();
        log.push(event.clone());
        
        if let Some(handlers) = self.subscribers.get(&event.event_type) {
            for handler in handlers.value().iter() {
                handler(event.clone());
            }
        }
        
        tracing::info!("Event published: {}", event.event_type);
        Ok(())
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }

    pub fn event_count(&self) -> usize {
        self.event_log.lock().unwrap().len()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus() {
        let bus = EventBus::new();
        let event = Event::new("test".to_string(), serde_json::json!({}));
        assert!(bus.publish(event).await.is_ok());
        assert_eq!(bus.event_count(), 1);
    }
}
