use crate::{Connectable, ConnectorId, Result};
use parking_lot::Mutex;
use std::sync::Arc;

pub struct StreamConnector<T: Connectable> {
    id: ConnectorId,
    buffer: Arc<Mutex<Vec<T>>>,
}

impl<T: Connectable> StreamConnector<T> {
    pub fn new(id: ConnectorId, capacity: usize) -> Self {
        Self {
            id,
            buffer: Arc::new(Mutex::new(Vec::with_capacity(capacity))),
        }
    }

    pub async fn write(&self, item: T) -> Result<()> {
        let mut buf = self.buffer.lock();
        buf.push(item);
        tracing::debug!("Wrote to stream on connector {}", self.id);
        Ok(())
    }

    pub fn item_count(&self) -> usize {
        self.buffer.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Serialize, Deserialize)]
    struct TestItem(i32);

    impl Connectable for TestItem {
        fn type_id() -> u128 {
            4
        }
        fn schema() -> crate::connector::Schema {
            crate::connector::Schema {
                type_id: 4,
                name: "item".to_string(),
                version: (1, 0, 0),
                estimated_size: 8,
            }
        }
        fn memory_size(&self) -> usize {
            std::mem::size_of::<i32>()
        }
    }

    #[test]
    fn test_new() {
        let _conn: StreamConnector<TestItem> =
            StreamConnector::new(ConnectorId::new(), 1000);
    }

    #[tokio::test]
    async fn test_write() {
        let conn: StreamConnector<TestItem> =
            StreamConnector::new(ConnectorId::new(), 1000);
        assert!(conn.write(TestItem(42)).await.is_ok());
        assert_eq!(conn.item_count(), 1);
    }
}
