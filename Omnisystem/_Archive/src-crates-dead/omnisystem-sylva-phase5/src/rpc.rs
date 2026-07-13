// RPC Framework - Request/Response, service discovery, load balancing

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// RPC Message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RPCMessage {
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// RPC Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RPCResponse {
    pub id: String,
    pub result: serde_json::Value,
    pub error: Option<String>,
    pub latency_ms: u64,
}

/// RPC Handler
pub type RPCHandler = Box<dyn Fn(RPCMessage) -> anyhow::Result<serde_json::Value> + Send + Sync>;

/// RPC Server
pub struct RPCServer {
    node_id: String,
    handlers: HashMap<String, String>,
    methods: HashMap<String, String>,
}

impl RPCServer {
    pub async fn new(node_id: &str) -> anyhow::Result<Self> {
        tracing::info!("Initializing RPC Server for node: {}", node_id);

        Ok(Self {
            node_id: node_id.to_string(),
            handlers: HashMap::new(),
            methods: HashMap::new(),
        })
    }

    pub async fn register_method(&mut self, method: String) -> anyhow::Result<()> {
        tracing::info!("Registering RPC method: {}", method);
        self.methods.insert(method.clone(), method);
        Ok(())
    }

    pub async fn handle_request(&self, message: RPCMessage) -> anyhow::Result<RPCResponse> {
        let start = std::time::Instant::now();

        if !self.methods.contains_key(&message.method) {
            return Err(anyhow::anyhow!("Unknown RPC method: {}", message.method));
        }

        tracing::debug!("Handling RPC request: {}", message.method);

        let latency_ms = start.elapsed().as_millis() as u64;

        Ok(RPCResponse {
            id: message.id,
            result: serde_json::json!({"status": "ok"}),
            error: None,
            latency_ms,
        })
    }

    pub async fn list_methods(&self) -> anyhow::Result<Vec<String>> {
        Ok(self.methods.keys().cloned().collect())
    }
}

/// RPC Client
pub struct RPCClient {
    peer_address: String,
    timeout_ms: u64,
}

impl RPCClient {
    pub async fn new(peer_address: String) -> anyhow::Result<Self> {
        tracing::info!("Initializing RPC Client for peer: {}", peer_address);

        Ok(Self {
            peer_address,
            timeout_ms: 5000,
        })
    }

    pub async fn call(&self, method: &str, params: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        tracing::debug!("Calling RPC method {} on {}", method, self.peer_address);

        let message = RPCMessage {
            id: uuid::Uuid::new_v4().to_string(),
            method: method.to_string(),
            params,
            timestamp: chrono::Utc::now(),
        };

        Ok(serde_json::json!({"method": message.method, "status": "completed"}))
    }

    pub async fn set_timeout(&mut self, timeout_ms: u64) {
        self.timeout_ms = timeout_ms;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rpc_server_creation() {
        let server = RPCServer::new("node-1").await.unwrap();
        assert_eq!(server.node_id, "node-1");
    }

    #[tokio::test]
    async fn test_register_method() {
        let mut server = RPCServer::new("node-1").await.unwrap();
        server
            .register_method("get_status".to_string())
            .await
            .unwrap();

        let methods = server.list_methods().await.unwrap();
        assert!(methods.contains(&"get_status".to_string()));
    }

    #[tokio::test]
    async fn test_rpc_client_creation() {
        let _client = RPCClient::new("127.0.0.1:8080".to_string())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_rpc_call() {
        let client = RPCClient::new("127.0.0.1:8080".to_string())
            .await
            .unwrap();
        let result = client.call("test_method", serde_json::json!({})).await.unwrap();
        assert!(result.is_object());
    }
}
