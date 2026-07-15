use crate::{Connectable, ConnectorId, Result};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Default channel capacity for the underlying broadcast channel.
const DEFAULT_BROADCAST_CAPACITY: usize = 256;

pub struct BroadcastConnector<T: Connectable + Clone> {
    id: ConnectorId,
    message_count: Arc<std::sync::atomic::AtomicU64>,
    tx: broadcast::Sender<T>,
}

impl<T: Connectable + Clone> BroadcastConnector<T> {
    pub fn new(id: ConnectorId) -> Self {
        let (tx, _rx) = broadcast::channel(DEFAULT_BROADCAST_CAPACITY);
        Self {
            id,
            message_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            tx,
        }
    }

    /// Register a new receiver that will observe all future broadcast messages.
    pub fn subscribe(&self) -> broadcast::Receiver<T> {
        self.tx.subscribe()
    }

    pub async fn broadcast(&self, message: T) -> Result<()> {
        self.message_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tracing::debug!("Broadcasting on connector {}", self.id);

        // It's fine if there are no active receivers yet; that's a normal
        // state for a broadcast channel and not an error condition.
        let _ = self.tx.send(message);

        Ok(())
    }

    pub fn message_count(&self) -> u64 {
        self.message_count.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
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
            5
        }
        fn schema() -> crate::connector::Schema {
            crate::connector::Schema {
                type_id: 5,
                name: "broadcast".to_string(),
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
        let _conn: BroadcastConnector<TestMsg> =
            BroadcastConnector::new(ConnectorId::new());
    }

    #[tokio::test]
    async fn test_broadcast_with_no_subscribers() {
        let conn: BroadcastConnector<TestMsg> =
            BroadcastConnector::new(ConnectorId::new());
        let msg = TestMsg("hello".to_string());
        assert!(conn.broadcast(msg).await.is_ok());
        assert_eq!(conn.message_count(), 1);
    }

    #[tokio::test]
    async fn test_broadcast_delivers_to_subscriber() {
        let conn: BroadcastConnector<TestMsg> = BroadcastConnector::new(ConnectorId::new());
        let mut rx = conn.subscribe();
        assert_eq!(conn.receiver_count(), 1);

        conn.broadcast(TestMsg("hello".to_string())).await.unwrap();

        let received = rx.recv().await.expect("expected a delivered message");
        assert_eq!(received.0, "hello");
        assert_eq!(conn.message_count(), 1);
    }

    #[tokio::test]
    async fn test_broadcast_delivers_to_multiple_subscribers() {
        let conn: BroadcastConnector<TestMsg> = BroadcastConnector::new(ConnectorId::new());
        let mut rx1 = conn.subscribe();
        let mut rx2 = conn.subscribe();

        conn.broadcast(TestMsg("fanout".to_string())).await.unwrap();

        assert_eq!(rx1.recv().await.unwrap().0, "fanout");
        assert_eq!(rx2.recv().await.unwrap().0, "fanout");
    }
}
