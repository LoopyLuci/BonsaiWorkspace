use async_trait::async_trait;
use anyhow::Result;
use freellmapi_core::{OpenAIChatRequest, OpenAIChatResponse};
use freellmapi_providers_base::{ProviderAdapter, ProviderRequest, ProviderResponse, Message, Choice, Usage};
use serde::{Deserialize, Serialize};

pub struct GeminiAdapter {
    api_key: String,
    base_url: String,
}

impl GeminiAdapter {
    pub fn new(api_key: String) -> Self {
        GeminiAdapter {
            api_key,
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai".to_string(),
        }
    }

    fn get_supported_models_list() -> Vec<String> {
        vec![
            "gemini-1.5-pro".to_string(),
            "gemini-1.5-flash".to_string(),
            "gemini-1.0-pro".to_string(),
        ]
    }
}

#[async_trait]
impl ProviderAdapter for GeminiAdapter {
    fn provider_name(&self) -> &str {
        "gemini"
    }

    async fn authenticate(&self, _api_key: &str) -> Result<()> {
        Ok(())
    }

    async fn translate_request(&self, req: &OpenAIChatRequest) -> Result<ProviderRequest> {
        Ok(ProviderRequest {
            provider: "gemini".to_string(),
            model: req.model.clone(),
            messages: req.messages.iter().map(|m| Message {
                role: m.role.clone(),
                content: m.content.clone(),
            }).collect(),
            temperature: if req.temperature != 1.0 { Some(req.temperature) } else { None },
            max_tokens: req.max_tokens,
            top_p: None,
            stream: Some(req.stream),
        })
    }

    async fn send_request(&self, provider_req: &ProviderRequest) -> Result<ProviderResponse> {
        let client = reqwest::Client::new();
        let url = format!("{}/chat/completions?key={}", self.base_url, &self.api_key);

        let gemini_req = GeminiChatRequest {
            model: provider_req.model.clone(),
            messages: provider_req.messages.iter().map(|m| GeminiMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            }).collect(),
            temperature: provider_req.temperature,
            max_tokens: provider_req.max_tokens,
        };

        let response = client
            .post(&url)
            .json(&gemini_req)
            .send()
            .await?;

        let gemini_resp: GeminiChatResponse = response.json().await?;

        Ok(ProviderResponse {
            provider: "gemini".to_string(),
            model: gemini_resp.model,
            choices: gemini_resp.choices.into_iter().map(|c| Choice {
                index: c.index,
                message: Message {
                    role: c.message.role,
                    content: c.message.content,
                },
                finish_reason: c.finish_reason,
            }).collect(),
            usage: Usage {
                prompt_tokens: gemini_resp.usage.prompt_tokens,
                completion_tokens: gemini_resp.usage.completion_tokens,
                total_tokens: gemini_resp.usage.total_tokens,
            },
        })
    }

    async fn translate_response(&self, resp: &ProviderResponse) -> Result<OpenAIChatResponse> {
        Ok(OpenAIChatResponse {
            id: uuid::Uuid::new_v4().to_string(),
            object: "chat.completion".to_string(),
            created: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            model: resp.model.clone(),
            choices: resp.choices.iter().map(|c| freellmapi_core::Choice {
                index: c.index,
                message: freellmapi_core::ChatMessage {
                    role: c.message.role.clone(),
                    content: c.message.content.clone(),
                },
                finish_reason: c.finish_reason.clone(),
            }).collect(),
            usage: freellmapi_core::Usage {
                prompt_tokens: resp.usage.prompt_tokens,
                completion_tokens: resp.usage.completion_tokens,
                total_tokens: resp.usage.total_tokens,
            },
        })
    }

    async fn get_supported_models(&self) -> Result<Vec<String>> {
        Ok(Self::get_supported_models_list())
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}

#[derive(Serialize, Deserialize)]
pub struct GeminiMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct GeminiChatRequest {
    pub model: String,
    pub messages: Vec<GeminiMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Serialize, Deserialize)]
pub struct GeminiChatResponse {
    pub model: String,
    pub choices: Vec<GeminiChoice>,
    pub usage: GeminiUsage,
}

#[derive(Serialize, Deserialize)]
pub struct GeminiChoice {
    pub index: u32,
    pub message: GeminiMessage,
    pub finish_reason: String,
}

#[derive(Serialize, Deserialize)]
pub struct GeminiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use freellmapi_core::ChatMessage;

    fn sample_request() -> OpenAIChatRequest {
        OpenAIChatRequest {
            model: "gemini-1.5-pro".to_string(),
            messages: vec![ChatMessage { role: "user".to_string(), content: "hi".to_string() }],
            stream: false,
            temperature: 0.5,
            max_tokens: Some(128),
            tools: None,
            tool_choice: None,
        }
    }

    #[test]
    fn test_provider_name() {
        let adapter = GeminiAdapter::new("test-key".to_string());
        assert_eq!(adapter.provider_name(), "gemini");
    }

    #[tokio::test]
    async fn test_translate_request_maps_fields() {
        let adapter = GeminiAdapter::new("test-key".to_string());
        let req = adapter.translate_request(&sample_request()).await.unwrap();

        assert_eq!(req.provider, "gemini");
        assert_eq!(req.model, "gemini-1.5-pro");
        assert_eq!(req.messages[0].content, "hi");
        assert_eq!(req.temperature, Some(0.5));
        assert_eq!(req.max_tokens, Some(128));
    }

    #[tokio::test]
    async fn test_get_supported_models_includes_flash() {
        let adapter = GeminiAdapter::new("test-key".to_string());
        let models = adapter.get_supported_models().await.unwrap();
        assert!(models.contains(&"gemini-1.5-flash".to_string()));
    }
}
