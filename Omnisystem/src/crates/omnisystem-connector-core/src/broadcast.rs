use crate::{Connectable, ConnectorId, Result};
use std::sync::Arc;

pub struct BroadcastConnector<T: Connectable> {
    id: ConnectorId,
    message_count: Arc<std::sync::atomic::AtomicU64>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Connectable> BroadcastConnector<T> {
    pub fn new(id: ConnectorId) -> Self {
        Self {
            id,
            message_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn broadcast(&self, _message: T) -> Result<()> {
        self.message_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tracing::debug!("Broadcasting on connector {}", self.id);
        Ok(())
    }

    pub fn message_count(&self) -> u64 {
        self.message_count.load(std::sync::atomic::Ordering::SeqCst)
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
    async fn test_broadcast() {
        let conn: BroadcastConnector<TestMsg> =
            BroadcastConnector::new(ConnectorId::new());
        let msg = TestMsg("hello".to_string());
        assert!(conn.broadcast(msg).await.is_ok());
        assert_eq!(conn.message_count(), 1);
    }
}
