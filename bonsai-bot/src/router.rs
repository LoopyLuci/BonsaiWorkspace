use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
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
use crate::mgmt_client::MgmtClient;
use crate::platforms::{InboundMessage, MessagingPlatform};
use crate::sanitizer::sanitize;
use crate::session;

fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}

type UserLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;
// (limiter, last_access_epoch_secs)
type LimiterEntry = (UserLimiter, Arc<AtomicU64>);

#[allow(dead_code)]
pub struct Router {
    pub buddy:   Arc<BuddyClient>,
    pub mgmt:    Arc<MgmtClient>,
    pub dedup:   Arc<DedupCache>,
    pub db:      Db,
    pub metrics: SharedMetrics,
    pub config:  BotConfig,
    // Stage 3: per-user token-bucket rate limiters (10 msgs / 60 s per user)
    rate_limiters: DashMap<String, LimiterEntry>,
}

impl Router {
    pub fn new(
        buddy: Arc<BuddyClient>,
        mgmt: Arc<MgmtClient>,
        dedup: Arc<DedupCache>,
        db: Db,
        metrics: SharedMetrics,
        config: BotConfig,
    ) -> Self {
        Self { buddy, mgmt, dedup, db, metrics, config, rate_limiters: DashMap::new() }
    }

    /// Remove rate limiter entries not accessed in the last `stale_secs` seconds.
    pub fn evict_stale_limiters(&self, stale_secs: u64) {
        let cutoff = now_secs() as u64 - stale_secs;
        let before = self.rate_limiters.len();
        self.rate_limiters.retain(|_, (_, last)| last.load(AtomicOrdering::Relaxed) >= cutoff);
        let evicted = before - self.rate_limiters.len();
        if evicted > 0 {
            tracing::info!(evicted, "rate limiter stale entries evicted");
        }
    }

    fn rate_limiter_for(&self, key: &str) -> UserLimiter {
        let entry = self.rate_limiters
            .entry(key.to_string())
            .or_insert_with(|| {
                let limiter = Arc::new(RateLimiter::direct(
                    Quota::per_minute(NonZeroU32::new(10).unwrap()),
                ));
                let last_access = Arc::new(AtomicU64::new(now_secs() as u64));
                (limiter, last_access)
            });
        // Update last-access timestamp on each use
        entry.1.store(now_secs() as u64, AtomicOrdering::Relaxed);
        entry.0.clone()
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

        // Stage 4b: Slash command dispatch
        if clean_text.starts_with('/') {
            let reply = self.handle_slash(&clean_text, &msg).await;
            let _ = platform.send_reply(&msg.platform_id, &msg.user_id, &reply, msg.reply_to.as_deref()).await;
            self.metrics.messages_processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return;
        }

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

        // Stage 6: Buddy call (with direct llama-server fallback on circuit-open)
        let messages = vec![json!({
            "role": "user",
            "content": clean_text,
            "_bonsai_session": buddy_session,
        })];

        tracing::debug!(req_id=%req_id, "calling buddy");
        let buddy_body = self.buddy.build_request_auto(messages.clone()).await;
        let response = match self.buddy.chat(buddy_body.clone()).await {
            Ok(v) => v,
            Err(e) if e == "circuit_open" => {
                tracing::warn!(req_id=%req_id, "buddy circuit open — trying local llama-server");
                match self.buddy.chat_local(buddy_body).await {
                    Ok(v) => v,
                    Err(le) => {
                        tracing::warn!(req_id=%req_id, error=%le, "local fallback also failed");
                        let _ = platform.send_reply(&msg.platform_id, &msg.user_id,
                            "⚠️ Bonsai is currently unavailable. Try again shortly.",
                            msg.reply_to.as_deref()).await;
                        return;
                    }
                }
            }
            Err(e) => {
                tracing::warn!(req_id=%req_id, error=%e, "buddy call failed");
                let _ = platform.send_reply(&msg.platform_id, &msg.user_id,
                    "⚠️ Bonsai error. Please retry.", msg.reply_to.as_deref()).await;
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

    // ── Slash commands ─────────────────────────────────────────────────────────

    async fn handle_slash(&self, text: &str, _msg: &InboundMessage) -> String {
        let parts: Vec<&str> = text.splitn(3, ' ').collect();
        let cmd  = parts[0].to_lowercase();
        let arg1 = parts.get(1).copied().unwrap_or("").trim();
        let rest = parts.get(2).copied().unwrap_or("").trim();

        match cmd.as_str() {
            "/swarm" => {
                let prompt = if arg1.is_empty() {
                    return "Usage: /swarm <your prompt>".to_string();
                } else if rest.is_empty() {
                    arg1.to_string()
                } else {
                    format!("{arg1} {rest}")
                };
                match self.mgmt.swarm_submit(&prompt).await {
                    Ok(v) => v["final_content"].as_str().unwrap_or("(no response)").to_string(),
                    Err(e) => format!("⚠️ Swarm error: {e}"),
                }
            }

            "/agents" => match self.mgmt.list_agents().await {
                Err(e) => format!("⚠️ Could not reach workspace: {e}"),
                Ok(v) => {
                    let agents = v["agents"].as_array().cloned().unwrap_or_default();
                    if agents.is_empty() {
                        return "No agents registered.".to_string();
                    }
                    let lines: Vec<String> = agents.iter().map(|a| {
                        let id   = a["id"].as_str().unwrap_or("?");
                        let name = a["name"].as_str().unwrap_or(id);
                        let desc = a["description"].as_str().unwrap_or("");
                        format!("• {name} ({id}) — {desc}")
                    }).collect();
                    format!("Registered agents:\n{}", lines.join("\n"))
                }
            }

            "/agent" => {
                if arg1.is_empty() || rest.is_empty() {
                    return "Usage: /agent <agent-id> <message>".to_string();
                }
                match self.mgmt.agent_message(arg1, rest).await {
                    Ok(v) => v["content"].as_str().unwrap_or("(no content)").to_string(),
                    Err(e) => format!("⚠️ Agent error: {e}"),
                }
            }

            "/features" => match self.mgmt.get_features().await {
                Err(e) => format!("⚠️ Could not reach workspace: {e}"),
                Ok(v) => {
                    let obj = v.as_object().cloned().unwrap_or_default();
                    if obj.is_empty() {
                        return "No feature flags found.".to_string();
                    }
                    let mut flags: Vec<String> = obj.iter().map(|(k, v)| {
                        let on = v.as_bool().unwrap_or(false);
                        format!("{} {}", if on { "✅" } else { "❌" }, k.replace('_', " "))
                    }).collect();
                    flags.sort();
                    format!("Feature flags:\n{}", flags.join("\n"))
                }
            }

            "/model" | "/models" => match self.mgmt.list_models().await {
                Err(e) => format!("⚠️ Could not reach workspace: {e}"),
                Ok(v) => {
                    let models = v["models"].as_array().cloned().unwrap_or_default();
                    if models.is_empty() {
                        return "No models found.".to_string();
                    }
                    let lines: Vec<String> = models.iter()
                        .filter(|m| m["valid"].as_bool().unwrap_or(false))
                        .map(|m| {
                            let name = m["name"].as_str().unwrap_or("?");
                            let ram  = m["ram_required_mb"].as_u64().unwrap_or(0);
                            format!("• {name} (~{ram} MB)")
                        })
                        .collect();
                    format!("Available models ({}):\n{}", lines.len(), lines.join("\n"))
                }
            }

            "/queue" => match self.mgmt.queue_status().await {
                Err(e) => format!("⚠️ Could not reach workspace: {e}"),
                Ok(v) => {
                    let active  = v["active_total"].as_u64().unwrap_or(0);
                    let pending = v["pending_total"].as_u64().unwrap_or(0);
                    let cpu     = v["cpu_pct"].as_f64().unwrap_or(0.0);
                    let ram_mb  = v["free_ram_mb"].as_u64().unwrap_or(0);
                    format!("Queue: {active} active, {pending} pending | CPU {cpu:.0}% | Free RAM {ram_mb} MB")
                }
            }

            // ── Chess commands ─────────────────────────────────────────────────
            "/chess" => {
                match arg1 {
                    "new" | "" => {
                        // /chess new [white|black] [normal|strong]
                        let parts2: Vec<&str> = rest.splitn(2, ' ').collect();
                        let color    = parts2.first().copied().unwrap_or("white");
                        let strength = parts2.get(1).copied().unwrap_or("interactive");
                        let color    = if color == "black" { "black" } else { "white" };
                        let strength = if strength == "strong" { "strong" } else { "interactive" };
                        match self.mgmt.chess_new(&msg.display_name, color, strength).await {
                            Ok(v) => {
                                let id = v["game_id"].as_str().unwrap_or("?");
                                format!("♟ New chess game started!\nGame ID: `{id}`\nYou play as **{color}** ({strength}).\nMake moves with: /chess move {id} <notation> (e.g. e2e4)")
                            }
                            Err(e) => format!("⚠️ Chess error: {e}"),
                        }
                    }
                    "move" => {
                        // /chess move <game-id> <notation>
                        let parts2: Vec<&str> = rest.splitn(2, ' ').collect();
                        let game_id  = parts2.first().copied().unwrap_or("");
                        let notation = parts2.get(1).copied().unwrap_or("").trim();
                        if game_id.is_empty() || notation.is_empty() {
                            return "/chess move <game-id> <uci-move>  e.g. /chess move abc123 e2e4".to_string();
                        }
                        match self.mgmt.chess_move(game_id, notation).await {
                            Ok(v) => {
                                let fen    = v["fen"].as_str().unwrap_or("?");
                                let ai_mv  = v["ai_move"].as_str().unwrap_or("(thinking…)");
                                let result = v["result"].as_str().unwrap_or("ongoing");
                                if result != "ongoing" {
                                    format!("Game over — **{result}**\nPosition: `{fen}`")
                                } else {
                                    format!("✅ You played **{notation}** → BonsAI replied **{ai_mv}**\n`{fen}`")
                                }
                            }
                            Err(e) => format!("⚠️ Chess error: {e}"),
                        }
                    }
                    "resign" => {
                        let game_id = rest.trim();
                        if game_id.is_empty() {
                            return "/chess resign <game-id>".to_string();
                        }
                        match self.mgmt.chess_resign(game_id).await {
                            Ok(_) => "🏳 You resigned. Better luck next time!".to_string(),
                            Err(e) => format!("⚠️ Chess error: {e}"),
                        }
                    }
                    "status" => {
                        let game_id = rest.trim();
                        if game_id.is_empty() {
                            return "/chess status <game-id>".to_string();
                        }
                        match self.mgmt.chess_status(game_id).await {
                            Ok(v) => {
                                let fen    = v["fen"].as_str().unwrap_or("?");
                                let turn   = v["turn"].as_str().unwrap_or("?");
                                let result = v["result"].as_str().unwrap_or("ongoing");
                                format!("Chess game `{game_id}`\nTurn: **{turn}** | Result: {result}\n`{fen}`")
                            }
                            Err(e) => format!("⚠️ Chess error: {e}"),
                        }
                    }
                    _ => "Chess commands:\n  /chess new [white|black] [normal|strong]\n  /chess move <id> <uci>\n  /chess resign <id>\n  /chess status <id>".to_string(),
                }
            }

            // ── Go commands ────────────────────────────────────────────────────
            "/go" => {
                match arg1 {
                    "new" | "" => {
                        // /go new [9|13|19] [black|white]
                        let parts2: Vec<&str> = rest.splitn(2, ' ').collect();
                        let size_str = parts2.first().copied().unwrap_or("19");
                        let color    = parts2.get(1).copied().unwrap_or("black");
                        let size: u8 = size_str.parse().unwrap_or(19);
                        let size     = if [9, 13, 19].contains(&size) { size } else { 19 };
                        let color    = if color == "white" { "white" } else { "black" };
                        match self.mgmt.go_new(&msg.display_name, color, size, 7.5).await {
                            Ok(v) => {
                                let id = v["game_id"].as_str().unwrap_or("?");
                                format!("⚫ New {size}×{size} Go game started!\nGame ID: `{id}`\nYou play as **{color}** (komi 7.5).\nMake moves with: /go move {id} <GTP> (e.g. D4)")
                            }
                            Err(e) => format!("⚠️ Go error: {e}"),
                        }
                    }
                    "move" => {
                        let parts2: Vec<&str> = rest.splitn(2, ' ').collect();
                        let game_id = parts2.first().copied().unwrap_or("");
                        let gtp     = parts2.get(1).copied().unwrap_or("").trim();
                        if game_id.is_empty() || gtp.is_empty() {
                            return "/go move <game-id> <GTP>  e.g. /go move abc123 D4".to_string();
                        }
                        match self.mgmt.go_move(game_id, gtp).await {
                            Ok(v) => {
                                let ai_mv  = v["ai_move"].as_str().unwrap_or("(thinking…)");
                                let result = v["result"].as_str().unwrap_or("ongoing");
                                if result != "ongoing" {
                                    format!("Game over — **{result}**")
                                } else {
                                    format!("✅ You played **{gtp}** → BonsAI replied **{ai_mv}**")
                                }
                            }
                            Err(e) => format!("⚠️ Go error: {e}"),
                        }
                    }
                    "pass" => {
                        let game_id = rest.trim();
                        if game_id.is_empty() {
                            return "/go pass <game-id>".to_string();
                        }
                        match self.mgmt.go_move(game_id, "pass").await {
                            Ok(v) => {
                                let ai_mv = v["ai_move"].as_str().unwrap_or("(thinking…)");
                                format!("⏸ You passed → BonsAI played **{ai_mv}**")
                            }
                            Err(e) => format!("⚠️ Go error: {e}"),
                        }
                    }
                    "resign" => {
                        let game_id = rest.trim();
                        if game_id.is_empty() {
                            return "/go resign <game-id>".to_string();
                        }
                        match self.mgmt.go_resign(game_id).await {
                            Ok(_) => "🏳 You resigned. Better luck next time!".to_string(),
                            Err(e) => format!("⚠️ Go error: {e}"),
                        }
                    }
                    _ => "Go commands:\n  /go new [9|13|19] [black|white]\n  /go move <id> <GTP>\n  /go pass <id>\n  /go resign <id>".to_string(),
                }
            }

            // ── Puzzle commands ────────────────────────────────────────────────
            "/puzzle" => {
                match arg1 {
                    "" | "daily" => {
                        match self.mgmt.puzzle_daily().await {
                            Ok(v) => {
                                let id    = v["id"].as_str().unwrap_or("?");
                                let desc  = v["description"].as_str().unwrap_or("Find the best move.");
                                let fen   = v["fen"].as_str().unwrap_or("");
                                let hint  = v["hint"].as_str().unwrap_or("");
                                format!("🧩 Daily Puzzle (ID: {id})\n{desc}\n`{fen}`\nHint: _{hint}_\nGuess with: /puzzle guess {id} <uci-move>")
                            }
                            Err(e) => format!("⚠️ Puzzle error: {e}"),
                        }
                    }
                    "guess" => {
                        let parts2: Vec<&str> = rest.splitn(2, ' ').collect();
                        let puzzle_id = parts2.first().copied().unwrap_or("");
                        let uci_move  = parts2.get(1).copied().unwrap_or("").trim();
                        if puzzle_id.is_empty() || uci_move.is_empty() {
                            return "/puzzle guess <puzzle-id> <uci-move>".to_string();
                        }
                        match self.mgmt.puzzle_check(puzzle_id, uci_move).await {
                            Ok(v) => {
                                let status = v["status"].as_str().unwrap_or("?");
                                let msg_txt = v["message"].as_str().unwrap_or("");
                                match status {
                                    "solved"  => format!("🎉 Solved! {msg_txt}"),
                                    "correct" => format!("✅ Correct! {msg_txt}"),
                                    "wrong"   => {
                                        let hint = v["hint"].as_str().unwrap_or("Keep trying!");
                                        format!("❌ Not quite. Hint: _{hint}_")
                                    }
                                    _ => "⚠️ Error checking move.".to_string(),
                                }
                            }
                            Err(e) => format!("⚠️ Puzzle error: {e}"),
                        }
                    }
                    _ => "Puzzle commands:\n  /puzzle daily\n  /puzzle guess <id> <uci-move>".to_string(),
                }
            }

            // ── Tournament commands ────────────────────────────────────────────
            "/tournament" | "/tourney" => {
                match arg1 {
                    "" | "list" => {
                        match self.mgmt.tournament_list().await {
                            Ok(v) => {
                                let list = v.as_array().cloned().unwrap_or_default();
                                if list.is_empty() {
                                    "No tournaments yet. Create one with: /tournament new <name> <agent1,agent2>".to_string()
                                } else {
                                    let lines: Vec<String> = list.iter().map(|t| {
                                        let name  = t["name"].as_str().unwrap_or("?");
                                        let state = t["state"].as_str().unwrap_or("?");
                                        let n     = t["participants"].as_array().map(|a| a.len()).unwrap_or(0);
                                        format!("• **{name}** — {state} ({n} players)")
                                    }).collect();
                                    format!("Tournaments:\n{}", lines.join("\n"))
                                }
                            }
                            Err(e) => format!("⚠️ Tournament error: {e}"),
                        }
                    }
                    "new" | "create" => {
                        // /tournament new <name> <agent1,agent2,...>
                        let parts2: Vec<&str> = rest.splitn(2, ' ').collect();
                        let name   = parts2.first().copied().unwrap_or("").trim();
                        let agents = parts2.get(1).copied().unwrap_or("").trim();
                        if name.is_empty() || agents.is_empty() {
                            return "/tournament new <name> <agent1,agent2,...>".to_string();
                        }
                        let agent_list: Vec<&str> = agents.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
                        match self.mgmt.tournament_create(name, &agent_list).await {
                            Ok(v) => {
                                let id = v["tournament_id"].as_str().unwrap_or("?");
                                format!("🏆 Tournament **{name}** created (ID: `{id}`) with {} participants!", agent_list.len())
                            }
                            Err(e) => format!("⚠️ Tournament error: {e}"),
                        }
                    }
                    _ => "Tournament commands:\n  /tournament list\n  /tournament new <name> <agent1,agent2>".to_string(),
                }
            }

            "/help" => concat!(
                "Bonsai workspace commands:\n",
                "  /swarm <prompt>       — multi-agent swarm task\n",
                "  /agent <id> <msg>     — message a specific agent\n",
                "  /agents               — list registered agents\n",
                "  /model                — list available models\n",
                "  /features             — show feature flags\n",
                "  /queue                — task queue status\n",
                "  /chess new …          — start a chess game vs BonsAI\n",
                "  /chess move <id> <mv> — make a chess move\n",
                "  /go new …             — start a Go game vs BonsAI\n",
                "  /go move <id> <GTP>   — make a Go move\n",
                "  /puzzle daily         — get today's chess puzzle\n",
                "  /puzzle guess <id> <mv> — submit a puzzle answer\n",
                "  /tournament list      — list tournaments\n",
                "  /tournament new …     — create a tournament\n",
                "  /help                 — this message",
            ).to_string(),

            _ => format!("Unknown command '{cmd}'. Try /help."),
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
