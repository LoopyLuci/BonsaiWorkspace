# Android Bridge Integration Guide

## Overview

This guide covers integrating the Android Bridge into the Bonsai Workspace IDE, MCP Agent system, and BTI Command interface.

## Part 1: Tauri IDE Integration

### 1.1 Create Android Commands Module

File: `bonsai-workspace/src-tauri/src/android_commands.rs`

```rust
use bonsai_android_bridge::{
    AndroidBridge, CapabilityType, streaming::BitrateConfig,
    input::InputEvent, device::DeviceStatus,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

/// Device info DTO for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub model: String,
    pub api_level: u32,
    pub status: String,
    pub ip: String,
    pub port: u16,
    pub battery_level: Option<u8>,
    pub screen_fps: u8,
    pub uptime_seconds: Option<u64>,
}

impl DeviceInfo {
    fn from_device(device: &bonsai_android_bridge::device::Device) -> Self {
        Self {
            id: device.id.clone(),
            name: device.name.clone(),
            model: device.model.clone(),
            api_level: device.api_level,
            status: format!("{:?}", device.status),
            ip: device.ip.clone(),
            port: device.port,
            battery_level: device.metrics.battery_level,
            screen_fps: 60, // From metrics
            uptime_seconds: device.get_uptime(),
        }
    }
}

/// List all discovered Android devices
#[tauri::command]
pub async fn android_list_devices(
    state: State<'_, AndroidBridge>,
) -> Result<Vec<DeviceInfo>, String> {
    let devices = state.get_device_pool().get_all_devices();
    Ok(devices.into_iter().map(|d| DeviceInfo::from_device(&d)).collect())
}

/// Register a device manually
#[tauri::command]
pub async fn android_register_device(
    state: State<'_, AndroidBridge>,
    device_id: String,
    name: String,
    model: String,
    api_level: u32,
    ip: String,
    port: u16,
    public_key: String,
) -> Result<(), String> {
    state
        .register_device(device_id, name, model, api_level, ip, port, public_key)
        .await
        .map_err(|e| e.to_string())
}

/// Connect to a device
#[tauri::command]
pub async fn android_connect(
    state: State<'_, AndroidBridge>,
    device_id: String,
) -> Result<String, String> {
    let handle = state.connect(&device_id).await.map_err(|e| e.to_string())?;
    Ok(handle.device_id)
}

/// Disconnect from device
#[tauri::command]
pub async fn android_disconnect(
    state: State<'_, AndroidBridge>,
    device_id: String,
) -> Result<(), String> {
    state
        .disconnect(&device_id)
        .await
        .map_err(|e| e.to_string())
}

/// Inject touch event
#[tauri::command]
pub async fn android_inject_touch(
    state: State<'_, AndroidBridge>,
    device_id: String,
    x: f32,
    y: f32,
) -> Result<(), String> {
    if !state.check_capability(&device_id, "system", &CapabilityType::InputInjection) {
        return Err("No input injection capability".to_string());
    }

    if let Some(device) = state.get_device_pool().get_device(&device_id) {
        // Create and inject event
        let _event = InputEvent::touch(x, y, bonsai_android_bridge::input::TouchAction::Down, 0);
        // TODO: Send event to device via connection
        Ok(())
    } else {
        Err("Device not connected".to_string())
    }
}

/// Inject keyboard event
#[tauri::command]
pub async fn android_inject_key(
    state: State<'_, AndroidBridge>,
    device_id: String,
    key_code: u32,
    pressed: bool,
) -> Result<(), String> {
    if !state.check_capability(&device_id, "system", &CapabilityType::InputInjection) {
        return Err("No input injection capability".to_string());
    }

    if let Some(_device) = state.get_device_pool().get_device(&device_id) {
        let action = if pressed {
            bonsai_android_bridge::input::KeyAction::Press
        } else {
            bonsai_android_bridge::input::KeyAction::Release
        };
        let _event = InputEvent::keyboard(key_code, action);
        // TODO: Send event to device via connection
        Ok(())
    } else {
        Err("Device not connected".to_string())
    }
}

/// Inject text (multiple keystrokes)
#[tauri::command]
pub async fn android_inject_text(
    state: State<'_, AndroidBridge>,
    device_id: String,
    text: String,
) -> Result<(), String> {
    if !state.check_capability(&device_id, "system", &CapabilityType::InputInjection) {
        return Err("No input injection capability".to_string());
    }

    if let Some(_device) = state.get_device_pool().get_device(&device_id) {
        // TODO: Create keyboard events for each character
        // and send to device
        Ok(())
    } else {
        Err("Device not connected".to_string())
    }
}

/// Get device screen (as base64-encoded frame)
#[tauri::command]
pub async fn android_get_screen(
    state: State<'_, AndroidBridge>,
    device_id: String,
) -> Result<String, String> {
    if !state.check_capability(&device_id, "system", &CapabilityType::ScreenStream) {
        return Err("No screen streaming capability".to_string());
    }

    if let Some(_device) = state.get_device_pool().get_device(&device_id) {
        // TODO: Get latest frame from streaming queue
        // Encode as base64 PNG or JPEG
        Ok("data:image/png;base64,...".to_string())
    } else {
        Err("Device not connected".to_string())
    }
}

/// Issue a capability token
#[tauri::command]
pub async fn android_issue_capability(
    state: State<'_, AndroidBridge>,
    device_id: String,
    subject: String,
    capability: String,
    duration_hours: i64,
) -> Result<String, String> {
    let cap_type = match capability.as_str() {
        "ScreenStream" => CapabilityType::ScreenStream,
        "InputInjection" => CapabilityType::InputInjection,
        "FileRead" => CapabilityType::FileRead,
        "FileWrite" => CapabilityType::FileWrite,
        "AppDeploy" => CapabilityType::AppDeploy,
        _ => return Err("Unknown capability".to_string()),
    };

    state
        .issue_capability(&device_id, &subject, cap_type, duration_hours)
        .await
        .map_err(|e| e.to_string())
}

/// Revoke a capability token
#[tauri::command]
pub async fn android_revoke_capability(
    state: State<'_, AndroidBridge>,
    token_id: String,
) -> Result<(), String> {
    state
        .revoke_capability(&token_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get device metrics
#[tauri::command]
pub async fn android_get_metrics(
    state: State<'_, AndroidBridge>,
    device_id: String,
) -> Result<DeviceMetrics, String> {
    let device = state
        .get_device_pool()
        .get_device(&device_id)
        .ok_or_else(|| "Device not found".to_string())?;

    Ok(DeviceMetrics {
        device_id,
        screen_frames_sent: device.metrics.screen_frames_sent,
        input_events_processed: device.metrics.input_events_processed,
        files_synced: device.metrics.files_synced,
        avg_screen_latency: device.metrics.avg_screen_latency,
        total_data_transferred: device.metrics.total_data_transferred,
        connection_uptime: device.metrics.connection_uptime,
        battery_level: device.metrics.battery_level,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceMetrics {
    pub device_id: String,
    pub screen_frames_sent: u64,
    pub input_events_processed: u64,
    pub files_synced: u64,
    pub avg_screen_latency: f64,
    pub total_data_transferred: u64,
    pub connection_uptime: u64,
    pub battery_level: Option<u8>,
}
```

### 1.2 Register Commands in Main

File: `bonsai-workspace/src-tauri/src/lib.rs`

```rust
mod android_commands;

pub fn setup_tauri_commands(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // ... existing setup ...

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // ... existing commands ...
            android_commands::android_list_devices,
            android_commands::android_register_device,
            android_commands::android_connect,
            android_commands::android_disconnect,
            android_commands::android_inject_touch,
            android_commands::android_inject_key,
            android_commands::android_inject_text,
            android_commands::android_get_screen,
            android_commands::android_issue_capability,
            android_commands::android_revoke_capability,
            android_commands::android_get_metrics,
        ])
        .build(app)?;

    Ok(())
}
```

### 1.3 Android Panel Svelte Component

File: `bonsai-workspace/src/lib/components/AndroidPanel.svelte`

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  interface Device {
    id: string;
    name: string;
    model: string;
    status: string;
    ip: string;
    battery_level: number | null;
  }

  let devices: Device[] = [];
  let selectedDevice: Device | null = null;
  let screenImage: string = '';
  let loading = false;

  onMount(async () => {
    await refreshDevices();
  });

  async function refreshDevices() {
    try {
      devices = await invoke('android_list_devices');
    } catch (err) {
      console.error('Failed to list devices:', err);
    }
  }

  async function connectDevice(device: Device) {
    loading = true;
    try {
      await invoke('android_connect', { deviceId: device.id });
      selectedDevice = device;
      await getScreen();
    } catch (err) {
      console.error('Connection failed:', err);
    } finally {
      loading = false;
    }
  }

  async function getScreen() {
    if (!selectedDevice) return;
    try {
      screenImage = await invoke('android_get_screen', {
        deviceId: selectedDevice.id,
      });
    } catch (err) {
      console.error('Failed to get screen:', err);
    }
  }

  async function injectTouch(e: MouseEvent) {
    if (!selectedDevice) return;
    const x = e.offsetX;
    const y = e.offsetY;
    try {
      await invoke('android_inject_touch', {
        deviceId: selectedDevice.id,
        x,
        y,
      });
    } catch (err) {
      console.error('Touch injection failed:', err);
    }
  }

  async function injectText(text: string) {
    if (!selectedDevice) return;
    try {
      await invoke('android_inject_text', {
        deviceId: selectedDevice.id,
        text,
      });
    } catch (err) {
      console.error('Text injection failed:', err);
    }
  }
</script>

<div class="android-panel">
  <div class="device-list">
    <h2>Android Devices</h2>
    <button on:click={refreshDevices} disabled={loading}>Refresh</button>

    {#each devices as device (device.id)}
      <div
        class="device-item"
        class:selected={selectedDevice?.id === device.id}
        on:click={() => connectDevice(device)}
      >
        <div class="device-name">{device.name}</div>
        <div class="device-info">
          {device.model} • API {device.api_level}
          {#if device.battery_level}
            • 🔋 {device.battery_level}%
          {/if}
        </div>
        <div class="device-status" class:connected={device.status === 'Connected'}>
          {device.status}
        </div>
      </div>
    {/each}
  </div>

  {#if selectedDevice}
    <div class="screen-view">
      <h2>{selectedDevice.name} Screen</h2>
      {#if screenImage}
        <img
          src={screenImage}
          alt="Device Screen"
          on:click={injectTouch}
          class="screen-image"
        />
      {:else}
        <div class="screen-placeholder">Loading screen...</div>
      {/if}

      <div class="controls">
        <input
          type="text"
          placeholder="Type text..."
          on:change={(e) => injectText(e.target.value)}
        />
        <button on:click={() => getScreen()}>Refresh Screen</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .android-panel {
    display: flex;
    gap: 20px;
    padding: 20px;
    height: 100%;
  }

  .device-list {
    flex: 0 0 250px;
    border-right: 1px solid var(--border-color);
    padding-right: 20px;
  }

  .device-item {
    padding: 10px;
    margin: 5px 0;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .device-item:hover {
    background-color: var(--hover-bg);
  }

  .device-item.selected {
    background-color: var(--accent-bg);
    border-color: var(--accent-color);
  }

  .screen-view {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  .screen-image {
    max-width: 100%;
    max-height: 600px;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    cursor: crosshair;
  }

  .device-status.connected {
    color: var(--success-color);
  }

  .controls {
    margin-top: 10px;
    display: flex;
    gap: 10px;
  }
</style>
```

## Part 2: MCP Tool Integration

### 2.1 Define MCP Tools

File: `crates/mcp-server/src/android_tools.rs`

```rust
use serde_json::{json, Value};
use crate::McpTool;

pub fn list_android_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "list_android_devices".into(),
            description: "List all discovered Android devices and their status".into(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        McpTool {
            name: "connect_android".into(),
            description: "Connect to an Android device".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "device_id": {"type": "string"}
                },
                "required": ["device_id"]
            }),
        },
        McpTool {
            name: "android_inject_input".into(),
            description: "Inject input (touch, keyboard, etc) to an Android device".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "device_id": {"type": "string"},
                    "input_type": {
                        "type": "string",
                        "enum": ["touch", "keyboard", "text", "swipe"]
                    },
                    "data": {"type": "object"}
                },
                "required": ["device_id", "input_type", "data"]
            }),
        },
        McpTool {
            name: "android_sync_files".into(),
            description: "Synchronize files between desktop and Android device".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "device_id": {"type": "string"},
                    "direction": {
                        "type": "string",
                        "enum": ["push", "pull", "bidirectional"]
                    },
                    "path": {"type": "string"}
                },
                "required": ["device_id", "direction", "path"]
            }),
        },
        McpTool {
            name: "android_install_app".into(),
            description: "Install an APK on an Android device".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "device_id": {"type": "string"},
                    "apk_path": {"type": "string"}
                },
                "required": ["device_id", "apk_path"]
            }),
        },
        McpTool {
            name: "android_grant_capability".into(),
            description: "Issue a capability token for an agent to access a device".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "device_id": {"type": "string"},
                    "agent_id": {"type": "string"},
                    "capability": {"type": "string"},
                    "duration_hours": {"type": "integer"}
                },
                "required": ["device_id", "agent_id", "capability"]
            }),
        },
    ]
}

pub async fn handle_android_tool(
    name: &str,
    params: &Value,
    bridge: &bonsai_android_bridge::AndroidBridge,
) -> Result<Value, String> {
    match name {
        "list_android_devices" => {
            let devices = bridge.get_device_pool().get_all_devices();
            Ok(json!({
                "devices": devices.iter().map(|d| json!({
                    "id": d.id,
                    "name": d.name,
                    "model": d.model,
                    "status": format!("{:?}", d.status),
                    "ip": d.ip,
                })).collect::<Vec<_>>()
            }))
        }

        "connect_android" => {
            let device_id = params
                .get("device_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing device_id")?;

            bridge
                .connect(device_id)
                .await
                .map_err(|e| e.to_string())?;

            Ok(json!({"status": "connected"}))
        }

        "android_inject_input" => {
            let device_id = params
                .get("device_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing device_id")?;

            let input_type = params
                .get("input_type")
                .and_then(|v| v.as_str())
                .ok_or("Missing input_type")?;

            let data = params.get("data").ok_or("Missing data")?;

            // TODO: Implement input injection based on type
            Ok(json!({"status": "injected"}))
        }

        "android_sync_files" => {
            let device_id = params
                .get("device_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing device_id")?;

            // TODO: Implement file sync
            Ok(json!({"status": "synced"}))
        }

        "android_install_app" => {
            let device_id = params
                .get("device_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing device_id")?;

            let apk_path = params
                .get("apk_path")
                .and_then(|v| v.as_str())
                .ok_or("Missing apk_path")?;

            // TODO: Implement app installation
            Ok(json!({"status": "installed"}))
        }

        "android_grant_capability" => {
            let device_id = params
                .get("device_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing device_id")?;

            let agent_id = params
                .get("agent_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing agent_id")?;

            let capability = params
                .get("capability")
                .and_then(|v| v.as_str())
                .ok_or("Missing capability")?;

            let duration_hours = params
                .get("duration_hours")
                .and_then(|v| v.as_i64())
                .unwrap_or(24);

            let cap_type = match capability {
                "ScreenStream" => bonsai_android_bridge::capability::CapabilityType::ScreenStream,
                "InputInjection" => bonsai_android_bridge::capability::CapabilityType::InputInjection,
                "FileRead" => bonsai_android_bridge::capability::CapabilityType::FileRead,
                "FileWrite" => bonsai_android_bridge::capability::CapabilityType::FileWrite,
                _ => return Err("Unknown capability".to_string()),
            };

            let token_id = bridge
                .issue_capability(device_id, agent_id, cap_type, duration_hours)
                .await
                .map_err(|e| e.to_string())?;

            Ok(json!({"token_id": token_id}))
        }

        _ => Err(format!("Unknown tool: {}", name)),
    }
}
```

### 2.2 Register in MCP Server

Update `crates/mcp-server/src/tools.rs`:

```rust
pub fn list_tools() -> Vec<McpTool> {
    let mut tools = vec![
        // ... existing tools ...
    ];

    // Add Android Bridge tools
    tools.extend(crate::android_tools::list_android_tools());

    tools
}
```

## Part 3: BTI Command Interface

### 3.1 Create BTI Commands

File: `scripts/bti-android-commands.sh`

```bash
#!/bin/bash
# Bonsai Terminal Interface commands for Android Bridge

set -e

BRIDGE_HOST="${BONSAI_BRIDGE_HOST:-localhost}"
BRIDGE_PORT="${BONSAI_BRIDGE_PORT:-8080}"

function android_list() {
    curl -s "http://${BRIDGE_HOST}:${BRIDGE_PORT}/api/android/devices" | jq
}

function android_connect() {
    local device_id=$1
    [ -z "$device_id" ] && { echo "Usage: android_connect <device_id>"; exit 1; }
    curl -s -X POST "http://${BRIDGE_HOST}:${BRIDGE_PORT}/api/android/devices/$device_id/connect" | jq
}

function android_tap() {
    local device_id=$1
    local x=$2
    local y=$3
    [ -z "$y" ] && { echo "Usage: android_tap <device_id> <x> <y>"; exit 1; }
    curl -s -X POST "http://${BRIDGE_HOST}:${BRIDGE_PORT}/api/android/devices/$device_id/input" \
        -H "Content-Type: application/json" \
        -d "{\"type\": \"touch\", \"x\": $x, \"y\": $y}" | jq
}

function android_type() {
    local device_id=$1
    local text=$2
    [ -z "$text" ] && { echo "Usage: android_type <device_id> <text>"; exit 1; }
    curl -s -X POST "http://${BRIDGE_HOST}:${BRIDGE_PORT}/api/android/devices/$device_id/input" \
        -H "Content-Type: application/json" \
        -d "{\"type\": \"text\", \"data\": \"$text\"}" | jq
}

function android_sync() {
    local device_id=$1
    local direction=${2:-bidirectional}
    [ -z "$device_id" ] && { echo "Usage: android_sync <device_id> [push|pull|bidirectional]"; exit 1; }
    curl -s -X POST "http://${BRIDGE_HOST}:${BRIDGE_PORT}/api/android/devices/$device_id/sync" \
        -H "Content-Type: application/json" \
        -d "{\"direction\": \"$direction\"}" | jq
}

function android_metrics() {
    local device_id=$1
    [ -z "$device_id" ] && { echo "Usage: android_metrics <device_id>"; exit 1; }
    curl -s "http://${BRIDGE_HOST}:${BRIDGE_PORT}/api/android/devices/$device_id/metrics" | jq
}

# Main dispatcher
case "${1:-help}" in
    list) android_list ;;
    connect) android_connect "$2" ;;
    tap) android_tap "$2" "$3" "$4" ;;
    type) android_type "$2" "$3" ;;
    sync) android_sync "$2" "$3" ;;
    metrics) android_metrics "$2" ;;
    *)
        echo "Bonsai Android Bridge Commands"
        echo ""
        echo "Usage: bti android <command> [args]"
        echo ""
        echo "Commands:"
        echo "  list                      List all connected devices"
        echo "  connect <device_id>       Connect to a device"
        echo "  tap <device_id> <x> <y>  Tap at coordinates"
        echo "  type <device_id> <text>  Type text"
        echo "  sync <device_id> [dir]    Sync files (push/pull/bidirectional)"
        echo "  metrics <device_id>       Get device metrics"
        ;;
esac
```

## Part 4: State Management

### 4.1 App State Integration

Update `bonsai-workspace/src-tauri/src/lib.rs`:

```rust
use bonsai_android_bridge::{AndroidBridge, telemetry::TelemetryCollector};
use std::sync::Arc;

pub struct AppState {
    pub android_bridge: AndroidBridge,
    // ... other state ...
}

#[tauri::command]
pub async fn initialize_app(app_handle: AppHandle) -> Result<(), String> {
    // Create telemetry
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let telemetry = TelemetryCollector::new(tx, 1000);

    // Create bridge
    let bridge = AndroidBridge::new(
        telemetry,
        std::time::Duration::from_secs(5),
    );

    // Initialize
    bridge.initialize().await.map_err(|e| e.to_string())?;

    // Store in app state
    app_handle.manage(bridge);

    // Handle telemetry events in background
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            // Forward to W&B, Universe, etc.
            tracing::info!("Telemetry: {:?}", event);
        }
    });

    Ok(())
}
```

## Part 5: Example Usage

### Tauri Frontend
```typescript
import { invoke } from '@tauri-apps/api/core';

// List devices
const devices = await invoke('android_list_devices');

// Connect
await invoke('android_connect', { deviceId: 'device1' });

// Inject tap
await invoke('android_inject_touch', {
  deviceId: 'device1',
  x: 500,
  y: 1000
});
```

### MCP Agent
```python
tools = client.tools()

# List devices
devices = await client.use_mcp_tool('list_android_devices', {})

# Issue capability
token = await client.use_mcp_tool('android_grant_capability', {
    'device_id': 'device1',
    'agent_id': 'my_agent',
    'capability': 'ScreenStream',
    'duration_hours': 24
})
```

### BTI CLI
```bash
# List devices
bti android list

# Connect to device
bti android connect device1

# Tap at coordinates
bti android tap device1 500 1000

# Type text
bti android type device1 "Hello Android"

# Get metrics
bti android metrics device1
```
