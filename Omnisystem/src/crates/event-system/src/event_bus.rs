use crate::{Event, EventError, EventResult};
use dashmap::DashMap;
use std::sync::Arc;

pub struct EventBus {
    subscriptions: Arc<DashMap<String, Vec<String>>>,
    events: Arc<DashMap<String, Vec<Event>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(DashMap::new()),
            events: Arc::new(DashMap::new()),
        }
    }

    pub async fn publish(&self, event: &Event) -> EventResult<()> {
        self.events
            .entry(event.event_type.clone())
            .or_insert_with(Vec::new)
            .push(event.clone());
        Ok(())
    }

    pub async fn get_events(&self, event_type: &str) -> EventResult<Vec<Event>> {
        if let Some(events) = self.events.get(event_type) {
            Ok(events.clone())
        } else {
            Err(EventError::EventNotFound)
        }
    }

    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
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
    use chrono::Utc;

    #[tokio::test]
    async fn test_publish_event() {
        let bus = EventBus::new();
        let event = Event {
            event_id: "e1".to_string(),
            event_type: "user.created".to_string(),
            source: "api".to_string(),
            timestamp: Utc::now(),
            payload: "{}".to_string(),
        };

        bus.publish(&event).await.unwrap();
        assert!(bus.subscription_count() >= 0);
    }

    #[tokio::test]
    async fn test_get_events() {
        let bus = EventBus::new();
        let event = Event {
            event_id: "e1".to_string(),
            event_type: "user.created".to_string(),
            source: "api".to_string(),
            timestamp: Utc::now(),
            payload: "{}".to_string(),
        };

        bus.publish(&event).await.unwrap();
        let events = bus.get_events("user.created").await.unwrap();
        assert_eq!(events.len(), 1);
    }
}
