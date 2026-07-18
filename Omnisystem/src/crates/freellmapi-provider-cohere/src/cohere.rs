use async_trait::async_trait;
use anyhow::Result;
use freellmapi_core::{OpenAIChatRequest, OpenAIChatResponse};
use freellmapi_providers_base::{ProviderAdapter, ProviderRequest, ProviderResponse, Message, Choice, Usage};
use serde::{Deserialize, Serialize};

pub struct CohereAdapter {
    api_key: String,
    base_url: String,
}

impl CohereAdapter {
    pub fn new(api_key: String) -> Self {
        CohereAdapter {
            api_key,
            base_url: "https://api.cohere.com/v1".to_string(),
        }
    }

    fn get_supported_models_list() -> Vec<String> {
        vec![
            "command-r-plus".to_string(),
            "command-r".to_string(),
            "command".to_string(),
        ]
    }
}

#[async_trait]
impl ProviderAdapter for CohereAdapter {
    fn provider_name(&self) -> &str {
        "cohere"
    }

    async fn authenticate(&self, _api_key: &str) -> Result<()> {
        Ok(())
    }

    async fn translate_request(&self, req: &OpenAIChatRequest) -> Result<ProviderRequest> {
        Ok(ProviderRequest {
            provider: "cohere".to_string(),
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
        let url = format!("{}/chat", self.base_url);

        let cohere_req = CohereChatRequest {
            model: provider_req.model.clone(),
            messages: provider_req.messages.iter().map(|m| CohereMessage {
                role: m.role.clone(),
                message: m.content.clone(),
            }).collect(),
            temperature: provider_req.temperature,
            max_tokens: provider_req.max_tokens,
        };

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .json(&cohere_req)
            .send()
            .await?;

        let cohere_resp: CohereChatResponse = response.json().await?;

        Ok(ProviderResponse {
            provider: "cohere".to_string(),
            model: cohere_resp.model,
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: cohere_resp.text,
                },
                finish_reason: cohere_resp.finish_reason,
            }],
            usage: Usage {
                prompt_tokens: cohere_resp.meta.billed_units.input_tokens,
                completion_tokens: cohere_resp.meta.billed_units.output_tokens,
                total_tokens: cohere_resp.meta.billed_units.input_tokens + cohere_resp.meta.billed_units.output_tokens,
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
pub struct CohereMessage {
    pub role: String,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct CohereChatRequest {
    pub model: String,
    pub messages: Vec<CohereMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Serialize, Deserialize)]
pub struct CohereChatResponse {
    pub model: String,
    pub text: String,
    pub finish_reason: String,
    pub meta: CohereMeta,
}

#[derive(Serialize, Deserialize)]
pub struct CohereMeta {
    pub billed_units: CohereBilledUnits,
}

#[derive(Serialize, Deserialize)]
pub struct CohereBilledUnits {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use freellmapi_core::ChatMessage;

    fn sample_request() -> OpenAIChatRequest {
        OpenAIChatRequest {
            model: "command-r-plus".to_string(),
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
        let adapter = CohereAdapter::new("test-key".to_string());
        assert_eq!(adapter.provider_name(), "cohere");
    }

    #[tokio::test]
    async fn test_translate_request_maps_fields() {
        let adapter = CohereAdapter::new("test-key".to_string());
        let req = adapter.translate_request(&sample_request()).await.unwrap();

        assert_eq!(req.provider, "cohere");
        assert_eq!(req.model, "command-r-plus");
        assert_eq!(req.messages[0].content, "hi");
        assert_eq!(req.temperature, Some(0.5));
        assert_eq!(req.max_tokens, Some(128));
    }

    #[tokio::test]
    async fn test_get_supported_models_includes_command() {
        let adapter = CohereAdapter::new("test-key".to_string());
        let models = adapter.get_supported_models().await.unwrap();
        assert!(models.contains(&"command".to_string()));
    }
}
