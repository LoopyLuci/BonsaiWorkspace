use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use dashmap::DashMap;
use anyhow::Result;
use async_trait::async_trait;
use crate::EventType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub id: String,
    pub event_type: EventType,
    pub tenant_id: String,
    pub data: serde_json::Value,
    pub timestamp: u64,
}

#[async_trait]
pub trait EventLog: Send + Sync {
    async fn append(&self, record: &EventRecord) -> Result<()>;
    async fn query(&self, tenant_id: &str, limit: usize) -> Result<Vec<EventRecord>>;
    async fn query_by_type(&self, tenant_id: &str, event_type: EventType, limit: usize) -> Result<Vec<EventRecord>>;
}

pub struct InMemoryEventLog {
    events: Arc<DashMap<String, Vec<EventRecord>>>,
}

impl InMemoryEventLog {
    pub fn new() -> Self {
        InMemoryEventLog {
            events: Arc::new(DashMap::new()),
        }
    }
}

#[async_trait]
impl EventLog for InMemoryEventLog {
    async fn append(&self, record: &EventRecord) -> Result<()> {
        let mut entry = self.events
            .entry(record.tenant_id.clone())
            .or_insert_with(Vec::new);
        entry.push(record.clone());
        Ok(())
    }

    async fn query(&self, tenant_id: &str, limit: usize) -> Result<Vec<EventRecord>> {
        if let Some(entry) = self.events.get(tenant_id) {
            let events = entry.value().clone();
            let start = if events.len() > limit { events.len() - limit } else { 0 };
            Ok(events[start..].to_vec())
        } else {
            Ok(vec![])
        }
    }

    async fn query_by_type(&self, tenant_id: &str, event_type: EventType, limit: usize) -> Result<Vec<EventRecord>> {
        if let Some(entry) = self.events.get(tenant_id) {
            let filtered: Vec<_> = entry.value()
                .iter()
                .filter(|e| e.event_type == event_type)
                .take(limit)
                .cloned()
                .collect();
            Ok(filtered)
        } else {
            Ok(vec![])
        }
    }
}

impl Default for InMemoryEventLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_append_event() {
        let log = InMemoryEventLog::new();
        let record = EventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: EventType::RequestStarted,
            tenant_id: "test-tenant".to_string(),
            data: json!({"model": "gpt-4"}),
            timestamp: 1000,
        };

        let result = log.append(&record).await;
        assert!(result.is_ok());

        let events = log.query("test-tenant", 10).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EventType::RequestStarted);
    }

    #[tokio::test]
    async fn test_query_by_type() {
        let log = InMemoryEventLog::new();

        log.append(&EventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: EventType::RequestStarted,
            tenant_id: "test-tenant".to_string(),
            data: json!({}),
            timestamp: 1000,
        }).await.unwrap();

        log.append(&EventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: EventType::RequestCompleted,
            tenant_id: "test-tenant".to_string(),
            data: json!({}),
            timestamp: 2000,
        }).await.unwrap();

        let completed = log.query_by_type("test-tenant", EventType::RequestCompleted, 10).await.unwrap();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].event_type, EventType::RequestCompleted);
    }
}
