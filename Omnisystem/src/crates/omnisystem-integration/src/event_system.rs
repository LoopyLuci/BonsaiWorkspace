/// Event System
/// Cross-system event notification and handling

use dashmap::DashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

/// System Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub event_type: String,
    pub source_system: String,
    pub payload: serde_json::Value,
    pub timestamp: u64,
}

impl Event {
    pub fn new(event_type: String, source: String, payload: serde_json::Value) -> Self {
        Event {
            id: uuid::Uuid::new_v4().to_string(),
            event_type,
            source_system: source,
            payload,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

/// Event Handler Callback
pub type EventHandler = Arc<dyn Fn(Event) + Send + Sync>;

/// Event Subscription
#[derive(Clone)]
pub struct EventSubscription {
    id: String,
    event_type: String,
    handler: EventHandler,
}

/// Event System - Manages event distribution
pub struct EventSystem {
    subscriptions: Arc<DashMap<String, Vec<EventHandler>>>,
    event_history: Arc<DashMap<u64, Event>>,
    counter: Arc<std::sync::atomic::AtomicU64>,
}

impl EventSystem {
    pub fn new() -> Self {
        EventSystem {
            subscriptions: Arc::new(DashMap::new()),
            event_history: Arc::new(DashMap::new()),
            counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub fn subscribe(&self, event_type: String, handler: EventHandler) -> String {
        let subscription_id = uuid::Uuid::new_v4().to_string();

        self.subscriptions
            .entry(event_type)
            .or_insert_with(Vec::new)
            .push(handler);

        subscription_id
    }

    pub async fn emit(&self, event: Event) -> anyhow::Result<()> {
        // Store event in history
        let seq = self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.event_history.insert(seq, event.clone());

        // Dispatch to subscribers
        if let Some(handlers) = self.subscriptions.get(&event.event_type) {
            for handler in handlers.iter() {
                handler(event.clone());
            }
        }

        Ok(())
    }

    pub fn get_event_history(&self, limit: usize) -> Vec<Event> {
        let mut events: Vec<_> = self
            .event_history
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        events.sort_by_key(|e| e.timestamp);
        events.into_iter().rev().take(limit).collect()
    }

    pub fn subscriber_count(&self, event_type: &str) -> usize {
        self.subscriptions
            .get(event_type)
            .map(|subs| subs.len())
            .unwrap_or(0)
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        self.subscriptions.clear();
        self.event_history.clear();
        Ok(())
    }
}

impl Default for EventSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[tokio::test]
    async fn test_event_subscription() {
        let event_system = EventSystem::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = Arc::clone(&called);

        let handler = Arc::new(move |_event: Event| {
            called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        event_system.subscribe("test_event".to_string(), handler);

        let event = Event::new(
            "test_event".to_string(),
            "sys_1".to_string(),
            serde_json::json!({"data": "test"}),
        );

        event_system.emit(event).await.unwrap();
        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_event_history() {
        let event_system = EventSystem::new();

        for i in 0..5 {
            let event = Event::new(
                "test".to_string(),
                "sys_1".to_string(),
                serde_json::json!({"index": i}),
            );
            event_system.emit(event).await.unwrap();
        }

        let history = event_system.get_event_history(10);
        assert_eq!(history.len(), 5);
    }
}
