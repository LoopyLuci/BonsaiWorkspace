use async_trait::async_trait;
use anyhow::Result;
use freellmapi_core::{OpenAIChatRequest, OpenAIChatResponse};
use freellmapi_providers_base::{ProviderAdapter, ProviderRequest, ProviderResponse, Message, Choice, Usage};
use serde::{Deserialize, Serialize};

pub struct AnthropicAdapter {
    api_key: String,
    base_url: String,
}

impl AnthropicAdapter {
    pub fn new(api_key: String) -> Self {
        AnthropicAdapter {
            api_key,
            base_url: "https://api.anthropic.com/v1".to_string(),
        }
    }

    fn get_supported_models_list() -> Vec<String> {
        vec![
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
        ]
    }
}

#[async_trait]
impl ProviderAdapter for AnthropicAdapter {
    fn provider_name(&self) -> &str {
        "anthropic"
    }

    async fn authenticate(&self, _api_key: &str) -> Result<()> {
        Ok(())
    }

    async fn translate_request(&self, req: &OpenAIChatRequest) -> Result<ProviderRequest> {
        Ok(ProviderRequest {
            provider: "anthropic".to_string(),
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
        let url = format!("{}/messages", self.base_url);

        let anthropic_req = AnthropicMessageRequest {
            model: provider_req.model.clone(),
            max_tokens: provider_req.max_tokens.unwrap_or(1024),
            messages: provider_req.messages.iter().map(|m| AnthropicMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            }).collect(),
            temperature: provider_req.temperature,
        };

        let response = client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&anthropic_req)
            .send()
            .await?;

        let anthropic_resp: AnthropicMessageResponse = response.json().await?;

        Ok(ProviderResponse {
            provider: "anthropic".to_string(),
            model: anthropic_resp.model,
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: anthropic_resp.content.first()
                        .map(|c| c.text.clone())
                        .unwrap_or_default(),
                },
                finish_reason: anthropic_resp.stop_reason,
            }],
            usage: Usage {
                prompt_tokens: anthropic_resp.usage.input_tokens,
                completion_tokens: anthropic_resp.usage.output_tokens,
                total_tokens: anthropic_resp.usage.input_tokens + anthropic_resp.usage.output_tokens,
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
pub struct AnthropicMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct AnthropicMessageRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<AnthropicMessage>,
    pub temperature: Option<f32>,
}

#[derive(Serialize, Deserialize)]
pub struct AnthropicMessageResponse {
    pub id: String,
    pub model: String,
    pub content: Vec<AnthropicContent>,
    pub stop_reason: String,
    pub usage: AnthropicUsage,
}

#[derive(Serialize, Deserialize)]
pub struct AnthropicContent {
    pub r#type: String,
    pub text: String,
}

#[derive(Serialize, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}
