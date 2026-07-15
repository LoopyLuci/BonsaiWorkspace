// Note: a `grpc` protocol module existed in the archived crate
// (tonic::include_proto! over proto/bridge.proto) but required a build.rs
// invoking `protoc`, which is not available in this environment. It has
// been left out; `proto/bridge.proto` remains as the contract for a future
// gRPC gateway.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translated_request_roundtrips_json() {
        let req = TranslatedRequest {
            service: "inference".to_string(),
            method: "generate".to_string(),
            payload: serde_json::json!({ "prompt": "hi" }),
            required_capability: "ApiCap:inference".to_string(),
            trace_id: "trace-1".to_string(),
        };

        let json = serde_json::to_string(&req).unwrap();
        let back: TranslatedRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.service, "inference");
        assert_eq!(back.trace_id, "trace-1");
    }
}
