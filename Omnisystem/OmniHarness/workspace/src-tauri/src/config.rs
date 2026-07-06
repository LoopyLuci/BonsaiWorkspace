use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use crate::inference_mode::InferenceMode;

pub const DEFAULT_API_HOST: &str = "127.0.0.1";
// Ports below used to sit at 11369/11420/11421/11425/11370 — all inside the
// 11001-11800 TCP range Windows commonly reserves/excludes for Hyper-V/WSL
// (`netsh interface ipv4 show excludedportrange protocol=tcp`), which made
// every bind fail with PermissionDenied (os error 10013) on affected machines.
// Moved to a base clear of every commonly-reserved Windows range.
pub const DEFAULT_API_PORT: u16 = 47100; // Workspace
pub const BUDDY_API_PORT: u16 = 47110; // Workspace Buddy
pub const MCP_PORT: u16 = 47120;
pub const MCP_TELEMETRY_PORT: u16 = 47125;
pub const A2A_PORT: u16 = 47130;
pub const ORCHESTRATOR_CONTROL_PORT: u16 = 47140;

fn default_buddy_api_port() -> u16 {
    BUDDY_API_PORT
}
fn default_inference_mode() -> InferenceMode {
    InferenceMode::default()
}
fn default_critic_threshold() -> f32 {
    0.55
}

/// Bump this whenever a fix changes what a *correct* persisted value should
/// be (not just when adding a new field — `#[serde(default)]` already
/// handles that case for free). Every port-binding bug, the GPU-crash
/// hardcoded-Hybrid-mode bug, and others this session were all the same root
/// cause: a value written to disk by an older, buggier version of this app
/// silently overriding a fixed default forever, because on-disk data always
/// wins over `Default::default()`. `migrate_config` runs once per version
/// bump and resets exactly the values known to be stale from a specific past
/// bug, so the fix in the code actually takes effect instead of needing a
/// human to delete the config file by hand.
pub const CONFIG_SCHEMA_VERSION: u32 = 1;

fn default_schema_version() -> u32 {
    CONFIG_SCHEMA_VERSION
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Absent (defaults to 0) on any config written before this field
    /// existed — treated as "pre-migration" so every historical fixup in
    /// `migrate_config` applies to it once.
    #[serde(default)]
    pub schema_version: u32,
    pub api_host: String,
    pub api_port: u16,
    #[serde(default = "default_buddy_api_port")]
    pub buddy_api_port: u16,
    #[serde(default)]
    pub current_session_id: Option<String>,
    #[serde(default)]
    pub current_session_title: Option<String>,
    #[serde(default)]
    pub desktop_connection_ip: Option<String>,
    #[serde(default)]
    pub assistant_window_open: bool,
    #[serde(default)]
    pub usb_lab_window_open: bool,
    #[serde(default)]
    pub main_window_x: Option<i32>,
    #[serde(default)]
    pub main_window_y: Option<i32>,
    #[serde(default)]
    pub main_window_width: Option<u32>,
    #[serde(default)]
    pub main_window_height: Option<u32>,
    /// Additional directories scanned for .gguf model files beyond the bootstrap path.
    #[serde(default)]
    pub extra_model_dirs: Vec<String>,
    /// Model ID last loaded by the user — restored on next startup.
    #[serde(default)]
    pub last_model_id: Option<String>,
    /// Optional allowlist for MCP server commands. Empty means allow all.
    #[serde(default)]
    pub mcp_allowed_commands: Vec<String>,
    /// Default mode applied to newly discovered local models.
    #[serde(default = "default_inference_mode")]
    pub default_inference_mode: InferenceMode,
    /// Pairing token for the REST management API and QR-code Android pairing.
    /// Regenerated each launch and persisted here so omni-bot can read it.
    #[serde(default)]
    pub pair_token: String,
    /// Set to true when the GPU driver crashed (0xC0000409 / STATUS_STACK_BUFFER_OVERRUN).
    /// When true, models load CPU-only by default. Cleared when the user explicitly
    /// enables GPU layers from Settings.
    #[serde(default)]
    pub gpu_crash_fallback: bool,
    /// Path to a small (0.5–1.5B) draft model for speculative decoding.
    /// When set and the file exists, llama-server is started with `--model-draft`.
    #[serde(default)]
    pub draft_model_path: Option<String>,
    /// Directory where LoRA adapters are stored. Defaults to ~/.workspace/adapters.
    #[serde(default)]
    pub adapters_dir: Option<String>,
    /// Path to LLaVA CLIP mmproj file for vision support.
    #[serde(default)]
    pub vision_mmproj_path: Option<String>,
    /// Whether the critic quality-gate is enabled (auto-retry responses below threshold).
    #[serde(default)]
    pub critic_enabled: bool,
    /// Minimum critic score to accept a response without retry (0.0–1.0).
    #[serde(default = "default_critic_threshold")]
    pub critic_threshold: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: default_schema_version(),
            api_host: DEFAULT_API_HOST.into(),
            api_port: DEFAULT_API_PORT,
            buddy_api_port: BUDDY_API_PORT,
            current_session_id: None,
            current_session_title: None,
            desktop_connection_ip: None,
            assistant_window_open: false,
            usb_lab_window_open: false,
            main_window_x: None,
            main_window_y: None,
            main_window_width: None,
            main_window_height: None,
            extra_model_dirs: Vec::new(),
            last_model_id: None,
            mcp_allowed_commands: Vec::new(),
            default_inference_mode: InferenceMode::default(),
            pair_token: String::new(),
            gpu_crash_fallback: false,
            draft_model_path: None,
            adapters_dir: None,
            vision_mmproj_path: None,
            critic_enabled: true,
            critic_threshold: 0.55,
        }
    }
}

fn config_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
    Ok(app_data_dir.join("workspace-config.json"))
}

pub fn load_config(app_handle: &AppHandle) -> Result<AppConfig, String> {
    let path = config_path(app_handle)?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut config: AppConfig = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    if config.schema_version < CONFIG_SCHEMA_VERSION {
        let from = config.schema_version;
        migrate_config(&mut config);
        config.schema_version = CONFIG_SCHEMA_VERSION;
        tracing::info!("[config-migration] migrated workspace-config.json v{} -> v{}", from, CONFIG_SCHEMA_VERSION);
        // Persist immediately so the migration is a one-time cost, not
        // re-evaluated (and re-logged) on every single startup.
        let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        let _ = crate::atomic_write(&path, content.as_bytes());
    }

    Ok(config)
}

/// Reset exactly the persisted values known to be stale from a specific past
/// bug, gated by the schema version the config was last saved at. Each
/// `if from_version < N` block corresponds to one `CONFIG_SCHEMA_VERSION`
/// bump — never delete an old block, since a config could be jumping
/// multiple versions in one migration (e.g. a user updating after skipping
/// several releases).
fn migrate_config(config: &mut AppConfig) {
    let from_version = config.schema_version;

    if from_version < 1 {
        // v0 -> v1: ports used to default into 11001-11800, a range Windows
        // commonly reserves for Hyper-V/WSL, causing every bind to fail with
        // PermissionDenied. Old configs may still have a stale port
        // persisted from before that fix (see DEFAULT_API_PORT/BUDDY_API_PORT
        // comments above) — reset anything in the known-bad range or at the
        // specific old default values.
        if (11001..=11800).contains(&config.api_port) || config.api_port == 11369 {
            tracing::info!(
                "[config-migration] resetting stale api_port {} -> {}",
                config.api_port, DEFAULT_API_PORT
            );
            config.api_port = DEFAULT_API_PORT;
        }
        if (11001..=11800).contains(&config.buddy_api_port) || config.buddy_api_port == 11420 {
            tracing::info!(
                "[config-migration] resetting stale buddy_api_port {} -> {}",
                config.buddy_api_port, BUDDY_API_PORT
            );
            config.buddy_api_port = BUDDY_API_PORT;
        }

        // `InferenceMode::default()` used to be `Hybrid { gpu_layers: 20 }`,
        // which bypassed the orchestrator's quant-safety/VRAM checks
        // entirely and was the deepest cause of a GPU crash-loop this
        // session. A config that still has that literal old default
        // persisted gets reset to the new `Auto` default so the safety
        // logic actually runs; a value the user deliberately chose
        // (any other gpu_layers count) is left alone.
        if let InferenceMode::Hybrid { gpu_layers: 20 } = config.default_inference_mode {
            tracing::info!("[config-migration] resetting stale default_inference_mode Hybrid{{20}} -> Auto");
            config.default_inference_mode = InferenceMode::Auto;
        }
    }
}

pub fn save_config(app_handle: &AppHandle, config: &AppConfig) -> Result<AppConfig, String> {
    let path = config_path(app_handle)?;

    // Capture before hash for the Universe event
    let before_hash = std::fs::read(&path)
        .ok()
        .map(|b| blake3::hash(&b).to_hex().to_string());

    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    crate::atomic_write(&path, content.as_bytes()).map_err(|e| e.to_string())?;

    // Emit ConfigChanged on the SystemEventBus (best-effort, non-blocking)
    if let Some(state) = app_handle.try_state::<crate::AppState>() {
        let after_hash = blake3::hash(content.as_bytes()).to_hex().to_string();
        state.event_bus.publish(crate::system_event_bus::SystemEvent::ConfigChanged {
            key: "workspace-config.json".into(),
            old_value: before_hash.unwrap_or_default(),
            new_value: after_hash,
        });
    }

    Ok(config.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_stale_excluded_range_ports() {
        // 11001..=11800 is the actual Windows-excluded range this bug hit;
        // 11801 (one past the boundary) is deliberately NOT used here — that
        // value is what `find_free_port`'s scan-forward landed on in the real
        // incident, and it's a working, non-excluded port, not a broken one
        // (see `leaves_deliberately_chosen_ports_alone`, which pins that
        // migration must not stomp values outside the actually-broken range).
        let mut config = AppConfig { schema_version: 0, api_port: 11500, buddy_api_port: 11600, ..AppConfig::default() };
        migrate_config(&mut config);
        assert_eq!(config.api_port, DEFAULT_API_PORT);
        assert_eq!(config.buddy_api_port, BUDDY_API_PORT);
    }

    #[test]
    fn migrates_stale_old_default_ports() {
        let mut config = AppConfig { schema_version: 0, api_port: 11369, buddy_api_port: 11420, ..AppConfig::default() };
        migrate_config(&mut config);
        assert_eq!(config.api_port, DEFAULT_API_PORT);
        assert_eq!(config.buddy_api_port, BUDDY_API_PORT);
    }

    #[test]
    fn leaves_deliberately_chosen_ports_alone() {
        // A port outside the excluded range and not the old default is a
        // real user/deployment choice — migration must not stomp it.
        let mut config = AppConfig { schema_version: 0, api_port: 55555, buddy_api_port: 55556, ..AppConfig::default() };
        migrate_config(&mut config);
        assert_eq!(config.api_port, 55555);
        assert_eq!(config.buddy_api_port, 55556);
    }

    #[test]
    fn migrates_stale_hardcoded_hybrid_default() {
        let mut config = AppConfig {
            schema_version: 0,
            default_inference_mode: InferenceMode::Hybrid { gpu_layers: 20 },
            ..AppConfig::default()
        };
        migrate_config(&mut config);
        assert!(matches!(config.default_inference_mode, InferenceMode::Auto));
    }

    #[test]
    fn leaves_deliberately_chosen_hybrid_layer_count_alone() {
        let mut config = AppConfig {
            schema_version: 0,
            default_inference_mode: InferenceMode::Hybrid { gpu_layers: 12 },
            ..AppConfig::default()
        };
        migrate_config(&mut config);
        assert!(matches!(config.default_inference_mode, InferenceMode::Hybrid { gpu_layers: 12 }));
    }

    #[test]
    fn migrate_config_is_gated_by_schema_version_not_just_value() {
        // A config already at the current schema version is assumed to have
        // been through this exact migration before (or never had the stale
        // value at all) — `migrate_config` gates each fixup on
        // `from_version`, so calling it again must not re-touch a value that
        // was presumably a deliberate choice made after migrating.
        let mut config = AppConfig { schema_version: CONFIG_SCHEMA_VERSION, api_port: 11801, ..AppConfig::default() };
        migrate_config(&mut config);
        assert_eq!(config.api_port, 11801);
    }
}
