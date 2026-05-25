//! Secure plugin host with capability enforcement and blake3 integrity checks.
//!
//! Plugins are directories containing:
//!   - `bonsai-plugin.toml`  — capability manifest
//!   - `plugin.wasm` (optional) — WASM entrypoint (future; stubbed here)
//!   - `plugin.py`  (optional) — Python entrypoint executed via sandbox venv
//!
//! Execution path (current):
//!   1. Load + verify manifest (toml parse + blake3 hash of entrypoint)
//!   2. Enforce capability gate before any execution
//!   3. Run Python entrypoint in sandbox venv with 30 s timeout
//!
//! WASM execution will be enabled when the `wasmtime-host` feature is added
//! to src-tauri; the capability model and manifest format are already final.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::plugin_manifest::{Capability, PluginManifest};
use crate::sandbox_executor::{SandboxRequest, SandboxResult, SandboxTier};

// ── Loaded plugin record ───────────────────────────────────────────────────────

#[derive(Clone)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    /// blake3 hex digest of the entrypoint file at load time.
    pub entrypoint_hash: String,
    /// Absolute path to the plugin directory.
    pub dir: PathBuf,
}

// ── Plugin execution result ────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct PluginOutput {
    pub plugin: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

// ── Plugin host ────────────────────────────────────────────────────────────────

pub struct PluginHost {
    plugins: RwLock<HashMap<String, LoadedPlugin>>,
    plugins_dir: PathBuf,
}

impl PluginHost {
    pub fn new() -> Self {
        let dir = dirs::data_local_dir()
            .unwrap_or_default()
            .join("com.bonsai.workspace")
            .join("plugins");
        Self {
            plugins: RwLock::new(HashMap::new()),
            plugins_dir: dir,
        }
    }

    pub fn with_dir(dir: PathBuf) -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
            plugins_dir: dir,
        }
    }

    /// Load a plugin from a directory. Verifies integrity with blake3.
    pub async fn load(&self, id: &str, plugin_dir: &Path) -> Result<(), String> {
        let manifest_path = plugin_dir.join("bonsai-plugin.toml");
        let manifest_str = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Cannot read manifest: {e}"))?;
        let manifest = PluginManifest::from_toml(&manifest_str)?;

        let entrypoint_path = plugin_dir.join(&manifest.entrypoint);
        if !entrypoint_path.exists() {
            return Err(format!("Entrypoint '{}' not found in plugin dir", manifest.entrypoint));
        }

        let entrypoint_bytes = std::fs::read(&entrypoint_path)
            .map_err(|e| format!("Cannot read entrypoint: {e}"))?;
        let hash = blake3::hash(&entrypoint_bytes).to_hex().to_string();

        info!(id, hash=%&hash[..16], caps=?manifest.capabilities, "[plugin_host] loaded plugin");

        self.plugins.write().await.insert(
            id.to_string(),
            LoadedPlugin {
                manifest,
                entrypoint_hash: hash,
                dir: plugin_dir.to_path_buf(),
            },
        );
        Ok(())
    }

    /// Auto-discover and load all plugins in the plugins directory.
    pub async fn load_all(&self) {
        let dir = self.plugins_dir.clone();
        if !dir.exists() { return; }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("bonsai-plugin.toml").exists() {
                    let id = entry.file_name().to_string_lossy().into_owned();
                    if let Err(e) = self.load(&id, &path).await {
                        warn!(id, error=%e, "[plugin_host] failed to load plugin");
                    }
                }
            }
        }
    }

    /// Execute a loaded plugin with the given payload string.
    /// Enforces capability gate before execution.
    pub async fn execute(
        &self,
        id: &str,
        payload: &str,
        required_caps: &[Capability],
    ) -> Result<PluginOutput, String> {
        let plugins = self.plugins.read().await;
        let plugin = plugins.get(id).ok_or_else(|| format!("Plugin '{id}' not loaded"))?;

        // Capability enforcement
        for cap in required_caps {
            if !plugin.manifest.capabilities.contains(cap) {
                return Err(format!(
                    "Plugin '{id}' does not have capability {:?}",
                    cap
                ));
            }
        }

        let entrypoint_path = plugin.dir.join(&plugin.manifest.entrypoint);
        let plugin_dir = plugin.dir.clone();
        drop(plugins); // release read lock before async work

        // Integrity re-check at execution time
        let current_bytes = std::fs::read(&entrypoint_path)
            .map_err(|e| format!("Cannot read entrypoint: {e}"))?;
        let current_hash = blake3::hash(&current_bytes).to_hex().to_string();
        {
            let plugins = self.plugins.read().await;
            let plugin = plugins.get(id).unwrap();
            if current_hash != plugin.entrypoint_hash {
                return Err(format!("Plugin '{id}' integrity check failed — entrypoint was modified"));
            }
        }

        // Determine execution tier from entrypoint extension
        let ext = entrypoint_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let result: SandboxResult = match ext {
            "py" => {
                let code = format!(
                    "import sys, json, os\nos.chdir(r'{}')\n{}",
                    plugin_dir.display(),
                    String::from_utf8_lossy(&current_bytes)
                );
                // Inject payload as environment variable
                let code_with_payload = format!(
                    "import os; os.environ['BONSAI_PAYLOAD'] = {}\n{code}",
                    serde_json::to_string(payload).unwrap_or_default()
                );
                crate::sandbox_executor::execute_plugin_code(&code_with_payload).await?
            }
            "wasm" => {
                // WASM execution: enable with wasmtime-host feature in Cargo.toml
                return Err("WASM plugin execution not yet enabled in this build. Add wasmtime-host feature to Cargo.toml.".into());
            }
            _ => return Err(format!("Unknown entrypoint type: .{ext}")),
        };

        Ok(PluginOutput {
            plugin: id.to_string(),
            stdout: result.stdout,
            stderr: result.stderr,
            exit_code: result.exit_code,
            timed_out: result.timed_out,
        })
    }

    /// List all loaded plugin IDs and their manifests.
    pub async fn list(&self) -> Vec<(String, PluginManifest)> {
        self.plugins
            .read()
            .await
            .iter()
            .map(|(k, v)| (k.clone(), v.manifest.clone()))
            .collect()
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_loaded_plugins(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let list = state.plugin_host.list().await;
    Ok(list
        .into_iter()
        .map(|(id, manifest)| {
            serde_json::json!({
                "id": id,
                "name": manifest.name,
                "version": manifest.version,
                "capabilities": manifest.capabilities,
            })
        })
        .collect())
}

#[tauri::command]
pub async fn load_plugin_from_dir(
    state: tauri::State<'_, crate::AppState>,
    id: String,
    path: String,
) -> Result<(), String> {
    state.plugin_host.load(&id, Path::new(&path)).await
}

#[tauri::command]
pub async fn execute_plugin(
    state: tauri::State<'_, crate::AppState>,
    id: String,
    payload: String,
) -> Result<PluginOutput, String> {
    // Default: require no special capability for basic execute
    state.plugin_host.execute(&id, &payload, &[]).await
}
