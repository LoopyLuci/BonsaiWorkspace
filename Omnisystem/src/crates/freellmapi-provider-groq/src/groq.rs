use async_trait::async_trait;
use anyhow::Result;
use freellmapi_core::{OpenAIChatRequest, OpenAIChatResponse};
use freellmapi_providers_base::{ProviderAdapter, ProviderRequest, ProviderResponse, Message, Choice, Usage};
use serde::{Deserialize, Serialize};

pub struct GroqAdapter {
    api_key: String,
    base_url: String,
}

impl GroqAdapter {
    pub fn new(api_key: String) -> Self {
        GroqAdapter {
            api_key,
            base_url: "https://api.groq.com/openai/v1".to_string(),
        }
    }

    fn get_supported_models_list() -> Vec<String> {
        vec![
            "mixtral-8x7b-32768".to_string(),
            "mixtral-8x7b-32768-vision-preview".to_string(),
            "llama2-70b-4096".to_string(),
            "llama3-70b-8192".to_string(),
            "llama3-8b-8192".to_string(),
            "gemma-7b-it".to_string(),
        ]
    }
}

#[async_trait]
impl ProviderAdapter for GroqAdapter {
    fn provider_name(&self) -> &str {
        "groq"
    }

    async fn authenticate(&self, _api_key: &str) -> Result<()> {
        // Groq API key is validated on first request
        Ok(())
    }

    async fn translate_request(&self, req: &OpenAIChatRequest) -> Result<ProviderRequest> {
        // Groq is OpenAI-compatible, so translation is minimal
        Ok(ProviderRequest {
            provider: "groq".to_string(),
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

        let groq_req = GroqChatRequest {
            model: provider_req.model.clone(),
            messages: provider_req.messages.iter().map(|m| GroqMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            }).collect(),
            temperature: provider_req.temperature,
            max_tokens: provider_req.max_tokens,
            top_p: provider_req.top_p,
        };

        let response = client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&groq_req)
            .send()
            .await?;

        let groq_resp: GroqChatResponse = response.json().await?;

        Ok(ProviderResponse {
            provider: "groq".to_string(),
            model: groq_resp.model,
            choices: groq_resp.choices.into_iter().map(|c| Choice {
                index: c.index,
                message: Message {
                    role: c.message.role,
                    content: c.message.content,
                },
                finish_reason: c.finish_reason,
            }).collect(),
            usage: Usage {
                prompt_tokens: groq_resp.usage.prompt_tokens,
                completion_tokens: groq_resp.usage.completion_tokens,
                total_tokens: groq_resp.usage.total_tokens,
            },
        })
    }

    async fn translate_response(&self, resp: &ProviderResponse) -> Result<OpenAIChatResponse> {
        // Groq response is already in OpenAI format
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
        let client = reqwest::Client::new();
        let url = format!("{}/models", self.base_url);

        match client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GroqMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct GroqChatRequest {
    pub model: String,
    pub messages: Vec<GroqMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
}

#[derive(Serialize, Deserialize)]
pub struct GroqChatResponse {
    pub model: String,
    pub choices: Vec<GroqChoice>,
    pub usage: GroqUsage,
}

#[derive(Serialize, Deserialize)]
pub struct GroqChoice {
    pub index: u32,
    pub message: GroqMessage,
    pub finish_reason: String,
}

#[derive(Serialize, Deserialize)]
pub struct GroqUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
