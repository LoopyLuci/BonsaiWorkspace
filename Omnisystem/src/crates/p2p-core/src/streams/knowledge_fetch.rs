use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeRequest {
    pub module_id: String,
    pub query_vector: Vec<f32>,
    pub top_k: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeItem {
    pub key_hash: String,
    pub value: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeResponse {
    pub items: Vec<KnowledgeItem>,
}

pub struct KnowledgeFetchStream;

impl KnowledgeFetchStream {
    pub fn encode_request(req: &KnowledgeRequest) -> Vec<u8> {
        serde_json::to_vec(req).unwrap_or_default()
    }

    pub fn decode_request(data: &[u8]) -> Option<KnowledgeRequest> {
        serde_json::from_slice(data).ok()
    }

    pub fn encode_response(resp: &KnowledgeResponse) -> Vec<u8> {
        serde_json::to_vec(resp).unwrap_or_default()
    }

    pub fn decode_response(data: &[u8]) -> Option<KnowledgeResponse> {
        serde_json::from_slice(data).ok()
    }
}
