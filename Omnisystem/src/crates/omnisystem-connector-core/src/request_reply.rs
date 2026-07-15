use crate::{Connectable, ConnectorError, ConnectorId, Result};
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
            .and_then(|r| r.map_err(|_| ConnectorError::ChannelClosed));

        self.pending.remove(&request_id);

        result
    }

    /// Fulfil a previously issued request by id, unblocking whoever is
    /// awaiting the corresponding `send_request` call.
    pub fn complete_request(&self, request_id: &str, response: Resp) -> Result<()> {
        let (_, tx) = self
            .pending
            .remove(request_id)
            .ok_or_else(|| ConnectorError::NotFound(request_id.to_string()))?;

        tx.send(response).map_err(|_| ConnectorError::ChannelClosed)
    }

    /// Ids of requests that are currently awaiting a reply.
    pub fn pending_request_ids(&self) -> Vec<String> {
        self.pending.iter().map(|entry| entry.key().clone()).collect()
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc as StdArc;
    use std::time::Duration;

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

    #[tokio::test]
    async fn test_send_request_times_out_without_completion() {
        let conn: RequestReplyConnector<TestReq, TestResp> =
            RequestReplyConnector::new(ConnectorId::new(), 20);
        let result = conn.send_request(&TestReq("ping".to_string())).await;
        assert!(matches!(result, Err(ConnectorError::Timeout)));
        // The pending entry should have been cleaned up after timing out.
        assert_eq!(conn.pending_count(), 0);
    }

    #[tokio::test]
    async fn test_send_request_completed_via_complete_request() {
        let conn = StdArc::new(RequestReplyConnector::<TestReq, TestResp>::new(
            ConnectorId::new(),
            5000,
        ));
        let conn2 = conn.clone();

        let handle = tokio::spawn(async move {
            conn2.send_request(&TestReq("ping".to_string())).await
        });

        // Wait until the request has actually registered as pending, then
        // complete it. This avoids any fixed-sleep race condition.
        let request_id = loop {
            if let Some(id) = conn.pending_request_ids().into_iter().next() {
                break id;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        };

        conn.complete_request(&request_id, TestResp("pong".to_string()))
            .unwrap();

        let resp = handle.await.unwrap().unwrap();
        assert_eq!(resp.0, "pong");
    }

    #[test]
    fn test_complete_request_unknown_id_errors() {
        let conn: RequestReplyConnector<TestReq, TestResp> =
            RequestReplyConnector::new(ConnectorId::new(), 5000);
        let result = conn.complete_request("does-not-exist", TestResp("x".to_string()));
        assert!(matches!(result, Err(ConnectorError::NotFound(_))));
    }
}
