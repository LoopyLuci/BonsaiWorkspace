use serde::{Deserialize, Serialize};
use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WebhookStatus {
    Pending,
    Delivered,
    Failed,
    Retrying,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
    pub id: String,
    pub tenant_id: String,
    pub webhook_url: String,
    pub event_id: String,
    pub payload: serde_json::Value,
    pub status: WebhookStatus,
    pub attempts: u32,
    pub max_retries: u32,
    pub created_at: u64,
    pub last_attempt_at: Option<u64>,
}

#[async_trait]
pub trait WebhookDispatcher: Send + Sync {
    async fn dispatch(&self, delivery: &WebhookDelivery) -> Result<bool>;
    async fn get_status(&self, delivery_id: &str) -> Result<WebhookStatus>;
}

pub struct HttpWebhookDispatcher {
    client: reqwest::Client,
}

impl HttpWebhookDispatcher {
    pub fn new() -> Self {
        HttpWebhookDispatcher {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for HttpWebhookDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WebhookDispatcher for HttpWebhookDispatcher {
    async fn dispatch(&self, delivery: &WebhookDelivery) -> Result<bool> {
        match self.client
            .post(&delivery.webhook_url)
            .json(&delivery.payload)
            .send()
            .await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    async fn get_status(&self, _delivery_id: &str) -> Result<WebhookStatus> {
        Ok(WebhookStatus::Delivered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_webhook_delivery_creation() {
        let delivery = WebhookDelivery {
            id: uuid::Uuid::new_v4().to_string(),
            tenant_id: "test-tenant".to_string(),
            webhook_url: "https://example.com/webhook".to_string(),
            event_id: "evt-123".to_string(),
            payload: serde_json::json!({"event": "test"}),
            status: WebhookStatus::Pending,
            attempts: 0,
            max_retries: 3,
            created_at: 1000,
            last_attempt_at: None,
        };

        assert_eq!(delivery.status, WebhookStatus::Pending);
        assert_eq!(delivery.attempts, 0);
    }

    #[tokio::test]
    async fn test_http_webhook_dispatcher() {
        let dispatcher = HttpWebhookDispatcher::new();
        assert!(dispatcher.get_status("any-id").await.is_ok());
    }
}
