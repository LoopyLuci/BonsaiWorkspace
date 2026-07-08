use async_trait::async_trait;
use anyhow::Result;
use freellmapi_core::{OpenAIChatRequest, OpenAIChatResponse};
use freellmapi_providers_base::{ProviderAdapter, ProviderRequest, ProviderResponse, Message, Choice, Usage};
use serde::{Deserialize, Serialize};

pub struct HuggingFaceAdapter {
    api_key: String,
    base_url: String,
}

impl HuggingFaceAdapter {
    pub fn new(api_key: String) -> Self {
        HuggingFaceAdapter {
            api_key,
            base_url: "https://api-inference.huggingface.co".to_string(),
        }
    }

    fn get_supported_models_list() -> Vec<String> {
        vec![
            "mistralai/Mistral-7B-Instruct-v0.1".to_string(),
            "meta-llama/Llama-2-70b-chat-hf".to_string(),
            "tiiuae/falcon-7b-instruct".to_string(),
        ]
    }
}

#[async_trait]
impl ProviderAdapter for HuggingFaceAdapter {
    fn provider_name(&self) -> &str {
        "huggingface"
    }

    async fn authenticate(&self, _api_key: &str) -> Result<()> {
        Ok(())
    }

    async fn translate_request(&self, req: &OpenAIChatRequest) -> Result<ProviderRequest> {
        Ok(ProviderRequest {
            provider: "huggingface".to_string(),
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
        let model_id = &provider_req.model;
        let url = format!("{}/models/{}", self.base_url, model_id);

        let user_messages: String = provider_req.messages.iter()
            .filter(|m| m.role == "user")
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let hf_req = HFChatRequest {
            inputs: format!("User: {}\nAssistant:", user_messages),
            parameters: HFParameters {
                temperature: provider_req.temperature,
                max_length: provider_req.max_tokens,
            },
        };

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .json(&hf_req)
            .send()
            .await?;

        let hf_resp: Vec<HFResponse> = response.json().await?;
        let text = hf_resp.first()
            .map(|r| r.generated_text.clone())
            .unwrap_or_default();

        Ok(ProviderResponse {
            provider: "huggingface".to_string(),
            model: provider_req.model.clone(),
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: text,
                },
                finish_reason: "stop".to_string(),
            }],
            usage: Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
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
pub struct HFChatRequest {
    pub inputs: String,
    pub parameters: HFParameters,
}

#[derive(Serialize, Deserialize)]
pub struct HFParameters {
    pub temperature: Option<f32>,
    pub max_length: Option<u32>,
}

#[derive(Serialize, Deserialize)]
pub struct HFResponse {
    pub generated_text: String,
}
