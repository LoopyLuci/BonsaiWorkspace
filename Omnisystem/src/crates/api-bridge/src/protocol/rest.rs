use crate::protocol::TranslatedRequest;

pub fn to_translated(path: &str, payload: serde_json::Value, trace_id: String) -> Option<TranslatedRequest> {
    let mapping = match path {
        "/api/v1/chat/completions" => ("mcp-server", "chat", "ApiCap:inference"),
        "/api/v1/inference" => ("inference", "generate", "ApiCap:inference"),
        "/api/v1/remote/peers" => ("discovery", "list_peers", "ApiCap:discovery"),
        "/api/v1/file/sync" => ("file-sync", "sync", "ApiCap:file_sync"),
        "/api/v1/blockchain/tx" => ("nexus-core", "submit_tx", "ApiCap:blockchain"),
        _ => return None,
    };

    Some(TranslatedRequest {
        service: mapping.0.to_string(),
        method: mapping.1.to_string(),
        payload,
        required_capability: mapping.2.to_string(),
        trace_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_routes_translate() {
        let req = to_translated(
            "/api/v1/inference",
            serde_json::json!({}),
            "t1".to_string(),
        )
        .unwrap();
        assert_eq!(req.service, "inference");
        assert_eq!(req.required_capability, "ApiCap:inference");

        let req = to_translated(
            "/api/v1/remote/peers",
            serde_json::json!({}),
            "t2".to_string(),
        )
        .unwrap();
        assert_eq!(req.service, "discovery");
        assert_eq!(req.required_capability, "ApiCap:discovery");
    }

    #[test]
    fn test_unknown_route_returns_none() {
        assert!(to_translated("/nope", serde_json::json!({}), "t".to_string()).is_none());
    }

    #[test]
    fn test_payload_and_trace_id_are_preserved() {
        let payload = serde_json::json!({ "key": "value" });
        let req = to_translated("/api/v1/blockchain/tx", payload.clone(), "abc".to_string()).unwrap();
        assert_eq!(req.payload, payload);
        assert_eq!(req.trace_id, "abc");
    }
}
