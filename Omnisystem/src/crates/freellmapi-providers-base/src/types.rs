use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRequest {
    pub provider: String,
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub provider: String,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub provider_name: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub timeout_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_request_roundtrip() {
        let req = ProviderRequest {
            provider: "groq".to_string(),
            model: "llama3-8b-8192".to_string(),
            messages: vec![Message { role: "user".to_string(), content: "hi".to_string() }],
            temperature: Some(0.7),
            max_tokens: Some(256),
            top_p: None,
            stream: Some(false),
        };

        let json = serde_json::to_string(&req).unwrap();
        let back: ProviderRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.provider, "groq");
        assert_eq!(back.messages.len(), 1);
        assert_eq!(back.temperature, Some(0.7));
    }

    #[test]
    fn test_provider_response_usage_totals() {
        let resp = ProviderResponse {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            choices: vec![Choice {
                index: 0,
                message: Message { role: "assistant".to_string(), content: "hello".to_string() },
                finish_reason: "stop".to_string(),
            }],
            usage: Usage { prompt_tokens: 10, completion_tokens: 5, total_tokens: 15 },
        };

        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.usage.total_tokens, resp.usage.prompt_tokens + resp.usage.completion_tokens);
    }
}
