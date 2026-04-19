#![cfg(feature = "matrix")]

use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::admin_api::PlatformStates;
use crate::config::MatrixConfig;
use crate::platforms::{InboundMessage, MessagingPlatform, ShedNotice};
use crate::metrics::SharedMetrics;

pub struct MatrixPlatform {
    pub password:        String,
    pub config:          MatrixConfig,
    pub metrics:         SharedMetrics,
    pub platform_states: PlatformStates,
    // Shared client — set once by run(), reused by send_reply() / send_confirm_prompt()
    client: Arc<RwLock<Option<matrix_sdk::Client>>>,
}

impl MatrixPlatform {
    pub fn new(
        password:        String,
        config:          MatrixConfig,
        metrics:         SharedMetrics,
        platform_states: PlatformStates,
    ) -> Arc<Self> {
        Arc::new(Self {
            password,
            config,
            metrics,
            platform_states,
            client: Arc::new(RwLock::new(None)),
        })
    }

    async fn get_or_init_client(&self) -> Option<matrix_sdk::Client> {
        // Fast-path: return existing client if present
        {
            let guard = self.client.read().await;
            if guard.is_some() {
                return guard.clone();
            }
        }

        // Acquire the write lock so only one initializer runs at a time.
        let mut guard = self.client.write().await;
        if guard.is_some() {
            return guard.clone();
        }

        // Not yet initialized — log in now while holding the write lock to
        // prevent concurrent double-logins.
        use matrix_sdk::Client;
        let homeserver = self.config.homeserver_url.parse().ok()?;
        let client = Client::new(homeserver).await.ok()?;
        if client.matrix_auth()
            .login_username(&self.config.username, &self.password)
            .initial_device_display_name("Bonsai Bot")
            .await
            .is_err()
        {
            return None;
        }

        *guard = Some(client.clone());
        Some(client)
    }
}

#[async_trait]
impl MessagingPlatform for MatrixPlatform {
    fn name(&self) -> &'static str { "matrix" }

    async fn run(
        self: Arc<Self>,
        tx: tokio::sync::mpsc::Sender<InboundMessage>,
        shed_tx: tokio::sync::mpsc::Sender<ShedNotice>,
    ) {
        use matrix_sdk::{config::SyncSettings, room::Room};
        use matrix_sdk::ruma::events::room::message::{
            MessageType, OriginalSyncRoomMessageEvent,
        };

        self.platform_states.insert("matrix".to_string(), "connecting".to_string());
        let client = match self.get_or_init_client().await {
            Some(c) => c,
            None => {
                tracing::error!("[matrix] Login failed — platform disabled");
                self.platform_states.insert("matrix".to_string(), "error".to_string());
                return;
            }
        };
        tracing::info!("[matrix] Logged in as {}", self.config.username);
        self.platform_states.insert("matrix".to_string(), "connected".to_string());

        let platform = self.clone();

        client.add_event_handler(
            move |ev: OriginalSyncRoomMessageEvent, room: Room| {
                let platform = platform.clone();
                let tx       = tx.clone();
                let shed_tx  = shed_tx.clone();

                async move {
                    let cfg = &platform.config;

                    if !cfg.allowed_rooms.is_empty() {
                        let room_id = room.room_id().to_string();
                        if !cfg.allowed_rooms.contains(&room_id) { return; }
                    }

                    let sender = ev.sender.to_string();
                    if !cfg.allowed_users.is_empty() && !cfg.allowed_users.contains(&sender) {
                        platform.metrics.allowlist_denials.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        return;
                    }

                    let text = match &ev.content.msgtype {
                        MessageType::Text(t) => t.body.clone(),
                        _ => return,
                    };

                    let room_id  = room.room_id().to_string();
                    let event_id = ev.event_id.to_string();

                    let inbound = InboundMessage {
                        platform:     "matrix".to_string(),
                        platform_id:  room_id.clone(),
                        user_id:      sender.clone(),
                        display_name: sender.clone(),
                        event_id,
                        text,
                        reply_to:     None,
                    };

                    platform.metrics.messages_inbound.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                    if tx.try_send(inbound).is_err() {
                        platform.metrics.messages_queued_full.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let _ = shed_tx.try_send(ShedNotice {
                            platform: "matrix".to_string(),
                            chat_id:  room_id,
                            user_id:  sender,
                            reply_to: None,
                        });
                    }
                }
            },
        );

        if let Err(e) = client.sync(SyncSettings::default()).await {
            tracing::error!("[matrix] Sync error: {e}");
        }
    }

    async fn send_reply(
        &self,
        chat_id: &str,
        _user_id: &str,
        text: &str,
        _reply_to: Option<&str>,
    ) -> Result<(), String> {
        use matrix_sdk::ruma::RoomId;
        use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;

        let client = self.get_or_init_client().await
            .ok_or_else(|| "matrix: client not available".to_string())?;

        let room_id = RoomId::parse(chat_id).map_err(|e| format!("room id: {e}"))?;
        let room    = client.get_room(&room_id).ok_or("matrix: room not found")?;

        let html    = crate::formatter::format(text, "matrix").chunks.join("\n");
        let content = RoomMessageEventContent::text_html(text, html);

        room.send(content).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn send_confirm_prompt(
        &self,
        chat_id: &str,
        user_id: &str,
        token: &str,
        prompt: &str,
        nonce: i64,
    ) -> Result<String, String> {
        let text = format!(
            "⚠️ Confirmation required (ref:{token}:{nonce})\n{prompt}\n\nReply 'yes' to approve or 'no' to deny (expires in 2 minutes)."
        );
        self.send_reply(chat_id, user_id, &text, None).await?;
        Ok(format!("{token}:{nonce}"))
    }
}
