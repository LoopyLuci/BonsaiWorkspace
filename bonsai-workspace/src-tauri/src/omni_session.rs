//! Workstream G — OmniSession: Unified user environment
//!
//! Ties OmniDesktop, OmniShell, ProcessManager, OmnipresentCapture, and the
//! PredictiveEngine into a single login/logout lifecycle.  Session snapshots
//! are serialised to CAS so the user's open apps and windows survive reboots.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use bonsai_cas::CasStore;
use crate::auth_commands::{AuthState, UserProfile};
use crate::omni_desktop::OmniDesktop;
use crate::omni_shell::OmniShellState;
use crate::omnipresent_capture::OmnipresentCapture;
use crate::predictive_engine::PredictiveEngine;
use crate::process_manager::ProcessManager;

// ─────────────────────────────────────────────────────────────────────────────
// § 1 — Session state types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSession {
    pub app_id: String,
    pub app_name: String,
    pub window_ids: Vec<String>,
    pub last_focused: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellSession {
    pub session_id: String,
    pub working_dir: String,
    pub history_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub active_session_id: Uuid,
    pub user_id: String,
    pub display_name: String,
    pub login_time: i64,
    pub last_activity: i64,
    pub open_apps: Vec<AppSession>,
    pub shell_sessions: Vec<ShellSession>,
    pub workspace_path: String,
    pub is_locked: bool,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            active_session_id: Uuid::new_v4(),
            user_id: String::new(),
            display_name: String::new(),
            login_time: 0,
            last_activity: 0,
            open_apps: vec![],
            shell_sessions: vec![],
            workspace_path: String::from("."),
            is_locked: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub session_id: Uuid,
    pub user_id: String,
    pub open_apps: Vec<AppSession>,
    pub shell_sessions: Vec<ShellSession>,
    pub active_workspace: u32,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummaryReport {
    pub user_id: String,
    pub session_id: String,
    pub duration_mins: i64,
    pub total_events: u64,
    pub apps_used: Vec<String>,
    pub commands_run: u64,
    pub ai_inferences: u64,
    pub files_touched: u64,
    pub highlights: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// § 2 — OmniSession
// ─────────────────────────────────────────────────────────────────────────────

pub struct OmniSession {
    pub desktop: Arc<OmniDesktop>,
    pub shell: Arc<OmniShellState>,
    pub process_manager: Arc<ProcessManager>,
    pub capture: Arc<OmnipresentCapture>,
    pub predictive: Arc<PredictiveEngine>,
    pub cas: Arc<CasStore>,
    pub auth: Arc<AuthState>,
    pub state: RwLock<SessionState>,
    /// CAS key of the most recent saved snapshot (hex), if any.
    pub last_snapshot_key: RwLock<Option<String>>,
}

impl OmniSession {
    pub fn new(
        desktop: Arc<OmniDesktop>,
        shell: Arc<OmniShellState>,
        process_manager: Arc<ProcessManager>,
        capture: Arc<OmnipresentCapture>,
        predictive: Arc<PredictiveEngine>,
        cas: Arc<CasStore>,
        auth: Arc<AuthState>,
    ) -> Arc<Self> {
        Arc::new(Self {
            desktop,
            shell,
            process_manager,
            capture,
            predictive,
            cas,
            auth,
            state: RwLock::new(SessionState::default()),
            last_snapshot_key: RwLock::new(None),
        })
    }

    // ── Login ─────────────────────────────────────────────────────────────────

    pub async fn login(&self, user_id: &str, _passphrase: &str) -> Result<SessionState, String> {
        // Resolve display name from auth
        let display_name = {
            let profiles = self.auth.profiles.read().await;
            profiles
                .iter()
                .find(|p| p.id == user_id)
                .map(|p| p.display_name.clone())
                .unwrap_or_else(|| user_id.to_string())
        };

        let mut state = self.state.write().await;
        state.active_session_id = Uuid::new_v4();
        state.user_id = user_id.to_string();
        state.display_name = display_name.clone();
        state.login_time = Utc::now().timestamp_millis();
        state.last_activity = state.login_time;
        state.is_locked = false;

        info!("[omni-session] login: user={} session={}", user_id, state.active_session_id);

        // Restore previous snapshot if available
        let snap_key = self.last_snapshot_key.read().await.clone();
        if let Some(key_hex) = snap_key {
            if let Ok(cas_key) = bonsai_cas::CasKey::from_hex(&key_hex) {
                if let Ok(Some(bytes)) = self.cas.get(&cas_key).await {
                    if let Ok(snap) = serde_json::from_slice::<SessionSnapshot>(&bytes) {
                        state.open_apps = snap.open_apps;
                        state.shell_sessions = snap.shell_sessions;
                        info!("[omni-session] restored snapshot from {}", key_hex);
                    }
                }
            }
        }

        Ok(state.clone())
    }

    // ── Logout ────────────────────────────────────────────────────────────────

    pub async fn logout(&self) -> Result<String, String> {
        // Save snapshot before logging out
        let snap_key = self.save_snapshot().await?;
        let mut state = self.state.write().await;
        state.is_locked = true;
        info!("[omni-session] logout: user={}", state.user_id);
        Ok(snap_key)
    }

    // ── Snapshot ──────────────────────────────────────────────────────────────

    pub async fn save_snapshot(&self) -> Result<String, String> {
        let state = self.state.read().await;
        let active_workspace = self.desktop.active_workspace_id();
        let snap = SessionSnapshot {
            session_id: state.active_session_id,
            user_id: state.user_id.clone(),
            open_apps: state.open_apps.clone(),
            shell_sessions: state.shell_sessions.clone(),
            active_workspace,
            timestamp: Utc::now().timestamp_millis(),
        };
        let bytes = serde_json::to_vec(&snap).map_err(|e| e.to_string())?;
        let key = self.cas.put(&bytes, "application/x-omni-session").await.map_err(|e| e.to_string())?;
        let hex = key.hex();
        *self.last_snapshot_key.write().await = Some(hex.clone());
        info!("[omni-session] snapshot saved: {}", hex);
        Ok(hex)
    }

    // ── AI Session Summary ────────────────────────────────────────────────────

    pub async fn session_summary(&self, hours_ago: u32) -> SessionSummaryReport {
        let since_ms = Utc::now().timestamp_millis() - (hours_ago as i64 * 3_600_000);
        let events = self.capture.get_events_since(since_ms).await;
        let state = self.state.read().await;

        let total = events.len() as u64;
        let mut apps_set = std::collections::HashSet::new();
        let mut commands_run: u64 = 0;
        let mut ai_inferences: u64 = 0;
        let mut files_touched: u64 = 0;

        for ev in &events {
            match &ev.event_type {
                crate::omnipresent_capture::OmnEventType::AppLaunch { app_id, .. }
                | crate::omnipresent_capture::OmnEventType::AppClose { app_id, .. }
                | crate::omnipresent_capture::OmnEventType::WindowFocus { app_id, .. } => {
                    apps_set.insert(app_id.clone());
                }
                crate::omnipresent_capture::OmnEventType::CommandCompleted { .. } => commands_run += 1,
                crate::omnipresent_capture::OmnEventType::ModelInference { .. } => ai_inferences += 1,
                crate::omnipresent_capture::OmnEventType::FileSave { .. }
                | crate::omnipresent_capture::OmnEventType::FileOpen { .. } => files_touched += 1,
                _ => {}
            }
        }

        let login_ms = state.login_time;
        let now_ms = Utc::now().timestamp_millis();
        let duration_mins = (now_ms - login_ms) / 60_000;

        let mut highlights = vec![];
        if commands_run > 0 { highlights.push(format!("Ran {} shell commands", commands_run)); }
        if ai_inferences > 0 { highlights.push(format!("Made {} AI inferences", ai_inferences)); }
        if files_touched > 0 { highlights.push(format!("Touched {} files", files_touched)); }

        SessionSummaryReport {
            user_id: state.user_id.clone(),
            session_id: state.active_session_id.to_string(),
            duration_mins,
            total_events: total,
            apps_used: apps_set.into_iter().collect(),
            commands_run,
            ai_inferences,
            files_touched,
            highlights,
        }
    }

    // ── Touch activity ────────────────────────────────────────────────────────

    pub async fn touch(&self) {
        self.state.write().await.last_activity = Utc::now().timestamp_millis();
    }

    pub async fn get_state(&self) -> SessionState {
        self.state.read().await.clone()
    }
}


// ─────────────────────────────────────────────────────────────────────────────
// § 5 — Tauri commands
// ─────────────────────────────────────────────────────────────────────────────

use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn omni_session_login(
    user_id: String,
    passphrase: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let sess = state.omni_session.login(&user_id, &passphrase).await?;
    Ok(serde_json::json!({
        "session_id": sess.active_session_id,
        "display_name": sess.display_name,
        "login_time": sess.login_time,
        "workspace_path": sess.workspace_path,
    }))
}

#[tauri::command]
pub async fn omni_session_logout(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let snap_key = state.omni_session.logout().await?;
    Ok(serde_json::json!({ "snapshot_key": snap_key }))
}

#[tauri::command]
pub async fn omni_session_summary(
    hours_ago: Option<u32>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let h = hours_ago.unwrap_or(24);
    let report = state.omni_session.session_summary(h).await;
    Ok(serde_json::to_value(report).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn omni_session_snapshot(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let key = state.omni_session.save_snapshot().await?;
    Ok(serde_json::json!({ "snapshot_key": key }))
}

#[tauri::command]
pub async fn omni_session_state(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let s = state.omni_session.get_state().await;
    Ok(serde_json::to_value(s).map_err(|e| e.to_string())?)
}
