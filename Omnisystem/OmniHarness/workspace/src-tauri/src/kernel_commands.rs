//! Tauri commands exposing the OmniHarness kernel bridge (`kernel_bridge.rs`)
//! to the frontend — a live connectivity indicator plus the kernel's
//! cross-language model registry, so the desktop app can show whether the
//! separate OmniHarness Rust kernel process is reachable.

use serde::Serialize;

#[derive(Serialize)]
pub struct KernelStatusPayload {
    pub connected: bool,
    pub version: Option<String>,
    pub uptime_secs: Option<i64>,
    pub events_stored: Option<u64>,
    pub tip_hash: Option<String>,
}

#[tauri::command]
pub async fn kernel_status(
    state: tauri::State<'_, crate::AppState>,
) -> Result<KernelStatusPayload, String> {
    let status = state.kernel_bridge.status().await;
    Ok(match status {
        Some(s) => KernelStatusPayload {
            connected: true,
            version: Some(s.version),
            uptime_secs: Some(s.uptime_secs),
            events_stored: Some(s.events_stored),
            tip_hash: Some(s.tip_hash),
        },
        None => KernelStatusPayload {
            connected: false,
            version: None,
            uptime_secs: None,
            events_stored: None,
            tip_hash: None,
        },
    })
}

#[derive(Serialize)]
pub struct KernelModelInfo {
    pub id: String,
    pub provider: String,
    pub display_name: String,
    pub context_window: i32,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub available: bool,
}

#[tauri::command]
pub async fn kernel_list_models(
    state: tauri::State<'_, crate::AppState>,
    provider: Option<String>,
) -> Result<Vec<KernelModelInfo>, String> {
    let models = state
        .kernel_bridge
        .list_models(&provider.unwrap_or_default())
        .await;
    Ok(models
        .into_iter()
        .map(|m| KernelModelInfo {
            id: m.id,
            provider: m.provider,
            display_name: m.display_name,
            context_window: m.context_window,
            supports_tools: m.supports_tools,
            supports_vision: m.supports_vision,
            available: m.available,
        })
        .collect())
}
