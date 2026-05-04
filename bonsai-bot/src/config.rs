use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Top-level config ─────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BotConfig {
    #[serde(default = "default_schema_version")]
    pub schema_version: u8,
    #[serde(default = "default_buddy_api_url")]
    pub buddy_api_url: String,
    #[serde(default = "default_admin_port")]
    pub admin_port: u16,
    #[serde(default)]
    pub reclaim_allowed_ports: Vec<u16>,
    #[serde(default)]
    pub allowed_script_paths: Vec<String>,
    #[serde(default)]
    pub runtime_limits: RuntimeLimits,
    #[serde(default)]
    pub db_path: String,
    #[serde(default)]
    pub discord: PlatformSlot<DiscordConfig>,
    #[serde(default)]
    pub telegram: PlatformSlot<TelegramConfig>,
    #[serde(default)]
    pub matrix: PlatformSlot<MatrixConfig>,
    #[serde(default)]
    pub email: PlatformSlot<EmailConfig>,
    #[serde(default)]
    pub backpressure: BackpressureConfig,
    #[serde(default)]
    pub circuit_breaker: CircuitBreakerConfig,
    #[serde(default)]
    pub swarm_peers: Vec<SwarmPeer>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RuntimeLimits {
    #[serde(default)]
    pub max_runtime_secs: Option<u64>,
    #[serde(default)]
    pub max_instances_per_user: Option<u32>,
}

impl Default for RuntimeLimits {
    fn default() -> Self {
        Self { max_runtime_secs: None, max_instances_per_user: None }
    }
}

fn default_schema_version() -> u8 { 1 }
fn default_buddy_api_url() -> String { "http://127.0.0.1:11420".to_string() }
fn default_admin_port() -> u16 { 11666 }

impl Default for BotConfig {
    fn default() -> Self {
        let db_path = config_dir()
            .map(|d| d.join("bonsai-bot.db").to_string_lossy().into_owned())
            .unwrap_or_else(|| "bonsai-bot.db".to_string());
        Self {
            schema_version: 1,
            buddy_api_url:  "http://127.0.0.1:11420".to_string(),
            admin_port:     11666,
            reclaim_allowed_ports: Vec::new(),
            allowed_script_paths: Vec::new(),
            runtime_limits: RuntimeLimits::default(),
            db_path,
            discord:         PlatformSlot::default(),
            telegram:        PlatformSlot::default(),
            matrix:          PlatformSlot::default(),
            email:           PlatformSlot::default(),
            backpressure:    BackpressureConfig::default(),
            circuit_breaker: CircuitBreakerConfig::default(),
            swarm_peers:     Vec::new(),
        }
    }
}

// ── Platform slots ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PlatformSlot<T: Default> {
    pub enabled: bool,
    pub config:  T,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DiscordConfig {
    pub allowed_guild_ids:   Vec<String>,
    pub allowed_channel_ids: Vec<String>,
    pub allowed_user_ids:    Vec<String>,
    #[serde(default = "default_command_prefix")]
    pub command_prefix: String,
}
fn default_command_prefix() -> String { "!".to_string() }

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TelegramConfig {
    pub allowed_chat_ids:   Vec<i64>,
    #[serde(default = "default_poll_timeout")]
    pub poll_timeout_secs: u64,
}
fn default_poll_timeout() -> u64 { 30 }

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct MatrixConfig {
    pub homeserver_url: String,
    pub username:       String,
    pub allowed_rooms:  Vec<String>,
    pub allowed_users:  Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EmailConfig {
    pub imap_host:           String,
    #[serde(default = "default_imap_port")]
    pub imap_port:           u16,
    pub imap_username:       String,
    #[serde(default = "default_subject_prefix")]
    pub subject_prefix:      String,
    pub smtp_host:           String,
    pub smtp_username:       String,
    pub smtp_from:           String,
    pub allowed_from_addrs:  Vec<String>,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs:  u64,
    pub delete_processed:    bool,
}
fn default_imap_port() -> u16 { 993 }
fn default_subject_prefix() -> String { "[BONSAI]".to_string() }
fn default_poll_interval() -> u64 { 30 }

// ── Tuning ────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BackpressureConfig {
    pub inbound_queue_capacity:  usize,
    pub worker_count:            usize,
    pub global_semaphore:        usize,
    pub per_platform_send_queue: usize,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            inbound_queue_capacity:  1024,
            worker_count:            8,
            global_semaphore:        64,
            per_platform_send_queue: 256,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CircuitBreakerConfig {
    pub open_after_failures:  u32,
    pub half_open_probe_secs: u64,
    pub close_on_successes:   u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            open_after_failures:  5,
            half_open_probe_secs: 30,
            close_on_successes:   1,
        }
    }
}

// ── Swarm peers ───────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SwarmPeer {
    /// Human name for this peer (used in logs and routing rules)
    pub name:      String,
    /// Full URL of the peer's admin API (e.g. "http://10.0.0.2:11666")
    pub admin_url: String,
    /// Admin token for the peer's API
    pub token:     String,
    /// Message routing rules: if the inbound text contains any of these
    /// keywords, prefer forwarding to this peer over handling locally.
    #[serde(default)]
    pub route_keywords: Vec<String>,
}

// ── Keyring ───────────────────────────────────────────────────────────────────

const KEYRING_SERVICE: &str = "bonsai-bot";

pub fn keyring_get(account: &str) -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, account)
        .ok()
        .and_then(|e| e.get_password().ok())
}

pub fn keyring_set(account: &str, value: &str) -> Result<(), String> {
    keyring::Entry::new(KEYRING_SERVICE, account)
        .map_err(|e| e.to_string())?
        .set_password(value)
        .map_err(|e| e.to_string())
}

#[allow(dead_code)]
pub fn keyring_delete(account: &str) -> Result<(), String> {
    keyring::Entry::new(KEYRING_SERVICE, account)
        .map_err(|e: keyring::Error| e.to_string())?
        .delete_password()
        .map_err(|e: keyring::Error| e.to_string())
}

/// Ensure `bot_admin_token` exists in keychain, creating one if absent.
pub fn ensure_admin_token() -> Result<String, String> {
    if let Some(tok) = keyring_get("bot_admin_token") {
        return Ok(tok);
    }
    let tok = uuid::Uuid::new_v4().to_string();
    keyring_set("bot_admin_token", &tok)?;
    tracing::info!("[config] Generated new bot_admin_token and stored in keychain");
    Ok(tok)
}

// ── Load / save ───────────────────────────────────────────────────────────────

pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("bonsai"))
}

pub fn config_path() -> PathBuf {
    config_dir()
        .map(|d| d.join("bonsai-bot-config.json"))
        .unwrap_or_else(|| PathBuf::from("bonsai-bot-config.json"))
}

pub fn load_config() -> BotConfig {
    let path = config_path();
    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(s) => match serde_json::from_str::<BotConfig>(&s) {
                Ok(c) => return c,
                Err(e) => tracing::warn!("[config] Parse error (using defaults): {e}"),
            },
            Err(e) => tracing::warn!("[config] Read error (using defaults): {e}"),
        }
    }
    let cfg = BotConfig::default();
    save_config(&cfg);
    cfg
}

pub fn save_config(cfg: &BotConfig) {
    let path = config_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match serde_json::to_string_pretty(cfg) {
        Ok(s) => { let _ = std::fs::write(&path, s); }
        Err(e) => tracing::error!("[config] Serialize error: {e}"),
    }
}
