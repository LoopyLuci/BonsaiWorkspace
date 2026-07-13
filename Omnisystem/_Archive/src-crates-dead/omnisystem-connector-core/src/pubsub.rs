use crate::{Connectable, ConnectorId, Result};
use dashmap::DashMap;
use std::sync::Arc;

pub struct PubSubConnector<T: Connectable> {
    id: ConnectorId,
    subscribers: Arc<DashMap<String, Arc<tokio::sync::mpsc::Sender<T>>>>,
}

impl<T: Connectable> PubSubConnector<T> {
    pub fn new(id: ConnectorId) -> Self {
        Self {
            id,
            subscribers: Arc::new(DashMap::new()),
        }
    }

    pub async fn publish(&self, _message: T) -> Result<()> {
        tracing::debug!("Publishing on connector {}", self.id);
        Ok(())
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Serialize, Deserialize)]
    struct TestMsg(String);

    impl Connectable for TestMsg {
        fn type_id() -> u128 {
            3
        }
        fn schema() -> crate::connector::Schema {
            crate::connector::Schema {
                type_id: 3,
                name: "msg".to_string(),
                version: (1, 0, 0),
                estimated_size: 100,
            }
        }
        fn memory_size(&self) -> usize {
            self.0.len()
        }
    }

    #[test]
    fn test_new() {
        let _conn: PubSubConnector<TestMsg> = PubSubConnector::new(ConnectorId::new());
    }

    #[test]
    fn test_subscriber_count() {
        let conn: PubSubConnector<TestMsg> = PubSubConnector::new(ConnectorId::new());
        assert_eq!(conn.subscriber_count(), 0);
    }

    #[tokio::test]
    async fn test_publish() {
        let conn: PubSubConnector<TestMsg> = PubSubConnector::new(ConnectorId::new());
        let msg = TestMsg("hello".to_string());
        assert!(conn.publish(msg).await.is_ok());
    }
}
