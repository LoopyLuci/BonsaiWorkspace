// Platform adapters for Telegram and Discord

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Platform identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    Telegram,
    Discord,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Platform::Telegram => write!(f, "telegram"),
            Platform::Discord => write!(f, "discord"),
        }
    }
}

/// Unique user ID (platform-specific)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId {
    pub platform: Platform,
    pub id: String,
}

impl UserId {
    pub fn telegram(id: impl Into<String>) -> Self {
        Self {
            platform: Platform::Telegram,
            id: id.into(),
        }
    }

    pub fn discord(id: impl Into<String>) -> Self {
        Self {
            platform: Platform::Discord,
            id: id.into(),
        }
    }
}

/// Unique message ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageId {
    pub platform: Platform,
    pub id: String,
}

/// Incoming message
#[derive(Debug, Clone)]
pub struct Message {
    pub id: MessageId,
    pub user_id: UserId,
    pub platform: Platform,
    pub text: String,
    pub timestamp: u64,
}

/// Platform adapter trait
#[async_trait]
pub trait PlatformAdapter: Send + Sync {
    fn platform(&self) -> Platform;

    async fn send_message(&self, user_id: &UserId, text: &str) -> anyhow::Result<()>;

    async fn send_message_with_buttons(
        &self,
        user_id: &UserId,
        text: &str,
        buttons: Vec<(String, String)>, // (label, callback_data)
    ) -> anyhow::Result<()>;

    async fn edit_message(
        &self,
        user_id: &UserId,
        message_id: &MessageId,
        text: &str,
    ) -> anyhow::Result<()>;

    async fn delete_message(&self, user_id: &UserId, message_id: &MessageId) -> anyhow::Result<()>;

    async fn get_user_info(&self, user_id: &UserId) -> anyhow::Result<UserInfo>;
}

/// User info from platform
#[derive(Debug, Clone)]
pub struct UserInfo {
    pub user_id: UserId,
    pub username: Option<String>,
    pub display_name: String,
    pub is_premium: bool,
}

/// Telegram adapter stub
pub struct TelegramAdapter {
    token: String,
}

impl TelegramAdapter {
    pub fn new(token: String) -> Self {
        Self { token }
    }
}

#[async_trait]
impl PlatformAdapter for TelegramAdapter {
    fn platform(&self) -> Platform {
        Platform::Telegram
    }

    async fn send_message(&self, user_id: &UserId, text: &str) -> anyhow::Result<()> {
        // In real implementation, call Telegram Bot API via reqwest
        tracing::info!("[Telegram] → {}: {}", user_id.id, text);
        Ok(())
    }

    async fn send_message_with_buttons(
        &self,
        user_id: &UserId,
        text: &str,
        buttons: Vec<(String, String)>,
    ) -> anyhow::Result<()> {
        tracing::info!("[Telegram] → {} (with {} buttons): {}", user_id.id, buttons.len(), text);
        Ok(())
    }

    async fn edit_message(
        &self,
        user_id: &UserId,
        message_id: &MessageId,
        text: &str,
    ) -> anyhow::Result<()> {
        tracing::info!("[Telegram] edit {}: {}", message_id.id, text);
        Ok(())
    }

    async fn delete_message(&self, _user_id: &UserId, message_id: &MessageId) -> anyhow::Result<()> {
        tracing::info!("[Telegram] delete {}", message_id.id);
        Ok(())
    }

    async fn get_user_info(&self, user_id: &UserId) -> anyhow::Result<UserInfo> {
        Ok(UserInfo {
            user_id: user_id.clone(),
            username: Some("user".into()),
            display_name: "Telegram User".into(),
            is_premium: false,
        })
    }
}

/// Discord adapter stub
pub struct DiscordAdapter {
    token: String,
}

impl DiscordAdapter {
    pub fn new(token: String) -> Self {
        Self { token }
    }
}

#[async_trait]
impl PlatformAdapter for DiscordAdapter {
    fn platform(&self) -> Platform {
        Platform::Discord
    }

    async fn send_message(&self, user_id: &UserId, text: &str) -> anyhow::Result<()> {
        tracing::info!("[Discord] → {}: {}", user_id.id, text);
        Ok(())
    }

    async fn send_message_with_buttons(
        &self,
        user_id: &UserId,
        text: &str,
        buttons: Vec<(String, String)>,
    ) -> anyhow::Result<()> {
        tracing::info!("[Discord] → {} (with {} buttons): {}", user_id.id, buttons.len(), text);
        Ok(())
    }

    async fn edit_message(
        &self,
        user_id: &UserId,
        message_id: &MessageId,
        text: &str,
    ) -> anyhow::Result<()> {
        tracing::info!("[Discord] edit {}: {}", message_id.id, text);
        Ok(())
    }

    async fn delete_message(&self, _user_id: &UserId, message_id: &MessageId) -> anyhow::Result<()> {
        tracing::info!("[Discord] delete {}", message_id.id);
        Ok(())
    }

    async fn get_user_info(&self, user_id: &UserId) -> anyhow::Result<UserInfo> {
        Ok(UserInfo {
            user_id: user_id.clone(),
            username: Some("user".into()),
            display_name: "Discord User".into(),
            is_premium: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id_creation() {
        let tg_id = UserId::telegram("12345");
        assert_eq!(tg_id.platform, Platform::Telegram);
        assert_eq!(tg_id.id, "12345");

        let dc_id = UserId::discord("67890");
        assert_eq!(dc_id.platform, Platform::Discord);
        assert_eq!(dc_id.id, "67890");
    }

    #[test]
    fn test_platform_display() {
        assert_eq!(Platform::Telegram.to_string(), "telegram");
        assert_eq!(Platform::Discord.to_string(), "discord");
    }
}
