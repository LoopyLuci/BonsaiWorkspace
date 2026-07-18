//! Small demo CLI exercising the real connector-core primitives:
//! registers a connector, publishes on a pub/sub channel, broadcasts on a
//! broadcast channel, and drives a request/reply round trip.

use omnisystem_connector_core::{
    connector::Schema, BroadcastConnector, Connectable, ConnectorId, ConnectorRegistry,
    PubSubConnector, RequestReplyConnector,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DemoMessage(String);

impl Connectable for DemoMessage {
    fn type_id() -> u128 {
        42
    }

    fn schema() -> Schema {
        Schema {
            type_id: 42,
            name: "demo_message".to_string(),
            version: (1, 0, 0),
            estimated_size: 64,
        }
    }

    fn memory_size(&self) -> usize {
        self.0.len()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = ConnectorRegistry::new();
    let pubsub_id = ConnectorId::new();
    registry.register_named(pubsub_id, "demo-pubsub".to_string())?;
    println!("Registered connector: {pubsub_id}");

    let pubsub: PubSubConnector<DemoMessage> = PubSubConnector::new(pubsub_id);
    let mut rx = pubsub.subscribe("demo-subscriber");
    pubsub
        .publish(DemoMessage("hello from pubsub".to_string()))
        .await?;
    if let Some(msg) = rx.recv().await {
        println!("pubsub delivered: {}", msg.0);
    }

    let broadcast: BroadcastConnector<DemoMessage> = BroadcastConnector::new(ConnectorId::new());
    let mut brx = broadcast.subscribe();
    broadcast
        .broadcast(DemoMessage("hello from broadcast".to_string()))
        .await?;
    if let Ok(msg) = brx.recv().await {
        println!("broadcast delivered: {}", msg.0);
    }

    let req_reply: RequestReplyConnector<DemoMessage, DemoMessage> =
        RequestReplyConnector::new(ConnectorId::new(), 1000);
    let req_reply = std::sync::Arc::new(req_reply);
    let responder = req_reply.clone();
    let handle = tokio::spawn(async move {
        responder
            .send_request(&DemoMessage("ping".to_string()))
            .await
    });

    let request_id = loop {
        if let Some(id) = req_reply.pending_request_ids().into_iter().next() {
            break id;
        }
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    };
    req_reply.complete_request(&request_id, DemoMessage("pong".to_string()))?;

    match handle.await? {
        Ok(resp) => println!("request/reply completed: {}", resp.0),
        Err(e) => println!("request/reply failed: {e}"),
    }

    Ok(())
}
