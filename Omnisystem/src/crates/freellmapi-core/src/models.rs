use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub email: String,
    pub tier: String, // "free", "pro", "enterprise"
    pub monthly_budget_usd: f64,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub tenant_id: String,
    pub key_hash: String, // SHA-256(key)
    pub scopes: Vec<String>,
    pub created_at: u64,
    pub expires_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderKey {
    pub id: String,
    pub tenant_id: String,
    pub provider: String,
    pub key_encrypted: Vec<u8>, // AES-256-GCM
    pub validation_status: String, // "valid", "rate_limited", "invalid"
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    pub id: String,
    pub tenant_id: String,
    pub api_key_id: String,
    pub model: String,
    pub provider: String,
    pub tokens_in: u32,
    pub tokens_out: u32,
    pub cost_usd: f64,
    pub latency_ms: u32,
    pub status_code: u16,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "user", "assistant", "system"
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub r#type: String, // "function"
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: Option<String>,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolChoice {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "none")]
    None,
    #[serde(untagged)]
    Function { function: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<Tool>>,
    pub tool_choice: Option<ToolChoice>,
}

fn default_temperature() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub event_type: String,
    pub tenant_id: Option<String>,
    pub timestamp: u64,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub event_type: String,
    pub timestamp: u64,
    pub tenant_id: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: String,
    pub tenant_id: String,
    pub url: String,
    pub events: Vec<String>,
    pub secret_key: String,
    pub enabled: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetrics {
    pub name: String,
    pub alpha: f64,
    pub beta: f64,
    pub avg_latency_ms: f64,
    pub cost_per_1k_tokens: f64,
}

impl ProviderMetrics {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            alpha: 1.0,
            beta: 1.0,
            avg_latency_ms: 0.0,
            cost_per_1k_tokens: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheValue<T> {
    pub value: T,
    pub expires_at: u64,
}

impl<T> CacheValue<T> {
    pub fn new(value: T, expires_at: u64) -> Self {
        Self { value, expires_at }
    }

    pub fn is_expired(&self, now: u64) -> bool {
        self.expires_at <= now
    }
}

pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36); // UUID format
    }

    #[test]
    fn test_cache_value_expiration() {
        let now = unix_now();
        let cached = CacheValue::new("test", now + 100);
        assert!(!cached.is_expired(now));
        assert!(cached.is_expired(now + 101));
    }

    #[test]
    fn test_provider_metrics_creation() {
        let metrics = ProviderMetrics::new("groq");
        assert_eq!(metrics.name, "groq");
        assert_eq!(metrics.alpha, 1.0);
        assert_eq!(metrics.beta, 1.0);
    }
}
