#![cfg(feature = "telegram")]

use std::sync::Arc;
use async_trait::async_trait;

use crate::admin_api::PlatformStates;
use crate::config::TelegramConfig;
use crate::metrics::SharedMetrics;
use crate::platforms::{InboundMessage, MessagingPlatform, ShedNotice};
use crate::router::Router;
use crate::session;

pub struct TelegramPlatform {
    pub token:           String,
    pub config:          TelegramConfig,
    pub metrics:         SharedMetrics,
    pub router:          Arc<Router>,
    pub platform_states: PlatformStates,
}

impl TelegramPlatform {
    pub fn new(
        token:           String,
        config:          TelegramConfig,
        metrics:         SharedMetrics,
        router:          Arc<Router>,
        platform_states: PlatformStates,
    ) -> Arc<Self> {
        Arc::new(Self { token, config, metrics, router, platform_states })
    }
}

#[async_trait]
impl MessagingPlatform for TelegramPlatform {
    fn name(&self) -> &'static str { "telegram" }

    async fn run(
        self: Arc<Self>,
        tx: tokio::sync::mpsc::Sender<InboundMessage>,
        shed_tx: tokio::sync::mpsc::Sender<ShedNotice>,
    ) {
        use teloxide::prelude::*;
        use teloxide::types::Update;

        self.platform_states.insert("telegram".to_string(), "connecting".to_string());

        let bot      = Bot::new(&self.token);
        let platform = self.clone();
        let platform2 = self.clone();

        let message_handler = Update::filter_message().branch(
            teloxide::dptree::endpoint(move |bot: Bot, msg: Message| {
                let tx       = tx.clone();
                let shed_tx  = shed_tx.clone();
                let platform = platform.clone();

                async move {
                    let chat_id = msg.chat.id.0;
                    let cfg = &platform.config;

                    if !cfg.allowed_chat_ids.is_empty() && !cfg.allowed_chat_ids.contains(&chat_id) {
                        platform.metrics.allowlist_denials.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        return Ok::<(), teloxide::RequestError>(());
                    }

                    let text = match msg.text() {
                        Some(t) => t.to_string(),
                        None    => return Ok::<(), teloxide::RequestError>(()),
                    };

                    let user_id = msg.from.as_ref()
                        .map(|u| u.id.to_string())
                        .unwrap_or_else(|| chat_id.to_string());

                    let display_name = msg.from.as_ref()
                        .and_then(|u| u.username.clone())
                        .unwrap_or_else(|| user_id.clone());

                    let inbound = InboundMessage {
                        platform:     "telegram".to_string(),
                        platform_id:  chat_id.to_string(),
                        user_id:      user_id.clone(),
                        display_name,
                        event_id:     msg.id.to_string(),
                        text,
                        reply_to:     None,
                    };

                    platform.metrics.messages_inbound.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                    if tx.try_send(inbound).is_err() {
                        platform.metrics.messages_queued_full.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let _ = shed_tx.try_send(ShedNotice {
                            platform: "telegram".to_string(),
                            chat_id:  chat_id.to_string(),
                            user_id,
                            reply_to: None,
                        });
                    }

                    Ok(())
                }
            })
        );

        let callback_handler = Update::filter_callback_query().branch(
            teloxide::dptree::endpoint(move |bot: Bot, cb: CallbackQuery| {
                let platform = platform2.clone();

                async move {
                    let data = match cb.data.as_deref() {
                        Some(d) => d.to_string(),
                        None    => {
                            let _ = bot.answer_callback_query(&cb.id).await;
                            return Ok(());
                        }
                    };

                    // Parse: "ca:{token}:{nonce}" approve or "cd:{token}:{nonce}" deny
                    let (approved, token, nonce) =
                        if let Some(rest) = data.strip_prefix("ca:") {
                            let (tok, n) = split_token_nonce(rest);
                            (true, tok, n)
                        } else if let Some(rest) = data.strip_prefix("cd:") {
                            let (tok, n) = split_token_nonce(rest);
                            (false, tok, n)
                        } else {
                            let _ = bot.answer_callback_query(&cb.id).await;
                            return Ok(());
                        };

                    // Validate nonce against stored value to reject stale interactions
                    let db = &platform.router.db;
                    let pending = session::load_unresolved_confirms(db).await;
                    let stored_nonce = pending.iter()
                        .find(|p| p.token == token)
                        .map(|p| p.prompt_nonce);

                    if stored_nonce != Some(nonce) {
                        // Stale — acknowledge silently
                        let _ = bot.answer_callback_query(&cb.id).await;
                        return Ok(());
                    }

                    // Acknowledge the button press
                    let _ = bot.answer_callback_query(&cb.id).await;

                    // Resolve in DB
                    let _ = session::resolve_confirm(db, token.clone()).await;
                    if approved {
                        platform.router.metrics.confirms_resolved.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }

                    // Notify Buddy
                    let reply = platform.router.send_confirm_response(&token, approved).await
                        .unwrap_or_else(|_| if approved {
                            "✅ Confirmed. Processing...".to_string()
                        } else {
                            "❌ Denied. No action taken.".to_string()
                        });

                    // Send reply to the chat where the button was pressed
                    if let Some(msg) = cb.message {
                        let chat_id = teloxide::types::ChatId(msg.chat().id.0);
                        let _ = bot.send_message(chat_id, reply).await;
                    }

                    Ok(())
                }
            })
        );

        let handler = teloxide::dptree::entry()
            .branch(message_handler)
            .branch(callback_handler);

        self.platform_states.insert("telegram".to_string(), "connected".to_string());
        Dispatcher::builder(bot, handler)
            .default_handler(|_| async {})
            .build()
            .dispatch()
            .await;
        self.platform_states.insert("telegram".to_string(), "disconnected".to_string());
    }

    async fn send_reply(
        &self,
        chat_id: &str,
        _user_id: &str,
        text: &str,
        _reply_to: Option<&str>,
    ) -> Result<(), String> {
        use teloxide::prelude::*;
        use teloxide::types::ChatId;

        let bot  = Bot::new(&self.token);
        let cid: i64 = chat_id.parse().map_err(|e| format!("chat id: {e}"))?;

        for chunk in crate::formatter::format(text, "telegram").chunks {
            bot.send_message(ChatId(cid), &chunk)
                .await
                .map_err(|e| format!("telegram send: {e}"))?;
        }
        Ok(())
    }

    async fn send_confirm_prompt(
        &self,
        chat_id: &str,
        _user_id: &str,
        token: &str,
        prompt: &str,
        nonce: i64,
    ) -> Result<String, String> {
        use teloxide::prelude::*;
        use teloxide::types::{
            ChatId, InlineKeyboardButton, InlineKeyboardMarkup,
        };

        let bot  = Bot::new(&self.token);
        let cid: i64 = chat_id.parse().map_err(|e| format!("chat id: {e}"))?;

        let approve = InlineKeyboardButton::callback("✅ Approve", format!("ca:{token}:{nonce}"));
        let deny    = InlineKeyboardButton::callback("❌ Deny",    format!("cd:{token}:{nonce}"));
        let keyboard = InlineKeyboardMarkup::new(vec![vec![approve, deny]]);

        let msg = bot.send_message(ChatId(cid), format!("⚠️ **Confirmation required**\n{prompt}"))
            .reply_markup(keyboard)
            .await
            .map_err(|e| format!("telegram confirm: {e}"))?;

        Ok(msg.id.to_string())
    }
}

fn split_token_nonce(s: &str) -> (String, i64) {
    if let Some(pos) = s.rfind(':') {
        let token = s[..pos].to_string();
        let nonce = s[pos + 1..].parse::<i64>().unwrap_or(-1);
        (token, nonce)
    } else {
        (s.to_string(), -1)
    }
}
