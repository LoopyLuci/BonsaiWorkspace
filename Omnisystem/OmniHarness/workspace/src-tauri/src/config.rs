use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use crate::inference_mode::InferenceMode;

pub const DEFAULT_API_HOST: &str = "127.0.0.1";
pub const DEFAULT_API_PORT: u16 = 11369; // Bonsai Workspace
pub const BUDDY_API_PORT: u16 = 11420; // Bonsai Buddy

fn default_buddy_api_port() -> u16 {
    BUDDY_API_PORT
}
fn default_inference_mode() -> InferenceMode {
    InferenceMode::default()
}
fn default_critic_threshold() -> f32 {
    0.55
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
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
    /// Directory where LoRA adapters are stored. Defaults to ~/.bonsai/adapters.
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
    Ok(app_data_dir.join("bonsai-config.json"))
}

pub fn load_config(app_handle: &AppHandle) -> Result<AppConfig, String> {
    let path = config_path(app_handle)?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
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
            key: "bonsai-config.json".into(),
            old_value: before_hash.unwrap_or_default(),
            new_value: after_hash,
        });
    }

    Ok(config.clone())
}
