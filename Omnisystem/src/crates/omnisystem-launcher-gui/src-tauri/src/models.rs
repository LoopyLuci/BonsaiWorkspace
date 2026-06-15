// Data models for Omnisystem Launcher

use serde::{Deserialize, Serialize};

// ============================================================================
// Application Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub icon: Option<String>,
    pub category: String,
    pub executable: String,
    pub args: Option<Vec<String>>,
    pub working_dir: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInstance {
    pub id: String,
    pub app_id: String,
    pub app_name: String,
    pub status: String,
    pub pid: u32,
    pub memory_mb: u64,
    pub cpu_percent: f64,
    pub launched_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchResult {
    pub success: bool,
    pub instance_id: Option<String>,
    pub message: String,
    pub error: Option<String>,
}

// ============================================================================
// System Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub healthy: bool,
    pub uptime_seconds: u64,
    pub active_instances: usize,
    pub total_apps: usize,
    pub memory_used_mb: u64,
    pub memory_available_mb: u64,
    pub cpu_cores: usize,
    pub load_average: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub running: bool,
    pub address: String,
    pub port: u16,
    pub uptime_seconds: u64,
    pub connections: usize,
    pub version: String,
    pub last_heartbeat: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub port: u16,
    pub host: String,
    pub log_level: String,
    pub auto_start: bool,
    pub dark_mode: bool,
    pub theme: String,
    pub window_remember_state: bool,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "127.0.0.1".to_string(),
            log_level: "info".to_string(),
            auto_start: true,
            dark_mode: true,
            theme: "modern".to_string(),
            window_remember_state: true,
        }
    }
}

// ============================================================================
// UI Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub maximized: bool,
    pub always_on_top: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: i64,
    pub level: String,
    pub message: String,
    pub source: String,
}

// ============================================================================
// IPC Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPCRequest {
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPCResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}
