use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn error(error: String) -> Self {
        ApiResponse {
            success: false,
            data: None,
            error: Some(error),
            timestamp: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallRequest {
    pub app_id: String,
    pub version: Option<String>,
    pub force: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRequest {
    pub version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigRequest {
    pub config: Option<HashMap<String, serde_json::Value>>,
    pub debug: Option<bool>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppInfo {
    pub app_id: String,
    pub version: String,
    pub state: String,
    pub installed_at: Option<String>,
    pub running: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemHealth {
    pub healthy: bool,
    pub uptime_seconds: u64,
    pub installed_apps: usize,
    pub running_apps: usize,
}
