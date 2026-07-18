use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
    pub event_type: String,
    pub payload: serde_json::Value,
    pub url: String,
}

pub async fn deliver_webhook(delivery: &WebhookDelivery) -> Result<()> {
    let client = reqwest::Client::new();
    client
        .post(&delivery.url)
        .json(&delivery.payload)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_delivery_to_malformed_url_fails() {
        let delivery = WebhookDelivery {
            event_type: "test".to_string(),
            payload: serde_json::json!({ "ok": true }),
            url: "not-a-valid-url".to_string(),
        };

        assert!(deliver_webhook(&delivery).await.is_err());
    }

    #[tokio::test]
    async fn test_delivery_to_unreachable_port_fails() {
        let delivery = WebhookDelivery {
            event_type: "test".to_string(),
            payload: serde_json::json!({ "ok": true }),
            // Port 1 is a well-known privileged port that nothing listens
            // on in a normal test environment, so this connection is
            // refused immediately rather than hanging.
            url: "http://127.0.0.1:1/webhook".to_string(),
        };

        assert!(deliver_webhook(&delivery).await.is_err());
    }
}
