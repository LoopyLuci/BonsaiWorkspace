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
