use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::num::NonZeroU32;
use dashmap::DashMap;
use governor::{Quota, RateLimiter, state::direct::NotKeyed, state::InMemoryState, clock::DefaultClock};
use serde_json::{json, Value};
use crate::session::Db;

use crate::buddy_client::BuddyClient;
use crate::config::BotConfig;
use crate::dedup::DedupCache;
use crate::metrics::SharedMetrics;
use crate::platforms::{InboundMessage, MessagingPlatform};
use crate::sanitizer::sanitize;
use crate::session;

fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}

type UserLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

#[allow(dead_code)]
pub struct Router {
    pub buddy:   Arc<BuddyClient>,
    pub dedup:   Arc<DedupCache>,
    pub db:      Db,
    pub metrics: SharedMetrics,
    pub config:  BotConfig,
    // Stage 3: per-user token-bucket rate limiters (10 msgs / 60 s per user)
    rate_limiters: DashMap<String, UserLimiter>,
}

impl Router {
    pub fn new(
        buddy: Arc<BuddyClient>,
        dedup: Arc<DedupCache>,
        db: Db,
        metrics: SharedMetrics,
        config: BotConfig,
    ) -> Self {
        Self { buddy, dedup, db, metrics, config, rate_limiters: DashMap::new() }
    }

    fn rate_limiter_for(&self, key: &str) -> UserLimiter {
        self.rate_limiters
            .entry(key.to_string())
            .or_insert_with(|| {
                Arc::new(RateLimiter::direct(
                    Quota::per_minute(NonZeroU32::new(10).unwrap()),
                ))
            })
            .clone()
    }

    pub async fn handle(
        &self,
        msg: InboundMessage,
        platform: &Arc<dyn MessagingPlatform>,
    ) {
        let req_id = uuid::Uuid::new_v4();

        // Stage 1: Dedup
        if self.dedup.is_duplicate(&msg.platform, &msg.event_id) {
            self.metrics.dedup_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            tracing::debug!(req_id=%req_id, platform=%msg.platform, event_id=%msg.event_id, "dedup hit");
            return;
        }

        tracing::info!(req_id=%req_id, platform=%msg.platform, user=%msg.user_id, event=%msg.event_id, "inbound");

        // Stage 3: Rate limit (token bucket — 10 msgs/min per platform:user)
        let rate_key = format!("{}:{}", msg.platform, msg.user_id);
        if self.rate_limiter_for(&rate_key).check().is_err() {
            self.metrics.rate_limit_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            tracing::warn!(req_id=%req_id, platform=%msg.platform, user=%msg.user_id, "rate limited");
            let _ = platform.send_reply(
                &msg.platform_id, &msg.user_id,
                "⏳ Rate limit exceeded. Please wait before sending more messages.",
                msg.reply_to.as_deref(),
            ).await;
            return;
        }

        // Stage 4: Sanitize
        let clean_text = match sanitize(&msg.text, &self.metrics) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(req_id=%req_id, platform=%msg.platform, user=%msg.user_id, reason=%e, "sanitize rejected");
                let _ = platform.send_reply(&msg.platform_id, &msg.user_id,
                    "⚠️ Your message could not be processed safely.", msg.reply_to.as_deref()).await;
                return;
            }
        };

        // Stage 5: Session resolution
        let buddy_session = match session::find_active_session(
            &self.db,
            msg.platform.clone(), msg.user_id.clone(), msg.platform_id.clone(),
        ).await {
            Some(id) => id,
            None => {
                let new_id = uuid::Uuid::new_v4().to_string();
                let _ = session::upsert_session(
                    &self.db,
                    msg.platform.clone(), msg.user_id.clone(), msg.platform_id.clone(),
                    msg.display_name.clone(), new_id.clone(),
                ).await;
                new_id
            }
        };

        // Stage 5b: Text-based confirm resolution (for email/matrix/plain-text platforms)
        // Check if the user is responding to a pending confirmation with "yes" or "no".
        let trimmed = clean_text.trim().to_lowercase();
        if trimmed == "yes" || trimmed == "no" || trimmed == "y" || trimmed == "n" {
            if let Some(reply) = self.try_resolve_confirm_by_text(
                &msg, &trimmed, platform,
            ).await {
                let _ = platform.send_reply(
                    &msg.platform_id, &msg.user_id, &reply, msg.reply_to.as_deref(),
                ).await;
                session::touch_session(&self.db, msg.platform.clone(), msg.user_id.clone(), msg.platform_id.clone()).await;
                self.metrics.messages_processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return;
            }
            // No pending confirm found — fall through to normal Buddy call
        }

        // Stage 6: Buddy call
        let messages = vec![json!({
            "role": "user",
            "content": clean_text,
            "_bonsai_session": buddy_session,
        })];

        tracing::debug!(req_id=%req_id, "calling buddy");
        let response = match self.buddy.chat(BuddyClient::build_request(messages, None)).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(req_id=%req_id, error=%e, "buddy call failed");
                let reply = if e == "circuit_open" {
                    "⚠️ Bonsai is currently unavailable. Try again shortly."
                } else {
                    "⚠️ Bonsai error. Please retry."
                };
                let _ = platform.send_reply(&msg.platform_id, &msg.user_id, reply, msg.reply_to.as_deref()).await;
                return;
            }
        };

        // Stage 7: Confirmation gate
        let finish_reason = response["choices"][0]["finish_reason"].as_str().unwrap_or("stop");
        if finish_reason == "tool_calls_pending_approval" {
            if let Some(ext) = response.get("bonsai_ext") {
                if ext.get("type").and_then(|v| v.as_str()) == Some("confirm_required") {
                    tracing::info!(req_id=%req_id, "confirm gate triggered");
                    self.handle_confirm(ext, &msg, platform).await;
                    return;
                }
            }
        }

        // Stage 8: Reply
        let reply_text = response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("(no response)");

        tracing::info!(req_id=%req_id, platform=%msg.platform, user=%msg.user_id, "reply sent");
        let _ = platform.send_reply(&msg.platform_id, &msg.user_id, reply_text, msg.reply_to.as_deref()).await;

        session::touch_session(&self.db, msg.platform.clone(), msg.user_id.clone(), msg.platform_id.clone()).await;
        self.metrics.messages_processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Like `handle`, but uses the streaming Buddy API path. Accumulates tokens and
    /// sends a single reply when complete (platforms can override to send chunks).
    pub async fn handle_streaming(
        &self,
        msg: InboundMessage,
        platform: &Arc<dyn MessagingPlatform>,
    ) {
        let req_id = uuid::Uuid::new_v4();

        if self.dedup.is_duplicate(&msg.platform, &msg.event_id) {
            self.metrics.dedup_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return;
        }

        let rate_key = format!("{}:{}", msg.platform, msg.user_id);
        if self.rate_limiter_for(&rate_key).check().is_err() {
            self.metrics.rate_limit_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let _ = platform.send_reply(&msg.platform_id, &msg.user_id,
                "⏳ Rate limit exceeded.", msg.reply_to.as_deref()).await;
            return;
        }

        let clean_text = match crate::sanitizer::sanitize(&msg.text, &self.metrics) {
            Ok(t) => t,
            Err(_) => {
                let _ = platform.send_reply(&msg.platform_id, &msg.user_id,
                    "⚠️ Your message could not be processed safely.", msg.reply_to.as_deref()).await;
                return;
            }
        };

        let buddy_session = match session::find_active_session(
            &self.db, msg.platform.clone(), msg.user_id.clone(), msg.platform_id.clone(),
        ).await {
            Some(id) => id,
            None => {
                let new_id = uuid::Uuid::new_v4().to_string();
                let _ = session::upsert_session(&self.db,
                    msg.platform.clone(), msg.user_id.clone(), msg.platform_id.clone(),
                    msg.display_name.clone(), new_id.clone()).await;
                new_id
            }
        };

        let messages = vec![json!({
            "role": "user",
            "content": clean_text,
            "_bonsai_session": buddy_session,
        })];

        let mut rx = match self.buddy.chat_stream(BuddyClient::build_request(messages, None)).await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(req_id=%req_id, error=%e, "buddy stream failed");
                let _ = platform.send_reply(&msg.platform_id, &msg.user_id,
                    "⚠️ Bonsai error. Please retry.", msg.reply_to.as_deref()).await;
                return;
            }
        };

        // Accumulate all stream tokens then send as one reply
        let mut full = String::new();
        while let Some(token) = rx.recv().await {
            full.push_str(&token);
        }

        if full.is_empty() { full = "(no response)".to_string(); }

        tracing::info!(req_id=%req_id, platform=%msg.platform, user=%msg.user_id, "stream reply sent");
        let _ = platform.send_reply(&msg.platform_id, &msg.user_id, &full, msg.reply_to.as_deref()).await;
        session::touch_session(&self.db, msg.platform.clone(), msg.user_id.clone(), msg.platform_id.clone()).await;
        self.metrics.messages_processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Send a structured confirm_response to Buddy, return Buddy's reply text.
    pub async fn send_confirm_response(
        &self,
        token: &str,
        approved: bool,
    ) -> Result<String, String> {
        let messages = vec![json!({
            "role": "user",
            "content": "__bot_confirm__",
            "bonsai_ext": {
                "schema": 1,
                "type": "confirm_response",
                "token": token,
                "approved": approved,
            }
        })];
        let resp = self.buddy.chat(BuddyClient::build_request(messages, None)).await?;
        Ok(resp["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or(if approved { "✅ Action confirmed and processing." } else { "❌ Action denied." })
            .to_string())
    }

    /// Check if the user has a pending confirmation and resolve it with their "yes"/"no" text.
    /// Returns the reply to send back, or `None` if no pending confirm was found.
    async fn try_resolve_confirm_by_text(
        &self,
        msg: &InboundMessage,
        text: &str,
        _platform: &Arc<dyn MessagingPlatform>,
    ) -> Option<String> {
        let pending = session::load_unresolved_confirms(&self.db).await;
        let my_pending = pending.into_iter().find(|p| {
            p.platform == msg.platform && p.user_id == msg.user_id
        })?;

        let approved = matches!(text, "yes" | "y");
        let token = my_pending.token.clone();

        // Resolve in our DB
        let _ = session::resolve_confirm(&self.db, token.clone()).await;

        if approved {
            self.metrics.confirms_resolved.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        // Send confirm_response to Buddy
        match self.send_confirm_response(&token, approved).await {
            Ok(reply) => Some(reply),
            Err(_) => Some(if approved {
                "✅ Confirmed. Please resend your original request.".to_string()
            } else {
                "❌ Denied. No action taken.".to_string()
            }),
        }
    }

    async fn handle_confirm(
        &self,
        ext: &Value,
        msg: &InboundMessage,
        platform: &Arc<dyn MessagingPlatform>,
    ) {
        let token      = ext["token"].as_str().unwrap_or_default();
        let tool       = ext["tool"].as_str().unwrap_or_default();
        let prompt     = ext["prompt"].as_str().unwrap_or_default();
        let expires_at = ext["expires_at"].as_i64().unwrap_or(0);
        let args_json  = ext["args"].to_string();

        if token.is_empty() || expires_at < now_secs() {
            let _ = platform.send_reply(
                &msg.platform_id, &msg.user_id,
                "⏰ Confirmation expired or invalid. Please resend your request.",
                msg.reply_to.as_deref(),
            ).await;
            return;
        }

        let _ = session::insert_confirm(
            &self.db,
            token.to_string(), msg.platform.clone(), msg.platform_id.clone(), msg.user_id.clone(),
            tool.to_string(), args_json, prompt.to_string(), expires_at,
        ).await;

        self.metrics.confirms_created.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        match session::mark_prompted(&self.db, token.to_string()).await {
            Ok(nonce) => {
                let _ = platform.send_confirm_prompt(
                    &msg.platform_id, &msg.user_id, token, prompt, nonce,
                ).await;
            }
            Err(e) => tracing::error!("[router] mark_prompted failed: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    /// Rate limiter allows 10 per minute, rejects the 11th
    #[test]
    fn rate_limiter_allows_10_rejects_11th() {
        // Test governor rate limiter quota directly
        use governor::{Quota, RateLimiter};
        use std::num::NonZeroU32;

        let limiter = RateLimiter::direct(Quota::per_minute(NonZeroU32::new(10).unwrap()));

        // First 10 should succeed
        for i in 0..10 {
            assert!(limiter.check().is_ok(), "message {} should be allowed", i + 1);
        }
        // 11th must fail
        assert!(limiter.check().is_err(), "11th message should be rate-limited");
    }
}
