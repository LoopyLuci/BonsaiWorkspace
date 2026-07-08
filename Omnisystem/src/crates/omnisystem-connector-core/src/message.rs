use crate::ConnectorId;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Message<T> {
    pub id: String,
    pub data: T,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> Message<T> {
    pub fn new(data: T) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            data,
            timestamp: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MessageEnvelope<T> {
    pub message: Message<T>,
    pub source: Option<ConnectorId>,
    pub metadata: HashMap<String, String>,
}

impl<T> MessageEnvelope<T> {
    pub fn new(data: T) -> Self {
        Self {
            message: Message::new(data),
            source: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_source(mut self, source: ConnectorId) -> Self {
        self.source = Some(source);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use crate::Connectable;

    #[derive(Clone, Serialize, Deserialize)]
    struct TestMsg(String);

    impl Connectable for TestMsg {
        fn type_id() -> u128 {
            123
        }
        fn schema() -> crate::connector::Schema {
            crate::connector::Schema {
                type_id: 123,
                name: "test".to_string(),
                version: (1, 0, 0),
                estimated_size: 100,
            }
        }
        fn memory_size(&self) -> usize {
            self.0.len()
        }
    }

    #[test]
    fn test_message_new() {
        let msg = Message::new(TestMsg("hello".to_string()));
        assert!(!msg.id.is_empty());
    }

    #[test]
    fn test_envelope_new() {
        let env = MessageEnvelope::new(TestMsg("hello".to_string()));
        assert!(env.source.is_none());
        assert!(env.metadata.is_empty());
    }

    #[test]
    fn test_envelope_with_source() {
        let id = ConnectorId::new();
        let env = MessageEnvelope::new(TestMsg("hello".to_string()))
            .with_source(id);
        assert_eq!(env.source, Some(id));
    }
}
