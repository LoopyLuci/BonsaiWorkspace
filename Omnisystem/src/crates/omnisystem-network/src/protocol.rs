/// Protocol Handler Module
///
/// Handles RPC protocol:
/// - Message framing
/// - Serialization/deserialization
/// - Request/response matching
/// - Error handling

use crate::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Protocol handler
pub struct ProtocolHandler;

impl ProtocolHandler {
    /// Create protocol handler
    pub fn new() -> Result<Self> {
        info!("Initializing Protocol Handler");
        Ok(Self)
    }

    /// Encode message
    pub fn encode(&self, msg: &RPCMessage) -> Result<Vec<u8>> {
        info!("Encoding RPC message: {:?}", msg.method);
        let json = serde_json::to_vec(msg)?;
        Ok(json)
    }

    /// Decode message
    pub fn decode(&self, data: &[u8]) -> Result<RPCMessage> {
        info!("Decoding RPC message");
        let msg = serde_json::from_slice(data)?;
        Ok(msg)
    }
}

/// RPC message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RPCMessage {
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
}

impl RPCMessage {
    /// Create request message
    pub fn request(method: &str, params: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            method: method.to_string(),
            params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_handler() {
        let handler = ProtocolHandler::new();
        assert!(handler.is_ok());
    }

    #[test]
    fn test_rpc_message() {
        let msg = RPCMessage::request("test", serde_json::json!({}));
        assert_eq!(msg.method, "test");
    }

    #[test]
    fn test_encode_decode_roundtrip_preserves_content() {
        let handler = ProtocolHandler::new().unwrap();
        let msg = RPCMessage::request("get_status", serde_json::json!({"verbose": true}));

        let bytes = handler.encode(&msg).unwrap();
        let decoded = handler.decode(&bytes).unwrap();

        assert_eq!(decoded.id, msg.id);
        assert_eq!(decoded.method, "get_status");
        assert_eq!(decoded.params["verbose"], true);
    }

    #[test]
    fn test_decode_rejects_garbage() {
        let handler = ProtocolHandler::new().unwrap();
        assert!(handler.decode(b"not json").is_err());
    }
}
