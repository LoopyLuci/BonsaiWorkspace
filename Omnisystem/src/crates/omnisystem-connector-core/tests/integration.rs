use omnisystem_connector_core::*;

#[test]
fn test_registry_integration() {
    let registry = ConnectorRegistry::new();
    let id = ConnectorId::new();

    registry.register(id).unwrap();
    assert!(registry.exists(id));

    registry.unregister(id).unwrap();
    assert!(!registry.exists(id));
}

#[test]
fn test_arena_integration() {
    let arena = Arena::new(10000);
    assert_eq!(arena.capacity(), 10000);
    assert_eq!(arena.used(), 0);
    assert_eq!(arena.available(), 10000);
}

#[tokio::test]
async fn test_connectors_exist() {
    let id = ConnectorId::new();

    let _req_reply: RequestReplyConnector<TestReq, TestResp> =
        RequestReplyConnector::new(id, 5000);
    let _pubsub: PubSubConnector<TestReq> = PubSubConnector::new(id);
    let _stream: StreamConnector<TestResp> = StreamConnector::new(id, 1000);
    let _broadcast: BroadcastConnector<TestReq> = BroadcastConnector::new(id);
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct TestReq(String);

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct TestResp(String);

impl Connectable for TestReq {
    fn type_id() -> u128 { 1 }
    fn schema() -> connector::Schema {
        connector::Schema {
            type_id: 1,
            name: "req".to_string(),
            version: (1, 0, 0),
            estimated_size: 100,
        }
    }
    fn memory_size(&self) -> usize { self.0.len() }
}

impl Connectable for TestResp {
    fn type_id() -> u128 { 2 }
    fn schema() -> connector::Schema {
        connector::Schema {
            type_id: 2,
            name: "resp".to_string(),
            version: (1, 0, 0),
            estimated_size: 100,
        }
    }
    fn memory_size(&self) -> usize { self.0.len() }
}
