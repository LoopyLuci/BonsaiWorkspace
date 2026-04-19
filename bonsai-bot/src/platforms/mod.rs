#[cfg(feature = "discord")]
pub mod discord;
#[cfg(feature = "telegram")]
pub mod telegram;
#[cfg(feature = "email")]
pub mod email;
#[cfg(feature = "matrix")]
pub mod matrix;

use async_trait::async_trait;
use std::sync::Arc;

// ── Inbound message ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct InboundMessage {
    pub platform:    String,
    pub platform_id: String,  // guild_id/chat_id/room_id
    pub user_id:     String,
    pub display_name: String,
    pub event_id:    String,
    pub text:        String,
    /// For email only
    pub reply_to:    Option<String>,
}

// ── Platform trait ────────────────────────────────────────────────────────────

#[async_trait]
pub trait MessagingPlatform: Send + Sync {
    fn name(&self) -> &'static str;

    /// Start ingest loop, sending inbound messages to `tx`.
    async fn run(
        self: Arc<Self>,
        tx: tokio::sync::mpsc::Sender<InboundMessage>,
        shed_tx: tokio::sync::mpsc::Sender<ShedNotice>,
    );

    /// Send a text reply to the given platform chat/user.
    async fn send_reply(
        &self,
        chat_id: &str,
        user_id: &str,
        text: &str,
        reply_to: Option<&str>,
    ) -> Result<(), String>;

    /// Send a confirmation prompt and return the message ID (for button tracking).
    async fn send_confirm_prompt(
        &self,
        chat_id: &str,
        user_id: &str,
        token: &str,
        prompt: &str,
        nonce: i64,
    ) -> Result<String, String>;
}

// ── Shed notice ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ShedNotice {
    pub platform: String,
    pub chat_id:  String,
    pub user_id:  String,
    pub reply_to: Option<String>,
}
