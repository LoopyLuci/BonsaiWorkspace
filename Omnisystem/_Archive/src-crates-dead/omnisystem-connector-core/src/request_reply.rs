use crate::{Connectable, ConnectorId, Result, ConnectorError};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::oneshot;

pub struct RequestReplyConnector<Req, Resp>
where
    Req: Connectable,
    Resp: Connectable,
{
    id: ConnectorId,
    pending: Arc<DashMap<String, oneshot::Sender<Resp>>>,
    timeout: std::time::Duration,
    _req: std::marker::PhantomData<Req>,
}

impl<Req, Resp> RequestReplyConnector<Req, Resp>
where
    Req: Connectable,
    Resp: Connectable,
{
    pub fn new(id: ConnectorId, timeout_ms: u64) -> Self {
        Self {
            id,
            pending: Arc::new(DashMap::new()),
            timeout: std::time::Duration::from_millis(timeout_ms),
            _req: std::marker::PhantomData,
        }
    }

    pub async fn send_request(&self, _request: &Req) -> Result<Resp> {
        let (tx, rx) = oneshot::channel();
        let request_id = uuid::Uuid::new_v4().to_string();

        self.pending.insert(request_id.clone(), tx);

        tracing::debug!(
            "Sending request {} on connector {}",
            request_id,
            self.id
        );

        tokio::time::sleep(std::time::Duration::from_micros(50)).await;

        let result = tokio::time::timeout(self.timeout, rx)
            .await
            .map_err(|_| ConnectorError::Timeout)
            .and_then(|r| r.map_err(|_| ConnectorError::ChannelClosed))?;

        self.pending.remove(&request_id);

        Ok(result)
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Serialize, Deserialize)]
    struct TestReq(String);

    #[derive(Clone, Serialize, Deserialize)]
    struct TestResp(String);

    impl Connectable for TestReq {
        fn type_id() -> u128 {
            1
        }
        fn schema() -> crate::connector::Schema {
            crate::connector::Schema {
                type_id: 1,
                name: "req".to_string(),
                version: (1, 0, 0),
                estimated_size: 100,
            }
        }
        fn memory_size(&self) -> usize {
            self.0.len()
        }
    }

    impl Connectable for TestResp {
        fn type_id() -> u128 {
            2
        }
        fn schema() -> crate::connector::Schema {
            crate::connector::Schema {
                type_id: 2,
                name: "resp".to_string(),
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
        let _conn: RequestReplyConnector<TestReq, TestResp> =
            RequestReplyConnector::new(ConnectorId::new(), 5000);
    }

    #[test]
    fn test_pending_count() {
        let conn: RequestReplyConnector<TestReq, TestResp> =
            RequestReplyConnector::new(ConnectorId::new(), 5000);
        assert_eq!(conn.pending_count(), 0);
    }
}
