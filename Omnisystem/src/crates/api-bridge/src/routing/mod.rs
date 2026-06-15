pub mod discovery;
pub mod load_balancer;

use crate::protocol::TranslatedRequest;

#[derive(Debug, Clone)]
pub struct BackendInstance {
    pub service: String,
    pub url: String,
    pub load: u32,
    pub latency_ms: u32,
}

pub fn route_to_service(req: &TranslatedRequest) -> Vec<BackendInstance> {
    let service = req.service.as_str();
    let instances = match service {
        "mcp-server" => vec![BackendInstance {
            service: service.to_string(),
            url: std::env::var("BONSAI_MCP_URL").unwrap_or_else(|_| "http://127.0.0.1:11425/v1/chat/completions".to_string()),
            load: 10,
            latency_ms: 5,
        }],
        "inference" => vec![BackendInstance {
            service: service.to_string(),
            url: std::env::var("BONSAI_INFERENCE_URL").unwrap_or_else(|_| "http://127.0.0.1:11426/v1/inference".to_string()),
            load: 20,
            latency_ms: 8,
        }],
        "discovery" => vec![BackendInstance {
            service: service.to_string(),
            url: "memory://discovery".to_string(),
            load: 1,
            latency_ms: 1,
        }],
        _ => vec![BackendInstance {
            service: service.to_string(),
            url: format!("http://127.0.0.1:11429/internal/{}", service),
            load: 50,
            latency_ms: 20,
        }],
    };
    instances
}
