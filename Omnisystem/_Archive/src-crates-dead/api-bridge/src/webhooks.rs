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
