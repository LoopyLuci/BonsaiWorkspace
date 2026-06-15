use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Event {
    pub event_id: String,
    pub source_system: String,
    pub event_type: String,
    pub payload: String,
}

pub struct EventBus {
    subscribers: Arc<DashMap<String, Vec<String>>>,
    events: Arc<DashMap<String, Event>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(DashMap::new()),
            events: Arc::new(DashMap::new()),
        }
    }

    pub fn subscribe(&self, event_type: &str, subscriber: String) {
        let mut subs = self.subscribers.entry(event_type.to_string()).or_insert_with(Vec::new);
        subs.push(subscriber);
    }

    pub fn publish(&self, event: Event) -> String {
        let event_id = event.event_id.clone();
        self.events.insert(event_id.clone(), event);
        event_id
    }

    pub fn get_subscribers(&self, event_type: &str) -> Vec<String> {
        self.subscribers.get(event_type).map(|s| s.clone()).unwrap_or_default()
    }

    pub fn get_event(&self, event_id: &str) -> Option<Event> {
        self.events.get(event_id).map(|e| e.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_publish() {
        let eb = EventBus::new();
        let event = Event {
            event_id: "evt_1".to_string(),
            source_system: "buddy".to_string(),
            event_type: "file_updated".to_string(),
            payload: "{}".to_string(),
        };
        let evt_id = eb.publish(event);
        assert!(!evt_id.is_empty());
    }

    #[test]
    fn test_subscribe() {
        let eb = EventBus::new();
        eb.subscribe("search_query", "omni-bot".to_string());
        let subs = eb.get_subscribers("search_query");
        assert_eq!(subs.len(), 1);
    }
}
