use async_trait::async_trait;
use anyhow::Result;
use freellmapi_core::{OpenAIChatRequest, OpenAIChatResponse};
use freellmapi_providers_base::{ProviderAdapter, ProviderRequest, ProviderResponse, Message, Choice, Usage};
use serde::{Deserialize, Serialize};

pub struct MistralAdapter {
    api_key: String,
    base_url: String,
}

impl MistralAdapter {
    pub fn new(api_key: String) -> Self {
        MistralAdapter {
            api_key,
            base_url: "https://api.mistral.ai/v1".to_string(),
        }
    }

    fn get_supported_models_list() -> Vec<String> {
        vec![
            "mistral-large".to_string(),
            "mistral-medium".to_string(),
            "mistral-small".to_string(),
        ]
    }
}

#[async_trait]
impl ProviderAdapter for MistralAdapter {
    fn provider_name(&self) -> &str {
        "mistral"
    }

    async fn authenticate(&self, _api_key: &str) -> Result<()> {
        Ok(())
    }

    async fn translate_request(&self, req: &OpenAIChatRequest) -> Result<ProviderRequest> {
        Ok(ProviderRequest {
            provider: "mistral".to_string(),
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
        let url = format!("{}/chat/completions", self.base_url);

        let mistral_req = MistralChatRequest {
            model: provider_req.model.clone(),
            messages: provider_req.messages.iter().map(|m| MistralMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            }).collect(),
            temperature: provider_req.temperature,
            max_tokens: provider_req.max_tokens,
        };

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .json(&mistral_req)
            .send()
            .await?;

        let mistral_resp: MistralChatResponse = response.json().await?;

        Ok(ProviderResponse {
            provider: "mistral".to_string(),
            model: mistral_resp.model,
            choices: mistral_resp.choices.into_iter().map(|c| Choice {
                index: c.index,
                message: Message {
                    role: c.message.role,
                    content: c.message.content,
                },
                finish_reason: c.finish_reason,
            }).collect(),
            usage: Usage {
                prompt_tokens: mistral_resp.usage.prompt_tokens,
                completion_tokens: mistral_resp.usage.completion_tokens,
                total_tokens: mistral_resp.usage.total_tokens,
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
pub struct MistralMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct MistralChatRequest {
    pub model: String,
    pub messages: Vec<MistralMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Serialize, Deserialize)]
pub struct MistralChatResponse {
    pub model: String,
    pub choices: Vec<MistralChoice>,
    pub usage: MistralUsage,
}

#[derive(Serialize, Deserialize)]
pub struct MistralChoice {
    pub index: u32,
    pub message: MistralMessage,
    pub finish_reason: String,
}

#[derive(Serialize, Deserialize)]
pub struct MistralUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
