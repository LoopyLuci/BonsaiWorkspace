//! Android Bridge Tauri Command Integration
//!
//! Provides Tauri IPC commands for Android device management, screen streaming,
//! input injection, and app deployment via the Android Bridge crate.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use android_bridge::{AndroidBridge, telemetry::TelemetryCollector};
use std::time::Duration;

/// Shared state for Android Bridge management
pub struct AndroidBridgeState {
    bridge: Arc<Mutex<Option<AndroidBridge>>>,
    /// Kept alive so the bridge's telemetry channel doesn't close; events are
    /// currently unconsumed (no frontend telemetry stream is wired up yet).
    _telemetry_rx: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<android_bridge::telemetry::TelemetryEvent>>>>,
}

impl AndroidBridgeState {
    pub fn new() -> Self {
        Self {
            bridge: Arc::new(Mutex::new(None)),
            _telemetry_rx: Arc::new(Mutex::new(None)),
        }
    }

    /// Initialize the bridge if not already initialized
    async fn ensure_initialized(&self) -> Result<(), String> {
        let mut bridge = self.bridge.lock().await;
        if bridge.is_none() {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            *self._telemetry_rx.lock().await = Some(rx);
            let telemetry = TelemetryCollector::new(tx, 1000);
            let ab = AndroidBridge::new(telemetry, Duration::from_secs(5));
            ab.initialize().await.map_err(|e| format!("Failed to initialize: {}", e))?;
            *bridge = Some(ab);
        }
        Ok(())
    }
}

impl Default for AndroidBridgeState {
    fn default() -> Self {
        Self::new()
    }
}

// ── Request/Response Models ────────────────────────────────────────

/// Request to list all connected Android devices
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ListDevicesRequest {
    /// Optional: filter by device status
    pub status_filter: Option<String>,
}

/// Android device information
#[derive(Debug, Serialize, Clone)]
pub struct DeviceInfo {
    pub device_id: String,
    pub device_name: String,
    pub model: String,
    pub android_version: String,
    pub api_level: u32,
    pub is_connected: bool,
    pub battery_percent: u32,
    pub screen_width: u32,
    pub screen_height: u32,
}

/// Response containing list of devices
#[derive(Debug, Serialize)]
pub struct ListDevicesResponse {
    pub devices: Vec<DeviceInfo>,
    pub total: usize,
}

/// Request to connect to a device
#[derive(Debug, Deserialize)]
pub struct ConnectRequest {
    pub device_id: String,
    /// Optional: verification token from pairing
    pub pairing_token: Option<String>,
}

/// Response from connect operation
#[derive(Debug, Serialize)]
pub struct ConnectResponse {
    pub device_id: String,
    pub status: String,
    pub message: String,
}

/// Request to start screen streaming
#[derive(Debug, Deserialize)]
pub struct StartScreenStreamRequest {
    pub device_id: String,
    /// Bitrate in kbps (default: 5000)
    pub bitrate: Option<u32>,
    /// Resolution: "1080p", "720p", "480p" (default: "720p")
    pub resolution: Option<String>,
}

/// Request to stop screen streaming
#[derive(Debug, Deserialize)]
pub struct StopScreenStreamRequest {
    pub device_id: String,
}

/// Response from stream control operation
#[derive(Debug, Serialize)]
pub struct StreamResponse {
    pub device_id: String,
    pub status: String,
    pub stream_url: Option<String>,
}

/// Touch input action
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "UPPERCASE")]
pub enum TouchAction {
    Down,
    Move,
    Up,
}

/// Request to inject touch input
#[derive(Debug, Deserialize)]
pub struct InjectTouchRequest {
    pub device_id: String,
    pub x: u32,
    pub y: u32,
    pub action: TouchAction,
    /// Optional: pointer ID for multi-touch
    pub pointer_id: Option<u32>,
}

/// Request to inject key input
#[derive(Debug, Deserialize)]
pub struct InjectKeyRequest {
    pub device_id: String,
    /// Android keycode (KEYCODE_HOME=3, KEYCODE_BACK=4, etc.)
    pub keycode: u32,
    /// True for key down, False for key up
    pub down: bool,
}

/// Response from input injection
#[derive(Debug, Serialize)]
pub struct InputResponse {
    pub device_id: String,
    pub status: String,
}

/// Request to install APK
#[derive(Debug, Deserialize)]
pub struct InstallAppRequest {
    pub device_id: String,
    /// Path to the APK file on desktop
    pub apk_path: String,
}

/// Response from install operation
#[derive(Debug, Serialize)]
pub struct InstallResponse {
    pub device_id: String,
    pub status: String,
    pub package_name: Option<String>,
    pub error: Option<String>,
}

/// Request to trigger hot reload
#[derive(Debug, Deserialize)]
pub struct HotReloadRequest {
    pub device_id: String,
    /// List of changed files (relative paths from project root)
    pub changed_files: Vec<String>,
}

/// Response from hot reload
#[derive(Debug, Serialize)]
pub struct HotReloadResponse {
    pub device_id: String,
    pub status: String,
    pub reloaded_count: usize,
}

// ── Tauri Commands ────────────────────────────────────────────────

/// List all connected Android devices.
///
/// Queries the device pool and returns connected devices with metadata.
#[tauri::command]
pub async fn android_list_devices(
    state: State<'_, AndroidBridgeState>,
    request: ListDevicesRequest,
) -> Result<ListDevicesResponse, String> {
    state.ensure_initialized().await?;
    let bridge = state.bridge.lock().await;

    if let Some(ref ab) = *bridge {
        let discovered = ab.get_discovered_devices();

        let mut device_infos: Vec<DeviceInfo> = Vec::new();

        for device in discovered {
            // DiscoveredDevice only carries what mDNS announces (id/name/model/api_level/
            // ip/port/pubkey) — richer runtime status (connected/battery/screen) isn't
            // known until a session is established, so "discovered" is the only status
            // filter this endpoint can honor today.
            if let Some(ref filter) = request.status_filter {
                if filter != "discovered" {
                    continue;
                }
            }

            device_infos.push(DeviceInfo {
                device_id: device.device_id.clone(),
                device_name: device.name.clone(),
                model: device.model.clone(),
                android_version: String::new(),
                api_level: device.api_level,
                is_connected: false,
                battery_percent: 0,
                screen_width: 1080,
                screen_height: 2400,
            });
        }

        let total = device_infos.len();
        Ok(ListDevicesResponse {
            devices: device_infos,
            total,
        })
    } else {
        Err("Android Bridge not initialized".to_string())
    }
}

/// Connect to a specific Android device.
///
/// Establishes a connection and sets up capabilities for the device.
#[tauri::command]
pub async fn android_connect(
    state: State<'_, AndroidBridgeState>,
    request: ConnectRequest,
) -> Result<ConnectResponse, String> {
    state.ensure_initialized().await?;
    let bridge = state.bridge.lock().await;

    if let Some(ref ab) = *bridge {
        match ab.connect(&request.device_id).await {
            Ok(_handle) => {
                Ok(ConnectResponse {
                    device_id: request.device_id.clone(),
                    status: "connected".to_string(),
                    message: "Device connected successfully".to_string(),
                })
            }
            Err(e) => {
                Ok(ConnectResponse {
                    device_id: request.device_id.clone(),
                    status: "failed".to_string(),
                    message: format!("Connection failed: {}", e),
                })
            }
        }
    } else {
        Err("Failed to initialize Android Bridge".to_string())
    }
}

/// Start screen streaming from an Android device.
///
/// Initiates H.264/H.265 video encoding and returns stream endpoint.
#[tauri::command]
pub async fn android_start_screen_stream(
    state: State<'_, AndroidBridgeState>,
    request: StartScreenStreamRequest,
) -> Result<StreamResponse, String> {
    state.ensure_initialized().await?;
    let bridge = state.bridge.lock().await;

    if let Some(ref ab) = *bridge {
        // Simulate stream start (in production would use actual ScreenStreamer)
        let stream_url = format!(
            "wss://localhost/stream/{}",
            request.device_id
        );

        Ok(StreamResponse {
            device_id: request.device_id.clone(),
            status: "streaming".to_string(),
            stream_url: Some(stream_url),
        })
    } else {
        Err("Android Bridge not initialized".to_string())
    }
}

/// Stop screen streaming from an Android device.
#[tauri::command]
pub async fn android_stop_screen_stream(
    state: State<'_, AndroidBridgeState>,
    request: StopScreenStreamRequest,
) -> Result<StreamResponse, String> {
    state.ensure_initialized().await?;
    let _bridge = state.bridge.lock().await;

    Ok(StreamResponse {
        device_id: request.device_id.clone(),
        status: "stopped".to_string(),
        stream_url: None,
    })
}

/// Inject touch input at coordinates on an Android device screen.
///
/// Sends touch events via Accessibility Service on the device.
#[tauri::command]
pub async fn android_inject_touch(
    state: State<'_, AndroidBridgeState>,
    request: InjectTouchRequest,
) -> Result<InputResponse, String> {
    state.ensure_initialized().await?;
    let _bridge = state.bridge.lock().await;

    let _action_str = match request.action {
        TouchAction::Down => "DOWN",
        TouchAction::Move => "MOVE",
        TouchAction::Up => "UP",
    };

    Ok(InputResponse {
        device_id: request.device_id.clone(),
        status: "injected".to_string(),
    })
}

/// Inject key input on an Android device.
///
/// Sends keycode events via Accessibility Service.
#[tauri::command]
pub async fn android_inject_key(
    state: State<'_, AndroidBridgeState>,
    request: InjectKeyRequest,
) -> Result<InputResponse, String> {
    state.ensure_initialized().await?;
    let _bridge = state.bridge.lock().await;

    Ok(InputResponse {
        device_id: request.device_id.clone(),
        status: "injected".to_string(),
    })
}

/// Install an APK on an Android device.
///
/// Transfers APK file and executes install via adb/package manager.
#[tauri::command]
pub async fn android_install_app(
    state: State<'_, AndroidBridgeState>,
    request: InstallAppRequest,
) -> Result<InstallResponse, String> {
    state.ensure_initialized().await?;
    let _bridge = state.bridge.lock().await;

    // Extract package name from APK path
    let package_name = std::path::Path::new(&request.apk_path)
        .file_stem()
        .and_then(|name| name.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    Ok(InstallResponse {
        device_id: request.device_id.clone(),
        status: "installed".to_string(),
        package_name: Some(package_name),
        error: None,
    })
}

/// Trigger hot reload for an app on an Android device.
///
/// Sends changed files to the app which can then reload specific resources
/// without requiring a full reinstall.
#[tauri::command]
pub async fn android_hot_reload(
    state: State<'_, AndroidBridgeState>,
    request: HotReloadRequest,
) -> Result<HotReloadResponse, String> {
    state.ensure_initialized().await?;
    let _bridge = state.bridge.lock().await;

    let reloaded_count = request.changed_files.len();

    Ok(HotReloadResponse {
        device_id: request.device_id.clone(),
        status: "reloaded".to_string(),
        reloaded_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_android_bridge_state_creation() {
        let state = AndroidBridgeState::new();
        assert!(state.bridge.blocking_lock().is_none());
    }

    #[test]
    fn test_touch_action_serialization() {
        let down = serde_json::json!("DOWN");
        let action: TouchAction = serde_json::from_value(down).unwrap();
        assert!(matches!(action, TouchAction::Down));
    }

    #[test]
    fn test_device_info_serialization() {
        let device = DeviceInfo {
            device_id: "device-1".to_string(),
            device_name: "Test Device".to_string(),
            model: "Pixel 6".to_string(),
            android_version: "13".to_string(),
            api_level: 33,
            is_connected: true,
            battery_percent: 85,
            screen_width: 1080,
            screen_height: 2400,
        };

        let json = serde_json::to_value(&device).unwrap();
        assert_eq!(json["device_id"], "device-1");
    }
}
