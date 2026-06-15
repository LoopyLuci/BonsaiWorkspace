use crate::{EventSchema, Event, Publisher, Subscriber, StreamingError, StreamingResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct EventStreamer {
    schemas: Arc<DashMap<Uuid, EventSchema>>,
    events: Arc<DashMap<Uuid, Event>>,
    publishers: Arc<DashMap<Uuid, Publisher>>,
    subscribers: Arc<DashMap<Uuid, Subscriber>>,
}

impl EventStreamer {
    pub fn new() -> Self {
        Self {
            schemas: Arc::new(DashMap::new()),
            events: Arc::new(DashMap::new()),
            publishers: Arc::new(DashMap::new()),
            subscribers: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_schema(&self, schema: &EventSchema) -> StreamingResult<()> {
        self.schemas.insert(schema.schema_id, schema.clone());
        Ok(())
    }

    pub async fn get_schema(&self, schema_id: Uuid) -> StreamingResult<EventSchema> {
        self.schemas
            .get(&schema_id)
            .map(|s| s.clone())
            .ok_or(StreamingError::SchemaNotFound)
    }

    pub async fn register_publisher(&self, publisher: &Publisher) -> StreamingResult<()> {
        self.publishers.insert(publisher.publisher_id, publisher.clone());
        Ok(())
    }

    pub async fn register_subscriber(&self, subscriber: &Subscriber) -> StreamingResult<()> {
        self.subscribers.insert(subscriber.subscriber_id, subscriber.clone());
        Ok(())
    }

    pub async fn publish_event(&self, event: &Event) -> StreamingResult<()> {
        if !self.schemas.contains_key(&event.schema_id) {
            return Err(StreamingError::SchemaNotFound);
        }

        self.events.insert(event.event_id, event.clone());
        Ok(())
    }

    pub async fn get_event(&self, event_id: Uuid) -> StreamingResult<Event> {
        self.events
            .get(&event_id)
            .map(|e| e.clone())
            .ok_or(StreamingError::PublishFailed)
    }

    pub fn schema_count(&self) -> usize {
        self.schemas.len()
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

impl Default for EventStreamer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_schema() {
        let streamer = EventStreamer::new();
        let schema = EventSchema {
            schema_id: Uuid::new_v4(),
            name: "user_created".to_string(),
            version: 1,
            fields: vec![("user_id".to_string(), "uuid".to_string())],
        };

        streamer.register_schema(&schema).await.unwrap();
        assert_eq!(streamer.schema_count(), 1);
    }

    #[tokio::test]
    async fn test_publish_event() {
        let streamer = EventStreamer::new();
        let schema_id = Uuid::new_v4();
        let schema = EventSchema {
            schema_id,
            name: "order_placed".to_string(),
            version: 1,
            fields: vec![("order_id".to_string(), "uuid".to_string())],
        };

        streamer.register_schema(&schema).await.unwrap();

        let event = Event {
            event_id: Uuid::new_v4(),
            schema_id,
            event_type: "order_placed".to_string(),
            payload: b"data".to_vec(),
        };

        streamer.publish_event(&event).await.unwrap();
        assert_eq!(streamer.event_count(), 1);
    }

    #[tokio::test]
    async fn test_register_publisher() {
        let streamer = EventStreamer::new();
        let publisher = Publisher {
            publisher_id: Uuid::new_v4(),
            name: "order_service".to_string(),
            topics: vec!["orders".to_string()],
        };

        streamer.register_publisher(&publisher).await.unwrap();
    }

    #[tokio::test]
    async fn test_register_subscriber() {
        let streamer = EventStreamer::new();
        let subscriber = Subscriber {
            subscriber_id: Uuid::new_v4(),
            name: "analytics".to_string(),
            topics: vec!["orders".to_string()],
            handler_url: Some("http://analytics:8080".to_string()),
        };

        streamer.register_subscriber(&subscriber).await.unwrap();
    }
}
