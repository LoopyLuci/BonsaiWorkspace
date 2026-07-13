use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use freellmapi_core::{ChatMessage, Choice, Usage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelList {
    pub object: String,
    pub data: Vec<ModelInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub object: String,
    pub owned_by: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventsResponse {
    pub events: Vec<EventData>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventData {
    pub id: String,
    pub event_type: String,
    pub timestamp: u64,
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookRequest {
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookResponse {
    pub id: String,
    pub url: String,
    pub created_at: u64,
}

pub async fn chat_completions(
    Json(payload): Json<ChatCompletionRequest>,
) -> impl IntoResponse {
    let response = ChatCompletionResponse {
        id: uuid::Uuid::new_v4().to_string(),
        object: "chat.completion".to_string(),
        created: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        model: payload.model.clone(),
        choices: vec![Choice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: "Mock response - real implementation uses provider adapters".to_string(),
            },
            finish_reason: "stop".to_string(),
        }],
        usage: Usage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
        },
    };

    (StatusCode::OK, Json(response))
}

pub async fn list_models() -> impl IntoResponse {
    let models = vec![
        ModelInfo {
            id: "gpt-4".to_string(),
            object: "model".to_string(),
            owned_by: "openai".to_string(),
        },
        ModelInfo {
            id: "gpt-3.5-turbo".to_string(),
            object: "model".to_string(),
            owned_by: "openai".to_string(),
        },
        ModelInfo {
            id: "claude-3-opus".to_string(),
            object: "model".to_string(),
            owned_by: "anthropic".to_string(),
        },
    ];

    let response = ModelList {
        object: "list".to_string(),
        data: models,
    };

    (StatusCode::OK, Json(response))
}

pub async fn get_events() -> impl IntoResponse {
    let response = EventsResponse {
        events: vec![],
        total: 0,
    };

    (StatusCode::OK, Json(response))
}

pub async fn register_webhook(
    Json(payload): Json<WebhookRequest>,
) -> impl IntoResponse {
    let response = WebhookResponse {
        id: uuid::Uuid::new_v4().to_string(),
        url: payload.url,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    (StatusCode::CREATED, Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_info_serialization() {
        let model = ModelInfo {
            id: "gpt-4".to_string(),
            object: "model".to_string(),
            owned_by: "openai".to_string(),
        };

        let json = serde_json::to_string(&model).unwrap();
        assert!(json.contains("gpt-4"));
    }

    #[test]
    fn test_model_list_creation() {
        let models = vec![
            ModelInfo {
                id: "gpt-4".to_string(),
                object: "model".to_string(),
                owned_by: "openai".to_string(),
            },
        ];

        let list = ModelList {
            object: "list".to_string(),
            data: models,
        };

        assert_eq!(list.data.len(), 1);
    }
}
