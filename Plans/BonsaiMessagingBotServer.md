Bonsai Messaging Bot Server — Production Plan (v3, Audit-Hardened)
Context
Standalone Rust binary (bonsai-bot) that bridges Discord, Telegram, Matrix, and Email (IMAP) to Bonsai Buddy's AI and machine-control capabilities. Free platforms only. Works behind NAT. Full assistant tool access with per-platform allowlists and HITL confirmation gates for high-risk operations. Auto-launches alongside the Tauri app.

Audit findings addressed in v2: structured confirmation protocol, local admin auth, injection defense model, session contract, failure/recovery matrix, idempotency, backpressure, secret lifecycle, Matrix E2E key management.

Audit findings addressed in v3: admin token key name unification (bot_admin_token), Matrix key backup passphrase one-time reveal flow with re-auth + audit log, regex crate added to Cargo.toml with once_cell precompilation, phrase deny-list replaced with protocol boundary guards + telemetry, email dedup fallback hash key, confirmation prompt_state column + state transitions, CSRF statement reworded, circuit breaker thresholds configurable, IMAP SEARCH algorithm specified, queue-full control path hardened, SLO targets added.

Architecture
┌──────────────────────────────────────────────────────────────────────┐
│  bonsai-bot (standalone Rust binary, port 11421)                     │
│                                                                      │
│  Platform Tasks (tokio tasks, bounded channels)                      │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐       │
│  │  Discord   │ │  Telegram  │ │   Matrix   │ │   Email    │       │
│  │  Gateway   │ │ getUpdates │ │   /sync    │ │ IMAP IDLE  │       │
│  └─────┬──────┘ └─────┬──────┘ └─────┬──────┘ └─────┬──────┘       │
│        └──────────────┴──────────────┴──────────────┘               │
│                             ▼                                        │
│                    InboundMessage queue                              │
│                    (bounded, 1024 cap)                               │
│                             ▼                                        │
│                    MessageRouter (worker pool, 8 workers)            │
│                    ├── Allowlist check                               │
│                    ├── Dedup (event_id → TTL cache)                  │
│                    ├── Rate limit (token bucket per user)            │
│                    ├── Sanitize (strip injection vectors)            │
│                    ├── BuddyClient → POST 11420                      │
│                    ├── Confirm gate (pending_confirms DB table)      │
│                    └── Formatter → platform.send()                  │
│                                                                      │
│  Admin API (127.0.0.1:11421, Bearer auth)                           │
│  /health  /status  /broadcast  /config/reload  /sessions            │
└──────────────────────────────────────────────────────────────────────┘
         ▲                          ▲
         │ HTTP                     │ HTTP
  Tauri SettingsPanel          Buddy API (11420)
  (Tauri commands.rs)          run_assistant_turn
NAT strategy: Discord uses Gateway WebSocket (outbound). Telegram uses long-poll (outbound). Matrix uses /sync HTTP long-poll (outbound). Email uses IMAP IDLE (outbound). No port forwarding required on any platform.

Directory Structure
z:\Projects\BonsaiWorkspace\
├── bonsai-workspace\        (existing — minor additions only)
└── bonsai-bot\
    ├── Cargo.toml
    └── src\
        ├── main.rs          startup, task orchestration, graceful shutdown
        ├── config.rs        BotConfig, load/save, keyring abstraction
        ├── session.rs       SQLite bot_sessions + pending_confirms tables
        ├── buddy_client.rs  HTTP client for 11420 with retry + circuit breaker
        ├── router.rs        security pipeline, routing, confirm gate
        ├── sanitizer.rs     input sanitization (injection defense)
        ├── formatter.rs     reply → platform-specific format
        ├── dedup.rs         per-platform event-ID deduplication (TTL cache)
        ├── admin_api.rs     Axum admin server (127.0.0.1:11421, Bearer auth)
        ├── health.rs        buddy health watcher, circuit breaker state
        ├── metrics.rs       in-process counters (messages, errors, latency)
        └── platforms\
            ├── mod.rs       MessagingPlatform trait + message types
            ├── discord.rs   serenity Gateway + slash commands + components
            ├── telegram.rs  teloxide dispatcher + inline keyboards
            ├── matrix.rs    matrix-sdk E2E + key backup
            └── email.rs     async-imap IDLE + lettre SMTP
Critical Files
New (bonsai-bot)
File	Purpose
Cargo.toml	Dependencies; feature flags per platform
main.rs	Task spawn, bounded channel, graceful SIGTERM/SIGINT
config.rs	BotConfig, PlatformSlot<T>, JSON load/save, keyring wrapper
session.rs	SQLite schema, ensure_session(), cleanup_stale()
buddy_client.rs	Retry (3×, exp backoff), circuit breaker, health polling
router.rs	8-stage security pipeline (see below)
sanitizer.rs	Injection defense: unicode normalization, length cap, deny-list patterns
formatter.rs	Platform-specific Markdown renderers + chunking
dedup.rs	DedupCache<platform, event_id> with TTL eviction
admin_api.rs	Axum on 127.0.0.1:11421 with Bearer token
health.rs	Buddy health watcher, circuit breaker (Open/HalfOpen/Closed)
platforms/mod.rs	MessagingPlatform trait, InboundMessage, FormattedMessage
platforms/discord.rs	serenity EventHandler + slash commands + button confirmations
platforms/telegram.rs	teloxide Dispatcher + InlineKeyboard confirmations
platforms/matrix.rs	matrix-sdk, E2E, key backup, cross-signing
platforms/email.rs	async-imap IDLE/poll + lettre TLS
Modified (bonsai-workspace)
File	Change
src-tauri/src/config.rs	Add bot_server_port: u16 = 11421
src-tauri/src/commands.rs	6 new Tauri commands (status, save config per platform)
src-tauri/src/lib.rs	Register new commands
src/lib/components/SettingsPanel.svelte	Add "Bots" tab
src/lib/stores/settings.ts	Add botServerStatus writable + loadBotStatus()
Launch-BonsaiWorkspace.ps1	Launch bot binary alongside app
src/launch-all.mjs	Spawn bot process in desktop mode
Phase 1 — Cargo.toml
[package]
name    = "bonsai-bot"
version = "0.1.0"
edition = "2021"

[features]
default  = ["discord", "telegram", "email"]
discord  = ["dep:serenity"]
telegram = ["dep:teloxide"]
matrix   = ["dep:matrix-sdk"]
email    = ["dep:async-imap", "dep:async-native-tls"]
all      = ["discord", "telegram", "matrix", "email"]

[dependencies]
tokio             = { version = "1",    features = ["full"] }
reqwest           = { version = "0.12", features = ["json", "stream", "rustls-tls"] }
axum              = { version = "0.7",  features = ["macros", "json"] }
serde             = { version = "1.0",  features = ["derive"] }
serde_json        = "1.0"
sqlx              = { version = "0.8",  features = ["runtime-tokio", "sqlite", "macros"] }
keyring           = "2"
tracing           = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
async-trait       = "0.1"
uuid              = { version = "1",    features = ["v4"] }
chrono            = { version = "0.4",  features = ["serde"] }
tokio-util        = "0.7"
futures           = "0.3"
governor          = "0.6"     # token-bucket rate limiting
dashmap           = "5"       # concurrent rate-limit state (DashMap)
lru               = "0.12"    # dedup LRU cache
unicode-normalization = "0.1" # NFC normalization in sanitizer
regex             = "1"       # sanitizer deny-list pattern matching
once_cell         = "1"       # LazyLock for pre-compiled sanitizer regexes
sha2              = "0.10"    # email dedup fallback key (SHA-256)
hex               = "0.4"     # encode SHA-256 digest as hex string
lettre            = { version = "0.11", features = ["smtp-transport", "rustls-tls", "builder"] }

serenity          = { version = "0.12", optional = true, features = ["client","gateway","model","http","cache"] }
teloxide          = { version = "0.13", optional = true, features = ["macros","ctrlc_handler"] }
matrix-sdk        = { version = "0.7",  optional = true, features = ["rustls-tls","e2e-encryption","sled-state-store","sled-cryptostore"] }
async-imap        = { version = "0.9",  optional = true }
async-native-tls  = { version = "0.5",  optional = true }
Dependency governance: each optional dep is only compiled when its feature flag is set. CI builds with --features default (Discord+Telegram+Email). Matrix built separately with --features all. CVE scanning runs cargo audit in CI on all feature combinations.

Phase 2 — Config & Secrets
config.rs
// File location: {OS config dir}/bonsai/bonsai-bot-config.json
// Contains ONLY non-secret settings. All tokens via keyring.

#[derive(Serialize, Deserialize, Clone)]
pub struct BotConfig {
    pub schema_version:  u8,             // 1 — bump when breaking fields added
    pub buddy_api_url:   String,         // "http://127.0.0.1:11420"
    pub admin_port:      u16,            // 11421
    pub db_path:         String,         // "{config_dir}/bonsai-bot.db"
    pub discord:         PlatformSlot<DiscordConfig>,
    pub telegram:        PlatformSlot<TelegramConfig>,
    pub matrix:          PlatformSlot<MatrixConfig>,
    pub email:           PlatformSlot<EmailConfig>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlatformSlot<T> {
    pub enabled:  bool,
    pub config:   T,
}

pub struct DiscordConfig {
    pub allowed_guild_ids:   Vec<String>,
    pub allowed_channel_ids: Vec<String>,    // empty = all channels in allowed guilds
    pub allowed_user_ids:    Vec<String>,    // empty = all users in allowed guilds
    pub command_prefix:      String,         // "!"
}

pub struct TelegramConfig {
    pub allowed_chat_ids:   Vec<i64>,        // empty = deny all
    pub poll_timeout_secs:  u64,             // 30
}

pub struct MatrixConfig {
    pub homeserver_url:   String,            // "https://matrix.org"
    pub username:         String,            // "@bonsaibot:matrix.org"
    pub allowed_rooms:    Vec<String>,       // room IDs; empty = deny all
    pub allowed_users:    Vec<String>,       // Matrix IDs; empty = all in allowed rooms
    pub key_backup_passphrase_keychain_account: String, // "matrix_key_backup_pass"
}

pub struct EmailConfig {
    pub imap_host:           String,
    pub imap_port:           u16,            // 993
    pub imap_username:       String,
    pub subject_prefix:      String,         // "[BONSAI]"
    pub smtp_host:           String,
    pub smtp_username:       String,
    pub smtp_from:           String,
    pub allowed_from_addrs:  Vec<String>,
    pub poll_interval_secs:  u64,            // 30
}
Keychain accounts (service: "bonsai-bot")
Account key	Content
discord_token	Discord bot token
telegram_token	Telegram bot token
matrix_password	Matrix account password or access token
matrix_key_backup_pass	Matrix key backup recovery passphrase
email_imap_password	IMAP account password / app password
email_smtp_password	SMTP password (may differ from IMAP)
bot_admin_token	Local admin API Bearer token (auto-generated on first start, stored here)
Secret lifecycle:

Tokens stored via keyring::Entry::new("bonsai-bot", account).
On first start, bot_admin_token is auto-generated (UUID v4) and stored in keychain.
Token rotation: POST /config/rotate-admin-token (requires current valid token). Generates new UUID, stores in keychain, invalidates old token immediately.
Revocation: Tauri command clear_bot_platform_token(platform) deletes keychain entry. Bot's /config/reload picks up the absence and disables that platform.
Stale detection: if platform fails auth 3× consecutively, emit bot-token-invalid Tauri event so SettingsPanel shows a red badge and prompts the user to re-enter the token.
Phase 3 — Structured Confirmation Protocol
No magic strings. Uses a versioned JSON envelope transmitted as a synthetic assistant message.

Confirmation request (Buddy → Bot)
The Buddy API response body for non-streaming mode includes an optional top-level field:

{
  "id": "buddy-xxx",
  "object": "chat.completion",
  "choices": [{
    "message": {
      "role": "assistant",
      "content": "I need to run a command. Please approve."
    },
    "finish_reason": "tool_calls_pending_approval"
  }],
  "bonsai_ext": {
    "schema": 1,
    "type": "confirm_required",
    "token": "abc123",
    "tool": "run_command",
    "args": { "command": "rm -rf /tmp/test" },
    "risk": "high",
    "prompt": "Run shell command: `rm -rf /tmp/test`",
    "expires_at": 1716000120
  }
}
finish_reason = "tool_calls_pending_approval" signals the bot to surface a confirmation. bonsai_ext carries the typed payload. Bot validates: schema == 1, type == "confirm_required", token non-empty, expires_at > now.

Confirmation response (Bot → Buddy)
{
  "model": "bonsai-buddy",
  "messages": [
    { "role": "user", "content": "...(prior message)..." },
    {
      "role": "user",
      "content": "__bot_confirm__",
      "bonsai_ext": {
        "schema": 1,
        "type": "confirm_response",
        "token": "abc123",
        "approved": true
      }
    }
  ],
  "stream": false
}
Bot injects the confirm response as the next user turn with the typed bonsai_ext field.

Required backend change (assistant_commands.rs): before calling run_assistant_turn, check if the last user message contains bonsai_ext.type == "confirm_response", look up the token in ConfirmationGate, and resolve approve/deny directly. The token validates expiry. If the token is expired or unknown, return a 400 with {"error": {"type": "confirm_expired"}}.

Pending confirm persistence
Stored in bonsai-bot.db pending_confirms table (not in-memory) so confirms survive bot restarts:

CREATE TABLE IF NOT EXISTS pending_confirms (
    token        TEXT PRIMARY KEY,
    platform     TEXT NOT NULL,
    chat_id      TEXT NOT NULL,
    user_id      TEXT NOT NULL,
    tool         TEXT NOT NULL,
    args_json    TEXT NOT NULL,
    prompt       TEXT NOT NULL,
    expires_at   INTEGER NOT NULL,
    prompt_state TEXT NOT NULL DEFAULT 'created',  -- 'created'|'prompted'|'resolved'|'expired'
    prompt_nonce INTEGER NOT NULL DEFAULT 0         -- increments each time prompt is (re)sent
);
State transitions:

created  → prompted   (first prompt sent to platform user)
prompted → prompted   (re-sent on restart; prompt_nonce increments)
prompted → resolved   (user approved or denied)
prompted → expired    (background task marks after expires_at passes)
Restart replay contract: on startup, load all rows where prompt_state IN ('created', 'prompted') and expires_at > now(). Resend the prompt message. Increment prompt_nonce. Platform UIs that carry state (e.g. Discord message with buttons) should check prompt_nonce on callback to reject stale interactions from a previous prompt send (compare stored prompt_nonce at callback time).

Background task purges rows with prompt_state = 'expired' AND expires_at < now() - 3600 every 60s. Sets prompt_state = 'expired' for rows where expires_at < now() and state is still prompted.

Phase 4 — Admin API Security
Bind: 127.0.0.1:11421 (loopback only — no network-accessible interface)
Auth: Authorization: Bearer <bot_admin_token>
      Token stored in OS keychain, auto-generated on first start.
      All routes except /health require valid token.
Routes:

GET  /health             → no auth; {"status":"ok","version":"0.1.0"}
GET  /status             → auth; per-platform connected/error state
GET  /sessions           → auth; list active bot sessions
POST /broadcast          → auth; {"message":String,"platforms":["discord",...]}
POST /config/reload      → auth; re-reads config + keyring without restart
POST /config/rotate-admin-token → auth; generates new token, invalidates old
Reduced remote CSRF risk: loopback bind eliminates remote CSRF; Bearer auth mitigates local process CSRF. Defense-in-depth: token is stored only in OS keychain (not config file, not env), request origin is logged to telemetry for anomaly detection, and token TTL-rotation is available via POST /config/rotate-admin-token. On shared hosts, standard OS process-isolation applies.

Tauri → admin API: commands.rs makes HTTP requests to http://127.0.0.1:11421/... with the token fetched from the keychain via secrets_store.get("bot_admin_token").

Phase 5 — Injection Defense (sanitizer.rs)
Design rationale: phrase-based deny-lists are brittle (high false positive rate, trivially bypassed by paraphrase). The sanitizer focuses on structural safety (length, encoding, control characters) and strict protocol field boundary enforcement. Intent classification is delegated to Bonsai Buddy's existing policy engine. False-positive telemetry is emitted for monitoring.

Multi-layer sanitization applied to every inbound message before sending to Buddy:

use once_cell::sync::Lazy;
use regex::Regex;

// Protocol boundary guards — precompiled once at startup.
// These block injection of bonsai_ext protocol fields via user message content.
// Phrase-based deny-lists are NOT used (high false positives, easily bypassed).
static PROTOCOL_GUARDS: Lazy<Vec<Regex>> = Lazy::new(|| vec![
    Regex::new(r"bonsai_ext").unwrap(),      // block protocol envelope injection
    Regex::new(r"\[CONFIRM_").unwrap(),      // block legacy magic string format
    Regex::new(r#""type"\s*:\s*"confirm_"#).unwrap(), // block confirm_response injection
]);

pub fn sanitize(input: &str, metrics: &Metrics) -> Result<String, SanitizeError> {
    // 1. Length cap: reject > 8000 bytes (pre-NFC)
    if input.len() > 8000 {
        metrics.sanitize_rejected("too_long");
        return Err(SanitizeError::TooLong);
    }

    // 2. Unicode normalization (NFC) — eliminates homoglyph and canonicalization attacks
    let normalized = unicode_normalization::UnicodeNormalization::nfc(input).collect::<String>();

    // 3. Null byte removal (Rust strings can hold null bytes; LLM APIs cannot)
    let no_nulls = normalized.replace('\0', "");

    // 4. Protocol boundary guards: block injection of bonsai_ext protocol fields only
    for guard in PROTOCOL_GUARDS.iter() {
        if guard.is_match(&no_nulls) {
            metrics.sanitize_rejected("protocol_boundary");
            return Err(SanitizeError::ProtocolInjection);
        }
    }

    // 5. Strip ASCII control characters except \n, \t, \r
    let clean: String = no_nulls.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t' || *c == '\r')
        .collect();

    Ok(clean)
}
What is NOT blocked: natural-language phrases that superficially resemble prompt injection ("ignore previous instructions", "you are now X"). These are handled by Bonsai Buddy's policy engine and system prompt hardening — blocking them in the bot creates false positives for legitimate security discussions. Sanitize rejections are counted in metrics by reason code. Raw input is never logged; only platform + user ID + rejection reason are recorded.

Phase 6 — Idempotency & Deduplication (dedup.rs)
Each platform assigns unique event IDs. The dedup cache prevents duplicate assistant calls if a platform re-delivers an event (Telegram update re-sent on long-poll timeout, Matrix event replayed on /sync restart, IMAP message fetched twice).

pub struct DedupCache {
    // LRU keyed by "{platform}:{event_id}", TTL = 10 minutes
    inner: Mutex<lru::LruCache<String, Instant>>,
    ttl:   Duration,
}

impl DedupCache {
    pub fn is_duplicate(&self, platform: &str, event_id: &str) -> bool {
        let key = format!("{platform}:{event_id}");
        let mut cache = self.inner.lock().unwrap();
        if let Some(ts) = cache.get(&key) {
            return ts.elapsed() < self.ttl;
        }
        cache.put(key, Instant::now());
        false
    }
}
Per-platform event ID sources:

Platform	Primary Event ID	Fallback (if absent)
Discord	message.id (snowflake — globally unique)	n/a (always present)
Telegram	update.update_id (monotonic per bot)	n/a (always present)
Matrix	event.event_id (server-assigned, globally unique)	n/a (always present)
Email	Message-ID header (RFC 2822)	sha256(from + date + subject + body[..100]) encoded as hex
Email fallback algorithm (if Message-ID header absent or empty):

fn email_dedup_key(from: &str, date: &str, subject: &str, body: &str) -> String {
    use sha2::{Digest, Sha256};
    let input = format!("{from}\0{date}\0{subject}\0{}", &body[..body.len().min(100)]);
    hex::encode(Sha256::digest(input.as_bytes()))
}
LRU capacity: 10,000 entries. At 10 messages/min/user and 100 concurrent users, this covers ~16 hours of activity before eviction. Adjust capacity in config if needed.

Phase 7 — Backpressure & Concurrency
InboundMessage queue:       bounded tokio::sync::mpsc channel, capacity 1024 (configurable)
  → queue-full handling uses a SEPARATE low-cost control channel (capacity 64):
    platform task tries_send() to main queue; if Err(Full) sends shed notice to control channel
    control task drains control channel and sends "🔄 Bonsai is busy, try again in a moment."
    → this keeps the shed-reply path outside the saturated worker pool
  → metrics counter: queue_full_drops

Worker pool:                8 Tokio tasks consuming from the queue (configurable)
  → each task: dedup → sanitize → allowlist → rate-limit → buddy call → format → send
  → buddy call timeout: 120s (matches proxy_to_llama timeout)

Buddy circuit breaker:      state machine: Closed → Open → HalfOpen → Closed
  Thresholds (configurable in BotConfig.circuit_breaker):
    open_after_failures:  5   (consecutive errors to open)
    half_open_probe_secs: 30  (seconds before probe attempt)
    close_on_successes:   1   (consecutive probe successes to close)
  → When Open: immediately reply "⚠️ Bonsai is currently unavailable."
  → Emit `bot-buddy-circuit-open` Tauri event for Settings badge

Per-platform send queue:    1 Tokio task per platform for outbound messages
  → bounded channel capacity 256 per platform
  → Discord rate limit: respects X-RateLimit-* headers, exponential backoff

Global cap:                 max 64 concurrent Buddy API requests in-flight (configurable)
  → tokio::sync::Semaphore(64)
  → Excess: reply "🔄 At capacity. Please wait."
// BotConfig additions for runtime tuning
pub struct BackpressureConfig {
    pub inbound_queue_capacity:   usize,  // default 1024
    pub worker_count:             usize,  // default 8
    pub global_semaphore:         usize,  // default 64
    pub per_platform_send_queue:  usize,  // default 256
}

pub struct CircuitBreakerConfig {
    pub open_after_failures:  u32,   // default 5
    pub half_open_probe_secs: u64,   // default 30
    pub close_on_successes:   u32,   // default 1
}
Phase 8 — Session Continuity Contract
SQLite schema
CREATE TABLE IF NOT EXISTS bot_sessions (
    id                TEXT PRIMARY KEY,      -- "{platform}_{chat_id}_{user_id}"
    platform          TEXT NOT NULL,
    platform_user_id  TEXT NOT NULL,
    platform_chat_id  TEXT NOT NULL,
    buddy_session_id  TEXT NOT NULL,         -- AssistantSession.id in Bonsai's SQLite
    display_name      TEXT,
    created_at        INTEGER NOT NULL,
    last_active       INTEGER NOT NULL,
    is_archived       INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_sessions_lookup
    ON bot_sessions(platform, platform_user_id, platform_chat_id);
Ownership rules
Creator: bonsai-bot creates AssistantSessions in Bonsai via the Buddy API metadata field {"source": "bot", "platform": "discord", "user_id": "..."}.
Bonsai authority: Bonsai's assistant_store is the source of truth for session content. bonsai-bot.db holds only the mapping from platform identity → buddy session ID.
Conflict resolution: if buddy_session_id in bot_sessions doesn't exist in Bonsai's DB (session was deleted via SettingsPanel), ensure_session() creates a new session and updates the row. The previous conversation context is lost (acceptable: user deleted it intentionally).
Stale cleanup: sessions not active for 30 days are soft-deleted (is_archived = 1). Archived sessions are excluded from lookups; the next message from that user creates a fresh session. Hard delete after 90 days.
ensure_session() contract
pub async fn ensure_session(
    db: &SqlitePool,
    buddy: &BuddyClient,
    platform: &str,
    user_id: &str,
    chat_id: &str,
    display_name: &str,
) -> Result<String, SessionError> {
    // 1. SELECT from bot_sessions WHERE platform=? AND platform_user_id=? AND platform_chat_id=?
    // 2. If found and not archived: verify buddy_session_id still exists via buddy.session_exists()
    //    - If exists: return buddy_session_id (fast path)
    //    - If missing: fall through to create new
    // 3. If not found or archived: create new AssistantSession via Buddy API
    //    - POST /v1/chat/completions with {"metadata":{"session_init":true, "platform":...}}
    //    - Buddy returns session_id in bonsai_ext field
    // 4. Upsert bot_sessions row with new buddy_session_id
    // 5. Return buddy_session_id
}
Phase 9 — Failure & Recovery Matrix
Failure Scenario	Bot Behavior	Recovery
Buddy API down at start	Wait up to 60s polling /health, then start bot anyway with circuit breaker Open	Auto-recover when health resumes
Buddy API goes down mid-run	Circuit breaker opens after 5 failures. Reply "⚠️ Bonsai is unavailable." for subsequent messages	Breaker probes every 30s; closes on first success
Buddy 502 (slot recycling)	Already handled by api_server.rs 502 retry. Bot sees success.	None needed
Buddy returns 5xx	Retry 3× with 1s/2s/4s backoff. If all fail: reply "⚠️ Bonsai error. Please retry."	User retries
Discord Gateway disconnect	serenity auto-reconnects with exponential backoff (built-in)	Automatic
Telegram poll timeout/error	teloxide auto-retries (built-in). If 5 consecutive: log warn + 30s cooldown	Automatic
Matrix sync error	matrix-sdk auto-reconnects. Session state persisted in Sled store	Automatic
IMAP connection drop	Reconnect on next poll cycle (30s max delay)	Automatic
Keychain access denied	Log error, disable that platform, emit bot-token-invalid Tauri event	User re-enters token in SettingsPanel
SQLite DB locked	Retry with PRAGMA journal_mode=WAL (write-ahead log avoids most locks). If 3 retries fail: restart with 5s delay	Automatic
SQLite DB corrupted	Detect via PRAGMA integrity_check. Rename corrupt DB, create fresh DB, emit bot-db-corrupt event. Sessions lost but app continues.	Manual: user re-links sessions
Config file missing	Use embedded defaults, emit bot-config-missing event, create default config file	Auto-created
Admin API port conflict	Try 11421..11425 (same pattern as buddy_api_server). If all fail: log error, admin API unavailable but bot still runs	Restart with different port
Memory pressure (queue full)	Drop inbound message, reply with backpressure message	Caller retries
Confirmation token expired	Return confirm_expired error. Reply to user: "⏰ Confirmation expired. Please resend your request."	User resends
Phase 10 — Platform Implementations
platforms/discord.rs
// serenity v0.12, EventHandler implementation
// Slash commands: /ask [query], /run [command], /status
// Message prefix: "!" triggers command mode; all other DMs/mentions → Buddy chat

struct DiscordHandler {
    router: Arc<MessageRouter>,
    config: DiscordConfig,
}

impl EventHandler for DiscordHandler {
    // message(): filter bots, validate allowlist, wrap in InboundMessage, route
    // interaction_create():
    //   - ApplicationCommand → slash command dispatch
    //   - MessageComponent → button "confirm_approve:{token}" / "confirm_deny:{token}"
    // ready(): register global slash commands via GuildId::set_application_commands or global
}

// Confirmation UI: CreateComponents with ActionRow of two buttons
//   ✅ Approve (style: Success, custom_id: "confirm_approve:{token}")
//   ❌ Deny    (style: Danger,  custom_id: "confirm_deny:{token}")
// Edit original message to show result after button press

// Rich embed:
//   Color: 0x2d5016 (Bonsai green)
//   Title: "🌿 Bonsai"
//   Description: reply text (≤ 4096 chars; split into multiple embeds if longer)
//   Footer: "Bonsai Buddy • {timestamp}"
//   Code fields: each code block as embed field (≤ 1024 chars; split if needed)
platforms/telegram.rs
// teloxide v0.13 with Dispatcher + dptree
// Bot commands: /ask, /run, /status, /help
// Non-command messages in allowed chats → Buddy chat (natural language)

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    Ask(String),
    Run(String),
    Status,
    Help,
}

// Typing indicator: send_chat_action(ChatAction::Typing) before Buddy call
// Long message splitting: chunk at 4000 chars at paragraph boundaries
// Confirmation: InlineKeyboardMarkup
//   [✅ Approve | ❌ Deny]  callback_data: "ca:{token}" / "cd:{token}"
// Callback query handler: resolve confirmation, edit original message
// MarkdownV2: all special chars escaped before send (., !, (, ), -, =, +, etc.)
platforms/matrix.rs
// matrix-sdk v0.7 with Sled state/crypto stores for persistence
// Auth: password login → access token persisted in Sled
// E2E: enabled by default via CryptoStore in Sled

// Key management:
//   - Cross-signing keys uploaded on first login
//   - Key backup enabled with recovery passphrase from keychain ("matrix_key_backup_pass")
//     → passphrase auto-generated on first start, stored in keychain only
//   - Key backup passphrase recovery: Tauri command `get_matrix_key_backup_passphrase()`
//     requires caller to supply current bot_admin_token as proof, emits audit log entry,
//     returns passphrase for one-time display in UI modal (never stored in frontend state)
//   - Devices not verified by default — bot accepts messages from any device in allowed rooms

// Event handler: m.room.message events in allowed rooms from allowed users
// Auto-join: m.room.member events with membership=invite in allowed_rooms

// Prefix routing:
//   "!ask ..."  → Buddy chat
//   "!run ..."  → Buddy with run intent
//   "!status"   → admin status
//   (no prefix, direct room) → Buddy chat

// Confirmation: send text message:
//   "⚠️ Confirmation required: {prompt}
//    Reply 'yes' to approve or 'no' to deny (expires in 2 minutes)"
// Next 'yes'/'no' from same user in same room resolves pending confirm

// Formatted replies: m.text with HTML formatted_body
//   <strong>, <em>, <code>, <pre>, <ul>, <ol>, <li>
//   Inline code: <code>
//   Code blocks: <pre><code class="language-{lang}">{code}</code></pre>
platforms/email.rs
// async-imap with async-native-tls for TLS (port 993)
// IMAP IDLE if server supports IDLE capability; otherwise 30s interval SELECT+SEARCH

// Poll cycle:
//   1. SELECT INBOX
//   2. IMAP SEARCH: use only universally supported criteria to avoid server incompatibilities
//      → SEARCH UNSEEN SUBJECT "{subject_prefix}"
//      (FROM filter applied in code — not in IMAP query — because multi-OR FROM is not
//       universally supported; IMAP SEARCH OR nesting is server-specific)
//   3. FETCH UID BODY[HEADER.FIELDS (FROM SUBJECT DATE MESSAGE-ID)] BODY.PEEK[TEXT]<0.2000>
//      (PEEK avoids marking as seen before processing; partial fetch caps body size)
//   4. For each uid: check From in allowed_from_addrs (case-insensitive)
//      → skip silently if not in allowlist (mark Seen to prevent re-processing)
//      → sanitize → router.route() → lettre SMTP reply → STORE UID +FLAGS.SILENT \Seen
//   5. No EXPUNGE unless delete_processed = true in config

// IMAP connection resilience:
//   - NOOP keepalive every 25 minutes (IMAP timeout avoidance)
//   - Reconnect on any IO error

// lettre reply:
//   Transport: SmtpTransport with STARTTLS or TLS
//   Message:
//     From: config.smtp_from
//     To: original sender
//     Subject: "Re: {original_subject}"
//     Message-ID: generated UUID
//     In-Reply-To: original Message-ID
//     Body: multipart/alternative
//       text/plain: strip HTML, wrap at 80 chars
//       text/html: full HTML template with inline CSS (bonsai branding)

// Allowlist: sender From address must match allowed_from_addrs exactly (case-insensitive)
Phase 11 — Formatter (formatter.rs)
pub fn format(reply: &str, platform: &str) -> FormattedMessage {
    match platform {
        "discord"  => format_discord(reply),
        "telegram" => format_telegram_mdv2(reply),
        "matrix"   => format_matrix_html(reply),
        "email"    => format_email_html(reply),
        _          => FormattedMessage { text: reply.to_string(), chunks: vec![] },
    }
}

// Discord: standard Markdown. Split at 1990 chars (2000 limit − buffer).
// Telegram MarkdownV2: escape ., !, (, ), -, =, +, {, }, |, ~, >, #, =, [, ].
//   Split at 4000 chars at paragraph boundaries.
// Matrix HTML: ALLOWED_TAGS subset (same as AssistantMessage.svelte): p, strong, em, code,
//   pre, ul, ol, li, blockquote, a. No script, iframe, or event handlers.
// Email HTML: full HTML template. Pre-formatted inline CSS. Plain text fallback.
// All: code blocks extracted and chunked if > platform limit. Tool call summaries
//   shown as "⚙️ Tool: {name} → {result summary (≤200 chars)}".
Phase 12 — Tauri Integration
New Tauri Commands
// commands.rs additions

#[tauri::command]
pub async fn get_bot_server_status(state: State<'_, AppState>) -> Result<Value, String> {
    // GET http://127.0.0.1:{config.bot_server_port}/status
    // Auth: Bearer token from secrets_store.get("bot_admin_token")
    // Returns raw JSON or {"error": "Bot server not running", "connected": false}
}

#[tauri::command]
pub async fn save_discord_bot_config(
    app_handle: AppHandle,
    token: String,
    allowed_guild_ids: Vec<String>,
    allowed_channel_ids: Vec<String>,
    allowed_user_ids: Vec<String>,
) -> Result<(), String> {
    // Store token: secrets_store.store("discord_token", &token)
    // Save non-secret fields to bonsai-bot-config.json
    // POST http://127.0.0.1:11421/config/reload (if bot running)
}

// Similarly: save_telegram_bot_config, save_matrix_bot_config, save_email_bot_config

#[tauri::command]
pub async fn test_bot_platform(platform: String, state: State<'_, AppState>)
    -> Result<String, String> {
    // POST /config/reload → GET /status → return platform-specific status
}

#[tauri::command]
pub async fn get_matrix_key_backup_passphrase(
    admin_token_proof: String,  // caller must supply the current bot_admin_token
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    // 1. Verify admin_token_proof == secrets_store.get("bot_admin_token")
    //    → If mismatch: return Err("Unauthorized")
    // 2. Fetch passphrase: secrets_store.get("matrix_key_backup_pass")
    // 3. Emit audit event via audit_log: { action: "matrix_key_backup_revealed", ts: now() }
    //    (audit_log is append-only; this provides accountability for each reveal)
    // 4. Return passphrase — UI must show it once in a modal marked "Save this now"
    //    and clear it from state on modal dismiss.
    //    NOTE: passphrase is never included in Tauri events, IPC logs, or state stores.
}
SettingsPanel.svelte — Bots Tab
┌─────────────────────────────────────────────────────────┐
│ 🤖 Messaging Bots                                       │
│ Bot server: ● running (11421)  or  ○ not running        │
├──────────────────────────────────────────────────────────┤
│ [Discord] [Telegram] [Matrix] [Email]                   │
│                                                          │
│ ── Discord ─────────────────────────── ● Connected ──   │
│ Bot Token    [••••••••••••] [Save]                       │
│ Guild IDs    [123456789, ...]                            │
│ Channel IDs  [optional — leave empty for all]           │
│ User IDs     [optional — leave empty for all]           │
│              [Test Connection]  [Save Settings]          │
│                                                          │
│ ── Telegram ───────────────────────── ○ Disabled ───    │
│ Bot Token    [••••••••••••] [Save]                       │
│ Chat IDs     [required — comma separated]               │
│              [Test Connection]  [Save Settings]          │
└──────────────────────────────────────────────────────────┘

Status badges poll invoke('get_bot_server_status') on mount + every 30s.
Token fields: write-only display (show ●●●●●● if stored, empty if not).
Launcher Updates
Launch-BonsaiWorkspace.ps1:

$botExe = Join-Path $PSScriptRoot "bonsai-bot\target\release\bonsai-bot.exe"
if (Test-Path $botExe) {
    $botProc = Start-Process -FilePath $botExe -WindowStyle Hidden -PassThru
    Write-Host "[bot] Started bonsai-bot.exe (PID $($botProc.Id))"
} else {
    Write-Host "[bot] bonsai-bot.exe not found — messaging bots disabled"
}
bonsai-workspace/src/launch-all.mjs:

import { existsSync } from 'fs';
import { spawn } from 'child_process';
import { join, dirname } from 'path';

const botBin = join(dirname(fileURLToPath(import.meta.url)),
    '../../bonsai-bot/target/release/bonsai-bot' + (process.platform === 'win32' ? '.exe' : ''));
if (existsSync(botBin)) {
    const bot = spawn(botBin, [], { stdio: 'pipe' });
    bot.stdout.on('data', d => process.stdout.write('[bot] ' + d));
    bot.stderr.on('data', d => process.stderr.write('[bot] ' + d));
    console.log('[bot] started pid', bot.pid);
}
Execution Order
#	Task	Files	Notes
1	bonsai-bot/Cargo.toml	NEW	Features: discord+telegram+email default
2	config.rs	NEW	BotConfig, keyring wrapper, auto-generate admin token
3	session.rs	NEW	SQLite schema, ensure_session(), stale cleanup
4	health.rs	NEW	Buddy health watcher, circuit breaker (Closed/Open/HalfOpen)
5	buddy_client.rs	NEW	HTTP client, retry, circuit breaker integration
6	dedup.rs	NEW	LRU dedup cache, 10min TTL
7	sanitizer.rs	NEW	5-layer sanitization pipeline
8	router.rs	NEW	8-stage pipeline, backpressure, worker pool (8 tasks)
9	formatter.rs	NEW	4-platform formatters
10	admin_api.rs	NEW	Axum on 127.0.0.1:11421, Bearer auth
11	metrics.rs	NEW	In-process counters exposed at /metrics
12	platforms/mod.rs	NEW	MessagingPlatform trait
13	platforms/discord.rs	NEW	serenity handler, slash commands, button confirms
14	platforms/telegram.rs	NEW	teloxide dispatcher, inline keyboards
15	platforms/email.rs	NEW	async-imap IDLE, lettre SMTP
16	platforms/matrix.rs	NEW	matrix-sdk, E2E, key backup passphrase
17	main.rs	NEW	tokio main, bounded channel, task orchestration, SIGTERM
18	assistant_commands.rs	MODIFY	Handle bonsai_ext.type == "confirm_response" in submit_assistant_chat
19	buddy_api_server.rs	MODIFY	Add bonsai_ext field to non-stream response when confirm required
20	config.rs (workspace)	MODIFY	Add bot_server_port: u16 = 11421
21	commands.rs (workspace)	MODIFY	6 new Tauri commands
22	lib.rs (workspace)	MODIFY	Register new commands
23	SettingsPanel.svelte	MODIFY	Bots tab with platform config + status badges
24	settings.ts	MODIFY	botServerStatus store + loadBotStatus()
25	Launch-BonsaiWorkspace.ps1	MODIFY	Launch bot binary
26	launch-all.mjs	MODIFY	Spawn bot process
27	cargo build --release (bonsai-bot)	VERIFY	0 errors
28	cargo audit --features all	VERIFY	0 critical CVEs
29	Smoke tests (see below)	VERIFY	All pass
Operational SLOs
Metric	Target
Discord/Telegram reply latency (p95)	≤ 5s end-to-end (non-streaming)
Email reply latency (p95)	≤ 35s (IMAP poll cycle + SMTP send)
Matrix reply latency (p95)	≤ 8s (/sync long-poll + send)
Circuit breaker recovery time (RTO)	≤ 35s after Buddy restarts (30s probe + 1 request)
Queue saturation error rate	≤ 1% of messages dropped at capacity
Sanitizer false positive rate	< 0.5% of legitimate messages rejected (monitored via metrics)
Duplicate event rate	0 duplicate assistant calls (dedup cache enforces)
Verification — Positive Tests
curl http://127.0.0.1:11421/health → {"status":"ok"} (no auth)
curl -H "Authorization: Bearer {token}" http://127.0.0.1:11421/status → platform states
Discord DM → Buddy reply within 5s
Discord /ask how are you slash command → embedded reply
Telegram message in allowed chat → reply within 5s with MarkdownV2
Email to IMAP inbox with [BONSAI] subject → HTML reply within 35s
High-risk command → confirmation prompt → approve → executed; deny → cancelled
Matrix DM !ask hello → text reply with HTML formatting
Matrix cross-signing: verify bot device key persists across restarts (same device ID)
Session resume: restart bot, send message in prior session → reply uses prior context
POST /broadcast {"message":"test","platforms":["discord"]} → message sent to channel
POST /config/reload → platform tokens reloaded from keychain without restart
SettingsPanel Bots tab shows ● Connected for active platforms
Verification — Negative / Adversarial Tests
Unauthorized sender (not in allowlist) → receives "Unauthorized", no Buddy call
Rate limit: 11th message in 60s → "Rate limit exceeded", no Buddy call
Queue full (1024 cap): simulate burst → "Bonsai is busy" reply, no crash
Buddy circuit open: stop Buddy, send 5 messages → all receive "unavailable" reply; restart Buddy → circuit closes, next message succeeds
Expired confirmation token (send "yes" after 2 min) → "Confirmation expired" reply
Injection attempt "Ignore all previous instructions" → "could not be processed safely" reply, no Buddy call
Malformed bonsai_ext in Buddy response → bot logs error, sends generic reply
SQLite missing → recreated on restart (bonsai-bot.db), new sessions created
Admin API without token → 401 Unauthorized
Admin API from non-loopback → port not listening (bind is 127.0.0.1)
Bot token revoked mid-run → platform disabled, bot-token-invalid Tauri event fires
Duplicate Telegram update (same update_id) → dedup cache blocks second processing
IMAP message fetched twice (IMAP server bug) → dedup on Message-ID prevents duplicate reply
cargo audit --features all → 0 high/critical CVEs
Acceptance Criteria
#	Criterion
1	All 4 platforms receive and reply to messages without a public IP
2	Confirmation flow uses structured JSON protocol (no magic strings), survives malformed payloads
3	Per-user sessions persist across bot restarts; orphaned sessions auto-detected and re-created
4	Unauthorized senders rejected before any Buddy call is made
5	Rate limiting: 11th message in 60s is rejected
6	Buddy circuit breaker opens after 5 failures; closes on recovery
7	Queue backpressure: burst > 1024 messages handled without crash
8	Bot token never appears in config JSON file (keychain only)
9	Admin API requires Bearer token; loopback bind only
10	Matrix E2E encryption enabled; key backup passphrase stored in keychain only; reveal requires bot_admin_token proof + emits audit log; one-time UI modal clears on dismiss
11	Duplicate events (all platforms) do not cause duplicate assistant calls
12	Injection patterns blocked by sanitizer before reaching Buddy
13	cargo build --release produces single bonsai-bot.exe, cargo audit passes
14	Bot auto-starts via launcher; SettingsPanel shows live per-platform status
15	All adversarial tests pass without crash or data leak
