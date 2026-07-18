use crate::{Connectable, ConnectorId, Result};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Default bound for per-subscriber channels.
const DEFAULT_SUBSCRIBER_CAPACITY: usize = 256;

pub struct PubSubConnector<T: Connectable> {
    id: ConnectorId,
    subscribers: Arc<DashMap<String, Arc<mpsc::Sender<T>>>>,
}

impl<T: Connectable> PubSubConnector<T> {
    pub fn new(id: ConnectorId) -> Self {
        Self {
            id,
            subscribers: Arc::new(DashMap::new()),
        }
    }

    /// Register a new subscriber and return the receiving end of its channel.
    pub fn subscribe(&self, id: impl Into<String>) -> mpsc::Receiver<T> {
        let (tx, rx) = mpsc::channel(DEFAULT_SUBSCRIBER_CAPACITY);
        self.subscribers.insert(id.into(), Arc::new(tx));
        rx
    }

    /// Remove a subscriber so it no longer receives published messages.
    pub fn unsubscribe(&self, id: &str) -> bool {
        self.subscribers.remove(id).is_some()
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }
}

impl<T: Connectable + Clone> PubSubConnector<T> {
    pub async fn publish(&self, message: T) -> Result<()> {
        tracing::debug!("Publishing on connector {}", self.id);

        let mut dead = Vec::new();
        for entry in self.subscribers.iter() {
            let sub_id = entry.key().clone();
            let sender = entry.value().clone();
            if sender.send(message.clone()).await.is_err() {
                dead.push(sub_id);
            }
        }

        for sub_id in dead {
            self.subscribers.remove(&sub_id);
        }

        Ok(())
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
    async fn test_publish_with_no_subscribers() {
        let conn: PubSubConnector<TestMsg> = PubSubConnector::new(ConnectorId::new());
        let msg = TestMsg("hello".to_string());
        assert!(conn.publish(msg).await.is_ok());
    }

    #[tokio::test]
    async fn test_publish_delivers_to_subscriber() {
        let conn: PubSubConnector<TestMsg> = PubSubConnector::new(ConnectorId::new());
        let mut rx = conn.subscribe("sub-1");
        assert_eq!(conn.subscriber_count(), 1);

        conn.publish(TestMsg("hello".to_string())).await.unwrap();

        let received = rx.recv().await.expect("expected a delivered message");
        assert_eq!(received.0, "hello");
    }

    #[tokio::test]
    async fn test_publish_delivers_to_multiple_subscribers() {
        let conn: PubSubConnector<TestMsg> = PubSubConnector::new(ConnectorId::new());
        let mut rx1 = conn.subscribe("sub-1");
        let mut rx2 = conn.subscribe("sub-2");

        conn.publish(TestMsg("broadcast".to_string())).await.unwrap();

        assert_eq!(rx1.recv().await.unwrap().0, "broadcast");
        assert_eq!(rx2.recv().await.unwrap().0, "broadcast");
    }

    #[test]
    fn test_unsubscribe() {
        let conn: PubSubConnector<TestMsg> = PubSubConnector::new(ConnectorId::new());
        let _rx = conn.subscribe("sub-1");
        assert_eq!(conn.subscriber_count(), 1);
        assert!(conn.unsubscribe("sub-1"));
        assert_eq!(conn.subscriber_count(), 0);
    }
}
