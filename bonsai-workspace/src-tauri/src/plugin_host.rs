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
use wasmtime::{Engine, Linker, Module, Store};

use crate::plugin_manifest::{Capability, PluginManifest};
use crate::sandbox_executor::SandboxResult;

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
                let wasm_bytes = current_bytes.clone();
                let payload_str = payload.to_string();
                // Run in blocking thread pool to avoid starving the async runtime
                let output = tokio::task::spawn_blocking(move || {
                    run_wasm_plugin(&wasm_bytes, &payload_str)
                })
                .await
                .map_err(|e| format!("WASM task join error: {e}"))??;
                SandboxResult {
                    stdout: output,
                    stderr: String::new(),
                    exit_code: 0,
                    timed_out: false,
                }
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

// ── WASM execution ────────────────────────────────────────────────────────────

/// Host state threaded through WASM store: captures log lines emitted by
/// the plugin via the `bonsai::log` import.
struct WasmHostState {
    log_buf: Vec<String>,
}

/// Execute a WASM plugin module. Exports expected:
///   - `memory` — linear memory
///   - `handle_message(ptr: i32, len: i32) -> i32`  — entry point; returns
///     pointer to a null-terminated JSON string in linear memory.
///
/// Imports provided by the host (namespace "bonsai"):
///   - `log(ptr: i32, len: i32)` — append UTF-8 string to host log buffer.
fn run_wasm_plugin(wasm_bytes: &[u8], payload: &str) -> Result<String, String> {
    let engine = Engine::default();
    let module = Module::new(&engine, wasm_bytes)
        .map_err(|e| format!("WASM compile error: {e}"))?;

    let mut linker: Linker<WasmHostState> = Linker::new(&engine);

    // Provide `bonsai::log(ptr, len)` — plugins call this to emit log lines.
    linker
        .func_wrap("bonsai", "log", |mut caller: wasmtime::Caller<'_, WasmHostState>, ptr: i32, len: i32| {
            let mem = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(m)) => m,
                _ => return,
            };
            let start = ptr as usize;
            let end = start.saturating_add(len as usize);
            // Copy out before mutable borrow of store data.
            let copied: Option<String> = {
                let data = mem.data(&caller);
                if end <= data.len() {
                    std::str::from_utf8(&data[start..end]).ok().map(|s| s.to_string())
                } else {
                    None
                }
            };
            if let Some(s) = copied {
                caller.data_mut().log_buf.push(s);
            }
        })
        .map_err(|e| format!("Linker error: {e}"))?;

    let mut store = Store::new(&engine, WasmHostState { log_buf: Vec::new() });
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| format!("WASM instantiate error: {e}"))?;

    // Write payload into linear memory via a stack-style allocator.
    // We use offset 4096 as a safe write region (plugins must not use it).
    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or_else(|| "WASM module has no exported 'memory'".to_string())?;

    let payload_bytes = payload.as_bytes();
    let payload_offset: i32 = 4096;
    let payload_len = payload_bytes.len() as i32;
    memory
        .write(&mut store, payload_offset as usize, payload_bytes)
        .map_err(|e| format!("Memory write error: {e}"))?;

    // Call `handle_message(ptr, len) -> i32` where the return value is a
    // pointer to a null-terminated result string in linear memory.
    let handle = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "handle_message")
        .map_err(|e| format!("Missing export 'handle_message': {e}"))?;

    let result_ptr = handle
        .call(&mut store, (payload_offset, payload_len))
        .map_err(|e| format!("WASM execution error: {e}"))?;

    // Read null-terminated result string from memory.
    let data = memory.data(&store);
    let start = result_ptr as usize;
    let end = data[start..]
        .iter()
        .position(|&b| b == 0)
        .map(|p| start + p)
        .unwrap_or(start);
    let result = std::str::from_utf8(&data[start..end])
        .map_err(|e| format!("WASM result not valid UTF-8: {e}"))?
        .to_string();

    // Prepend any host log lines.
    let logs = store.data().log_buf.join("\n");
    if logs.is_empty() {
        Ok(result)
    } else {
        Ok(format!("{logs}\n{result}"))
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
