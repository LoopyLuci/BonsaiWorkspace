//! Workstream H — Device Manager: Unified peripheral control
//!
//! Discovers, configures, and monitors all connected hardware peripherals:
//! displays, input devices, audio interfaces, network adapters, storage, GPUs.
//!
//! Hotplug events trigger window reflow (display disconnect) or predictive
//! engine hints (new device attached).  All mutations are async-safe via
//! RwLock-guarded Vec fields.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, warn};

// ─────────────────────────────────────────────────────────────────────────────
// § 1 — Device types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Display {
    pub id: String,
    pub name: String,
    pub resolution: (u32, u32),
    pub refresh_rate: u32,
    pub hdr_capable: bool,
    pub color_profile: ColorProfile,
    pub is_primary: bool,
    pub scaling: f32,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ColorProfile {
    pub name: String,
    pub gamma: f32,
    pub brightness: u8,
    pub contrast: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub resolution: (u32, u32),
    pub refresh_rate: u32,
    pub scaling: f32,
    pub hdr_enabled: bool,
    pub color_profile: Option<ColorProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputDevice {
    pub id: String,
    pub device_type: InputDeviceType,
    pub name: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub is_wireless: bool,
    pub battery_level: Option<u8>,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputDeviceType {
    Keyboard,
    Mouse,
    Touchpad,
    Touchscreen,
    Stylus,
    GameController,
    Microphone,
    Camera,
    FingerprintReader,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_input: bool,
    pub is_output: bool,
    pub is_default: bool,
    pub sample_rates: Vec<u32>,
    pub channels: u8,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAdapter {
    pub id: String,
    pub name: String,
    pub mac_address: String,
    pub adapter_type: NetworkAdapterType,
    pub is_connected: bool,
    pub ip_addresses: Vec<String>,
    pub link_speed_mbps: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkAdapterType {
    Ethernet,
    WiFi,
    Bluetooth,
    Loopback,
    Virtual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageDevice {
    pub id: String,
    pub name: String,
    pub device_type: StorageType,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub mount_point: Option<String>,
    pub is_removable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    NvmeSsd,
    SataSsd,
    HardDisk,
    UsbDrive,
    SdCard,
    NetworkShare,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDevice {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub vram_mb: u64,
    pub driver_version: String,
    pub has_vulkan: bool,
    pub has_directml: bool,
    pub temperature_c: Option<u32>,
    pub utilization_pct: Option<u8>,
}

// ─────────────────────────────────────────────────────────────────────────────
// § 2 — Inventory and events
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInventory {
    pub displays: Vec<Display>,
    pub inputs: Vec<InputDevice>,
    pub audio: Vec<AudioDevice>,
    pub network: Vec<NetworkAdapter>,
    pub storage: Vec<StorageDevice>,
    pub gpus: Vec<GpuDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HotplugEvent {
    Connected {
        device_id: String,
        device_class: String,
        name: String,
    },
    Disconnected {
        device_id: String,
        device_class: String,
    },
    BatteryLow {
        device_id: String,
        level: u8,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAction {
    pub device_id: String,
    pub action: String,
    pub parameter: Option<serde_json::Value>,
    pub reason: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// § 3 — DeviceManager
// ─────────────────────────────────────────────────────────────────────────────

pub struct DeviceManager {
    pub displays: RwLock<Vec<Display>>,
    pub input_devices: RwLock<Vec<InputDevice>>,
    pub audio_devices: RwLock<Vec<AudioDevice>>,
    pub network_adapters: RwLock<Vec<NetworkAdapter>>,
    pub storage_devices: RwLock<Vec<StorageDevice>>,
    pub gpu_devices: RwLock<Vec<GpuDevice>>,
    pub hotplug_log: RwLock<Vec<HotplugEvent>>,
}

impl DeviceManager {
    pub fn new() -> Arc<Self> {
        let mgr = Arc::new(Self {
            displays: RwLock::new(Self::enumerate_displays()),
            input_devices: RwLock::new(Self::enumerate_inputs()),
            audio_devices: RwLock::new(Self::enumerate_audio()),
            network_adapters: RwLock::new(Self::enumerate_network()),
            storage_devices: RwLock::new(Self::enumerate_storage()),
            gpu_devices: RwLock::new(Self::enumerate_gpus()),
            hotplug_log: RwLock::new(Vec::new()),
        });
        info!("[device-mgr] initialised");
        mgr
    }

    // ── Static enumeration stubs ──────────────────────────────────────────────
    // Real impl would call platform APIs (Win32/udev/IOKit).
    // These return plausible defaults so the rest of the system compiles.

    fn enumerate_displays() -> Vec<Display> {
        vec![Display {
            id: "display-0".into(),
            name: "Primary Display".into(),
            resolution: (1920, 1080),
            refresh_rate: 60,
            hdr_capable: false,
            color_profile: ColorProfile {
                name: "sRGB".into(),
                gamma: 2.2,
                brightness: 80,
                contrast: 75,
            },
            is_primary: true,
            scaling: 1.0,
            connected: true,
        }]
    }

    fn enumerate_inputs() -> Vec<InputDevice> {
        vec![
            InputDevice {
                id: "kbd-0".into(),
                device_type: InputDeviceType::Keyboard,
                name: "System Keyboard".into(),
                vendor_id: 0,
                product_id: 0,
                is_wireless: false,
                battery_level: None,
                connected: true,
            },
            InputDevice {
                id: "mouse-0".into(),
                device_type: InputDeviceType::Mouse,
                name: "System Mouse".into(),
                vendor_id: 0,
                product_id: 0,
                is_wireless: false,
                battery_level: None,
                connected: true,
            },
        ]
    }

    fn enumerate_audio() -> Vec<AudioDevice> {
        vec![AudioDevice {
            id: "audio-out-0".into(),
            name: "Default Audio Output".into(),
            is_input: false,
            is_output: true,
            is_default: true,
            sample_rates: vec![44100, 48000],
            channels: 2,
            connected: true,
        }]
    }

    fn enumerate_network() -> Vec<NetworkAdapter> {
        vec![NetworkAdapter {
            id: "net-lo".into(),
            name: "Loopback".into(),
            mac_address: "00:00:00:00:00:00".into(),
            adapter_type: NetworkAdapterType::Loopback,
            is_connected: true,
            ip_addresses: vec!["127.0.0.1".into()],
            link_speed_mbps: None,
        }]
    }

    fn enumerate_storage() -> Vec<StorageDevice> {
        vec![StorageDevice {
            id: "storage-0".into(),
            name: "System Drive".into(),
            device_type: StorageType::NvmeSsd,
            total_bytes: 512 * 1024 * 1024 * 1024,
            free_bytes: 256 * 1024 * 1024 * 1024,
            mount_point: Some("/".into()),
            is_removable: false,
        }]
    }

    fn enumerate_gpus() -> Vec<GpuDevice> {
        vec![GpuDevice {
            id: "gpu-0".into(),
            name: "System GPU".into(),
            vendor: "Unknown".into(),
            vram_mb: 8192,
            driver_version: "0.0.0".into(),
            has_vulkan: false,
            has_directml: false,
            temperature_c: None,
            utilization_pct: None,
        }]
    }

    // ── Public API ────────────────────────────────────────────────────────────

    pub async fn enumerate(&self) -> DeviceInventory {
        DeviceInventory {
            displays: self.displays.read().await.clone(),
            inputs: self.input_devices.read().await.clone(),
            audio: self.audio_devices.read().await.clone(),
            network: self.network_adapters.read().await.clone(),
            storage: self.storage_devices.read().await.clone(),
            gpus: self.gpu_devices.read().await.clone(),
        }
    }

    pub async fn configure_display(&self, id: &str, config: &DisplayConfig) -> Result<(), String> {
        let mut displays = self.displays.write().await;
        let display = displays
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or_else(|| format!("Display '{}' not found", id))?;
        display.resolution = config.resolution;
        display.refresh_rate = config.refresh_rate;
        display.scaling = config.scaling;
        display.hdr_capable = config.hdr_enabled || display.hdr_capable;
        if let Some(profile) = &config.color_profile {
            display.color_profile = profile.clone();
        }
        info!(
            "[device-mgr] display {} configured: {:?}",
            id, config.resolution
        );
        Ok(())
    }

    pub async fn set_default_audio_output(&self, id: &str) -> Result<(), String> {
        let mut audio = self.audio_devices.write().await;
        let mut found = false;
        for dev in audio.iter_mut() {
            if dev.id == id && dev.is_output {
                dev.is_default = true;
                found = true;
            } else if dev.is_output {
                dev.is_default = false;
            }
        }
        if found {
            info!("[device-mgr] default audio output set to {}", id);
            Ok(())
        } else {
            Err(format!("Audio output device '{}' not found", id))
        }
    }

    pub async fn input_device_info(&self, id: &str) -> Option<InputDevice> {
        self.input_devices
            .read()
            .await
            .iter()
            .find(|d| d.id == id)
            .cloned()
    }

    /// AI-powered peripheral optimization heuristic.
    pub async fn ai_optimize_peripheral(&self, complaint: &str) -> Vec<DeviceAction> {
        let lc = complaint.to_lowercase();
        let mut actions = Vec::new();

        if lc.contains("mouse") && (lc.contains("slow") || lc.contains("sluggish")) {
            actions.push(DeviceAction {
                device_id: "mouse-0".into(),
                action: "set_dpi".into(),
                parameter: Some(serde_json::json!({ "dpi": 1200 })),
                reason: "Increase DPI to improve mouse responsiveness".into(),
            });
            actions.push(DeviceAction {
                device_id: "mouse-0".into(),
                action: "set_polling_rate".into(),
                parameter: Some(serde_json::json!({ "hz": 1000 })),
                reason: "Increase polling rate for smoother tracking".into(),
            });
        } else if lc.contains("display") && lc.contains("blur") {
            actions.push(DeviceAction {
                device_id: "display-0".into(),
                action: "set_scaling".into(),
                parameter: Some(serde_json::json!({ "scaling": 1.0 })),
                reason: "Reset scaling to native resolution for sharpest output".into(),
            });
        } else if lc.contains("audio") && lc.contains("crackl") {
            actions.push(DeviceAction {
                device_id: "audio-out-0".into(),
                action: "set_sample_rate".into(),
                parameter: Some(serde_json::json!({ "hz": 44100 })),
                reason: "Lower sample rate may resolve audio glitches".into(),
            });
        } else {
            actions.push(DeviceAction {
                device_id: "unknown".into(),
                action: "run_diagnostics".into(),
                parameter: None,
                reason: format!("Run diagnostics for: {}", complaint),
            });
        }

        actions
    }

    pub async fn on_hotplug(&self, event: HotplugEvent) {
        info!("[device-mgr] hotplug: {:?}", event);
        match &event {
            HotplugEvent::Connected {
                device_id,
                device_class,
                name,
            } => {
                if device_class == "display" {
                    self.displays.write().await.push(Display {
                        id: device_id.clone(),
                        name: name.clone(),
                        resolution: (1920, 1080),
                        refresh_rate: 60,
                        hdr_capable: false,
                        color_profile: ColorProfile::default(),
                        is_primary: false,
                        scaling: 1.0,
                        connected: true,
                    });
                }
            }
            HotplugEvent::Disconnected {
                device_id,
                device_class,
            } => {
                if device_class == "display" {
                    let mut displays = self.displays.write().await;
                    if let Some(d) = displays.iter_mut().find(|d| &d.id == device_id) {
                        d.connected = false;
                    }
                }
            }
            HotplugEvent::BatteryLow { device_id, level } => {
                warn!(
                    "[device-mgr] battery low: device={} level={}%",
                    device_id, level
                );
            }
        }
        self.hotplug_log.write().await.push(event);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 4 — Tauri commands
// ─────────────────────────────────────────────────────────────────────────────

use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn omni_devices_list(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let inv = state.device_manager.enumerate().await;
    Ok(serde_json::to_value(inv).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn omni_display_config(
    display_id: String,
    resolution_w: u32,
    resolution_h: u32,
    refresh_rate: u32,
    scaling: f32,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let cfg = DisplayConfig {
        resolution: (resolution_w, resolution_h),
        refresh_rate,
        scaling,
        hdr_enabled: false,
        color_profile: None,
    };
    state
        .device_manager
        .configure_display(&display_id, &cfg)
        .await?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
pub async fn omni_audio_device_set(
    device_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    state
        .device_manager
        .set_default_audio_output(&device_id)
        .await?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
pub async fn omni_input_device_info(
    device_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    match state.device_manager.input_device_info(&device_id).await {
        Some(dev) => Ok(serde_json::to_value(dev).map_err(|e| e.to_string())?),
        None => Err(format!("Device '{}' not found", device_id)),
    }
}

#[tauri::command]
pub async fn omni_device_optimize(
    complaint: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let actions = state
        .device_manager
        .ai_optimize_peripheral(&complaint)
        .await;
    Ok(serde_json::to_value(actions).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn omni_device_hotplug(
    event_json: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let event: HotplugEvent = serde_json::from_value(event_json).map_err(|e| e.to_string())?;
    state.device_manager.on_hotplug(event).await;
    Ok(serde_json::json!({ "ok": true }))
}
