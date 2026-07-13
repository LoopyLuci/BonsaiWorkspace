use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Event {
    pub id: String,
    pub event_type: String,
    pub data: String,
}

pub struct Observer {
    events: Arc<DashMap<String, Event>>,
}

impl Observer {
    pub fn new() -> Self {
        Self { events: Arc::new(DashMap::new()) }
    }
    
    pub fn observe(&self, event_type: String, data: String) -> String {
        let id = format!("evt_{}", self.events.len());
        let event = Event { id: id.clone(), event_type, data };
        self.events.insert(id.clone(), event);
        id
    }
    
    pub fn get_event(&self, id: &str) -> Option<Event> {
        self.events.get(id).map(|e| e.clone())
    }
    
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_observe() {
        let observer = Observer::new();
        let evt_id = observer.observe("startup".to_string(), "system_started".to_string());
        assert!(!evt_id.is_empty());
    }
    
    #[test]
    fn test_get_event() {
        let observer = Observer::new();
        let evt_id = observer.observe("startup".to_string(), "system_started".to_string());
        assert!(observer.get_event(&evt_id).is_some());
    }
}
