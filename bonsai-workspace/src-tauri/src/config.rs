use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub const DEFAULT_API_HOST: &str = "127.0.0.1";
pub const DEFAULT_API_PORT: u16 = 11369;   // Bonsai Workspace
pub const BUDDY_API_PORT:   u16 = 11420;   // Bonsai Buddy

fn default_buddy_api_port() -> u16 { BUDDY_API_PORT }

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
    pub desktop_connection_token: Option<String>,
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
            desktop_connection_token: None,
            assistant_window_open: false,
            usb_lab_window_open: false,
            main_window_x: None,
            main_window_y: None,
            main_window_width: None,
            main_window_height: None,
        }
    }
}

fn config_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
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
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(config.clone())
}
