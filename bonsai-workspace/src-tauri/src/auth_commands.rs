//! Multi-user auth — encrypted workspace state with per-profile passphrase keys.
//!
//! Self-contained: no external bonsai_auth crate required.  In production this
//! should be backed by a proper KDF (Argon2) + AES-GCM.  For now it uses a
//! deterministic placeholder so the API surface compiles and the UI works.

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::RwLock;
use uuid::Uuid;

// ── Domain types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id:           String,
    pub display_name: String,
    /// Hex-encoded public key (placeholder).
    pub pub_key_hex:  String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub token:      String,
    pub profile_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id:    String,
    pub name:  String,
    pub owner: String,
    /// Access grants: (grantee_pub_hex, permissions_bitmask)
    pub grants: Vec<(String, u8)>,
}

// ── State ─────────────────────────────────────────────────────────────────────

pub struct AuthState {
    pub active_session: RwLock<Option<Session>>,
    pub profiles:       RwLock<Vec<UserProfile>>,
    pub workspaces:     RwLock<Vec<Workspace>>,
}

impl AuthState {
    pub fn new() -> Self {
        Self {
            active_session: RwLock::new(None),
            profiles:       RwLock::new(vec![]),
            workspaces:     RwLock::new(vec![]),
        }
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn derive_pub_key(passphrase: &str) -> String {
    // Placeholder: real impl would use ed25519 derivation from Argon2 hash.
    format!("{:x}", passphrase.len() * 0x1337)
}

fn verify_passphrase(_profile: &UserProfile, _passphrase: &str) -> bool {
    // Placeholder: real impl would verify against stored KDF hash.
    true
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn create_profile(
    state: State<'_, AuthState>,
    passphrase: String,
    display_name: String,
) -> Result<UserProfile, String> {
    let profile = UserProfile {
        id:           Uuid::new_v4().to_string(),
        display_name: display_name.clone(),
        pub_key_hex:  derive_pub_key(&passphrase),
    };
    state.profiles.write().await.push(profile.clone());
    Ok(profile)
}

#[tauri::command]
pub async fn unlock_profile(
    state: State<'_, AuthState>,
    profile_id: String,
    passphrase: String,
) -> Result<String, String> {
    let profiles = state.profiles.read().await;
    let profile = profiles.iter().find(|p| p.id == profile_id)
        .ok_or("profile not found")?;
    if !verify_passphrase(profile, &passphrase) {
        return Err("wrong passphrase".into());
    }
    drop(profiles);
    let session = Session {
        token:      Uuid::new_v4().to_string(),
        profile_id: profile_id.clone(),
    };
    let pid = session.profile_id.clone();
    *state.active_session.write().await = Some(session);
    Ok(pid)
}

#[tauri::command]
pub async fn lock_profile(state: State<'_, AuthState>) -> Result<(), String> {
    *state.active_session.write().await = None;
    Ok(())
}

#[tauri::command]
pub async fn create_workspace(
    state: State<'_, AuthState>,
    name: String,
) -> Result<Workspace, String> {
    let owner = {
        let guard = state.active_session.read().await;
        guard.as_ref().ok_or("no active session")?.profile_id.clone()
    };
    let ws = Workspace {
        id:     Uuid::new_v4().to_string(),
        name:   name.clone(),
        owner,
        grants: vec![],
    };
    state.workspaces.write().await.push(ws.clone());
    Ok(ws)
}

#[tauri::command]
pub async fn share_workspace(
    state: State<'_, AuthState>,
    workspace_id: String,
    grantee_pub: String,
    permissions: u8,
) -> Result<(), String> {
    let owner = {
        let guard = state.active_session.read().await;
        guard.as_ref().ok_or("no active session")?.profile_id.clone()
    };

    let mut workspaces = state.workspaces.write().await;
    let ws = workspaces.iter_mut().find(|w| w.id == workspace_id)
        .ok_or("workspace not found")?;
    if ws.owner != owner {
        return Err("not workspace owner".into());
    }
    ws.grants.push((grantee_pub, permissions));
    Ok(())
}

#[tauri::command]
pub async fn list_workspaces(state: State<'_, AuthState>) -> Result<Vec<Workspace>, String> {
    Ok(state.workspaces.read().await.clone())
}
