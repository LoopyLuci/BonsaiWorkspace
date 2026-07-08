//! ChatGateway Actor - Normalizes messages from multiple platforms
//!
//! Responsibilities:
//! - Receive messages from multiple platforms (Slack, Discord, Teams, etc.)
//! - Normalize to a common message format
//! - Route to CommandParser
//! - Track message metrics and history
//! - Handle platform-specific formatting

use crate::actor::{Actor, ActorId, Snapshot};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Platform where message originated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    Slack,
    Discord,
    Teams,
    Telegram,
    WebSocket,
    HTTP,
}

impl Platform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Slack => "slack",
            Self::Discord => "discord",
            Self::Teams => "teams",
            Self::Telegram => "telegram",
            Self::WebSocket => "websocket",
            Self::HTTP => "http",
        }
    }
}

/// Normalized message format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedMessage {
    pub message_id: String,
    pub user_id: String,
    pub content: String,
    pub platform: Platform,
    pub channel_id: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl NormalizedMessage {
    pub fn new(
        user_id: String,
        content: String,
        platform: Platform,
        channel_id: String,
    ) -> Self {
        Self {
            message_id: format!("msg-{}", Uuid::new_v4()),
            user_id,
            content,
            platform,
            channel_id,
            timestamp: Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }
}

/// Messages that ChatGateway can receive
#[derive(Debug, Clone)]
pub enum ChatGatewayMessage {
    /// Raw message from a platform
    IncomingMessage {
        platform: Platform,
        raw: serde_json::Value,
    },
    /// Get metrics
    GetMetrics,
    /// Get message history
    GetHistory { limit: usize },
    /// Stop the actor
    Stop,
}

/// Metrics tracked by ChatGateway
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatMetrics {
    pub total_messages: u64,
    pub messages_by_platform: std::collections::HashMap<String, u64>,
    pub avg_processing_time_ms: f64,
    pub last_message_time: Option<DateTime<Utc>>,
}

/// ChatGateway actor state
pub struct ChatGateway {
    id: ActorId,
    metrics: Arc<ChatMetrics>,
    message_history: Arc<Vec<NormalizedMessage>>,
    max_history: usize,
}

impl ChatGateway {
    pub fn new(max_history: usize) -> Self {
        Self {
            id: ActorId::new(),
            metrics: Arc::new(ChatMetrics::default()),
            message_history: Arc::new(Vec::new()),
            max_history,
        }
    }

    /// Normalize message based on platform
    fn normalize(&self, platform: Platform, raw: serde_json::Value) -> Result<NormalizedMessage, String> {
        match platform {
            Platform::Slack => self.normalize_slack(raw),
            Platform::Discord => self.normalize_discord(raw),
            Platform::Teams => self.normalize_teams(raw),
            Platform::Telegram => self.normalize_telegram(raw),
            Platform::WebSocket | Platform::HTTP => self.normalize_http(raw),
        }
    }

    fn normalize_slack(&self, raw: serde_json::Value) -> Result<NormalizedMessage, String> {
        let user_id = raw
            .get("user")
            .and_then(|v| v.as_str())
            .ok_or("Missing user field")?
            .to_string();

        let content = raw
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or("Missing text field")?
            .to_string();

        let channel_id = raw
            .get("channel")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let mut msg = NormalizedMessage::new(user_id, content, Platform::Slack, channel_id);
        if let Some(ts) = raw.get("ts") {
            msg.metadata
                .insert("slack_timestamp".to_string(), ts.clone());
        }

        Ok(msg)
    }

    fn normalize_discord(&self, raw: serde_json::Value) -> Result<NormalizedMessage, String> {
        let user_id = raw
            .get("author")
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_str())
            .ok_or("Missing author.id")?
            .to_string();

        let content = raw
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or("Missing content")?
            .to_string();

        let channel_id = raw
            .get("channel_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let mut msg = NormalizedMessage::new(user_id, content, Platform::Discord, channel_id);
        if let Some(id) = raw.get("id") {
            msg.metadata.insert("discord_message_id".to_string(), id.clone());
        }

        Ok(msg)
    }

    fn normalize_teams(&self, raw: serde_json::Value) -> Result<NormalizedMessage, String> {
        let user_id = raw
            .get("from")
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_str())
            .ok_or("Missing from.id")?
            .to_string();

        let content = raw
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or("Missing text")?
            .to_string();

        let channel_id = raw
            .get("channelId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(NormalizedMessage::new(user_id, content, Platform::Teams, channel_id))
    }

    fn normalize_telegram(&self, raw: serde_json::Value) -> Result<NormalizedMessage, String> {
        let user_id = raw
            .get("from")
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_u64())
            .ok_or("Missing from.id")?
            .to_string();

        let content = raw
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or("Missing text")?
            .to_string();

        let channel_id = raw
            .get("chat")
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            .to_string();

        Ok(NormalizedMessage::new(user_id, content, Platform::Telegram, channel_id))
    }

    fn normalize_http(&self, raw: serde_json::Value) -> Result<NormalizedMessage, String> {
        let user_id = raw
            .get("user_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing user_id")?
            .to_string();

        let content = raw
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or("Missing message")?
            .to_string();

        let channel_id = raw
            .get("channel_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        let platform = match raw.get("platform").and_then(|v| v.as_str()) {
            Some("websocket") => Platform::WebSocket,
            _ => Platform::HTTP,
        };

        Ok(NormalizedMessage::new(user_id, content, platform, channel_id))
    }
}

#[async_trait]
impl Actor for ChatGateway {
    type Message = ChatGatewayMessage;

    fn id(&self) -> ActorId {
        self.id
    }

    async fn handle(&mut self, msg: Self::Message) -> Result<bool, String> {
        match msg {
            ChatGatewayMessage::IncomingMessage { platform, raw } => {
                let start = Utc::now();
                match self.normalize(platform, raw) {
                    Ok(normalized) => {
                        let elapsed = Utc::now().signed_duration_since(start);
                        log::info!(
                            "[ChatGateway] Normalized message from {} ({}ms)",
                            platform.as_str(),
                            elapsed.num_milliseconds()
                        );

                        // Update metrics
                        let mut metrics = (*self.metrics).clone();
                        metrics.total_messages += 1;
                        *metrics
                            .messages_by_platform
                            .entry(platform.as_str().to_string())
                            .or_insert(0) += 1;
                        metrics.avg_processing_time_ms =
                            (metrics.avg_processing_time_ms + elapsed.num_milliseconds() as f64) / 2.0;
                        metrics.last_message_time = Some(Utc::now());

                        // Keep history limited
                        let mut history = (*self.message_history).clone();
                        if history.len() >= self.max_history {
                            history.remove(0);
                        }
                        history.push(normalized);

                        Ok(true)
                    }
                    Err(e) => {
                        log::error!("[ChatGateway] Normalization failed: {}", e);
                        Err(e)
                    }
                }
            }
            ChatGatewayMessage::GetMetrics => {
                log::info!("[ChatGateway] Metrics: {:?}", self.metrics);
                Ok(true)
            }
            ChatGatewayMessage::GetHistory { limit } => {
                let count = std::cmp::min(limit, self.message_history.len());
                log::info!("[ChatGateway] History: {} recent messages", count);
                Ok(true)
            }
            ChatGatewayMessage::Stop => {
                log::info!("[ChatGateway] Stop signal received");
                Ok(false)
            }
        }
    }

    async fn snapshot(&self) -> Result<Snapshot, String> {
        let state = serde_json::json!({
            "metrics": self.metrics.as_ref().clone(),
            "message_history_count": self.message_history.len(),
        });

        Ok(Snapshot::new(
            self.id,
            "ChatGateway".to_string(),
            state,
        ))
    }

    async fn restore(&mut self, _snapshot: Snapshot) -> Result<(), String> {
        log::info!("[ChatGateway] Restored from snapshot");
        Ok(())
    }

    fn actor_type(&self) -> &'static str {
        "ChatGateway"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_slack_normalization() {
        let gateway = ChatGateway::new(100);
        let raw = serde_json::json!({
            "user": "U123456",
            "text": "Hello from Slack",
            "channel": "C789012",
            "ts": "1234567890.000100"
        });

        let msg = gateway.normalize_slack(raw).unwrap();
        assert_eq!(msg.user_id, "U123456");
        assert_eq!(msg.content, "Hello from Slack");
        assert_eq!(msg.platform, Platform::Slack);
    }

    #[tokio::test]
    async fn test_discord_normalization() {
        let gateway = ChatGateway::new(100);
        let raw = serde_json::json!({
            "author": {"id": "user123"},
            "content": "Hello from Discord",
            "channel_id": "chan456"
        });

        let msg = gateway.normalize_discord(raw).unwrap();
        assert_eq!(msg.user_id, "user123");
        assert_eq!(msg.content, "Hello from Discord");
        assert_eq!(msg.platform, Platform::Discord);
    }

    #[tokio::test]
    async fn test_http_normalization() {
        let gateway = ChatGateway::new(100);
        let raw = serde_json::json!({
            "user_id": "http_user",
            "message": "Hello from HTTP",
            "channel_id": "default",
            "platform": "websocket"
        });

        let msg = gateway.normalize_http(raw).unwrap();
        assert_eq!(msg.user_id, "http_user");
        assert_eq!(msg.content, "Hello from HTTP");
        assert_eq!(msg.platform, Platform::WebSocket);
    }
}
