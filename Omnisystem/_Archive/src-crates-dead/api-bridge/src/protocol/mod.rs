pub mod grpc;
pub mod mcp;
pub mod rest;
pub mod websocket;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslatedRequest {
    pub service: String,
    pub method: String,
    pub payload: serde_json::Value,
    pub required_capability: String,
    pub trace_id: String,
}
