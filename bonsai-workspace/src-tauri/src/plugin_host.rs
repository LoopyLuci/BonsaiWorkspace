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

// ── Skill WASM execution (compiled skills from bonsai-skill-compiler) ─────────

/// Host state for skill WASM execution — includes log buffer and tool results.
struct SkillHostState {
    log_buf: Vec<String>,
    /// Last tool-call result written into the 0x8000 scratch region.
    last_tool_result: Vec<u8>,
}

/// Execute a compiled skill WASM module against `args_json`.
///
/// Host functions provided (namespace `"env"`):
///   - `bonsai_log(ptr, len)`                               — emit log line
///   - `bonsai_call_tool(name_ptr, name_len, args_ptr, args_len) -> i32`
///   - `bonsai_read_file(path_ptr, path_len, out_ptr, out_len) -> i32`
///
/// The `invoke(ptr: i32, len: i32) -> i32` export is called with the
/// serialised args at offset 0x1000; the i32 return is treated as a result
/// length at offset 0x6000 (or 0 = empty).
pub async fn execute_wasm_skill(
    skill_name: &str,
    wasm_bytes: &[u8],
    args: &serde_json::Value,
    tool_registry: Arc<crate::tool_registry::ToolRegistry>,
) -> Result<String, String> {
    let args_json = serde_json::to_string(args).unwrap_or_default();
    let wasm_bytes = wasm_bytes.to_vec();
    let skill_name = skill_name.to_string();
    let registry = tool_registry.clone();

    tokio::task::spawn_blocking(move || {
        run_skill_wasm_sync(&skill_name, &wasm_bytes, &args_json, registry)
    })
    .await
    .map_err(|e| format!("Skill WASM task join error: {e}"))?
}

fn run_skill_wasm_sync(
    skill_name: &str,
    wasm_bytes: &[u8],
    args_json: &str,
    tool_registry: Arc<crate::tool_registry::ToolRegistry>,
) -> Result<String, String> {
    let engine = Engine::default();
    let module = Module::new(&engine, wasm_bytes)
        .map_err(|e| format!("[skill:{skill_name}] WASM compile error: {e}"))?;

    let mut linker: Linker<SkillHostState> = Linker::new(&engine);

    // ── env::bonsai_log(ptr, len) ──────────────────────────────────────────────
    linker
        .func_wrap("env", "bonsai_log", |mut caller: wasmtime::Caller<'_, SkillHostState>, ptr: i32, len: i32| {
            let mem = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(m)) => m,
                _ => return,
            };
            let start = ptr as usize;
            let end = start.saturating_add(len as usize);
            let msg = {
                let data = mem.data(&caller);
                if end <= data.len() {
                    std::str::from_utf8(&data[start..end]).unwrap_or("").to_string()
                } else { String::new() }
            };
            if !msg.is_empty() {
                tracing::info!(skill = "wasm_skill", "[skill] {}", msg);
                caller.data_mut().log_buf.push(msg);
            }
        })
        .map_err(|e| format!("Linker bonsai_log: {e}"))?;

    // ── env::bonsai_call_tool(name_ptr, name_len, args_ptr, args_len) -> i32 ──
    // Writes result JSON into guest memory at offset 0x8000; returns byte count.
    let registry_for_tool = tool_registry.clone();
    linker
        .func_wrap("env", "bonsai_call_tool",
            move |mut caller: wasmtime::Caller<'_, SkillHostState>,
                  name_ptr: i32, name_len: i32,
                  args_ptr: i32, args_len: i32| -> i32 {
                let mem = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(m)) => m,
                    _ => return -1,
                };
                let (tool_name, args_str) = {
                    let data = mem.data(&caller);
                    let name = read_str(data, name_ptr, name_len);
                    let args = read_str(data, args_ptr, args_len);
                    (name, args)
                };
                let args_val: serde_json::Value =
                    serde_json::from_str(&args_str).unwrap_or(serde_json::Value::Null);

                // Block on the async tool call using the current Tokio runtime.
                let result_str = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        match registry_for_tool.execute(&tool_name, &args_val).await {
                            Some(r) => r.as_text().unwrap_or("").to_string(),
                            None => format!("Error: tool '{}' not found", tool_name),
                        }
                    })
                });

                let result_bytes = result_str.as_bytes();
                const OUT_OFFSET: usize = 0x8000;
                if let Ok(()) = mem.write(&mut caller, OUT_OFFSET, result_bytes) {
                    caller.data_mut().last_tool_result = result_bytes.to_vec();
                    result_bytes.len() as i32
                } else {
                    -1
                }
            },
        )
        .map_err(|e| format!("Linker bonsai_call_tool: {e}"))?;

    // ── env::bonsai_read_file(path_ptr, path_len, out_ptr, out_len) -> i32 ────
    linker
        .func_wrap("env", "bonsai_read_file",
            |mut caller: wasmtime::Caller<'_, SkillHostState>,
             path_ptr: i32, path_len: i32,
             out_ptr: i32, out_len: i32| -> i32 {
                let mem = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(m)) => m,
                    _ => return -1,
                };
                let path_str = {
                    let data = mem.data(&caller);
                    read_str(data, path_ptr, path_len)
                };
                let content = match std::fs::read_to_string(&path_str) {
                    Ok(c) => c,
                    Err(_) => return -1,
                };
                let bytes = content.as_bytes();
                let copy_len = bytes.len().min(out_len as usize);
                if mem.write(&mut caller, out_ptr as usize, &bytes[..copy_len]).is_ok() {
                    copy_len as i32
                } else {
                    -1
                }
            },
        )
        .map_err(|e| format!("Linker bonsai_read_file: {e}"))?;

    // Also wire the legacy "bonsai::log" namespace so old plugins still work.
    linker
        .func_wrap("bonsai", "log", |mut caller: wasmtime::Caller<'_, SkillHostState>, ptr: i32, len: i32| {
            let mem = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(m)) => m,
                _ => return,
            };
            let msg = {
                let data = mem.data(&caller);
                read_str(data, ptr, len)
            };
            caller.data_mut().log_buf.push(msg);
        })
        .map_err(|e| format!("Linker bonsai::log: {e}"))?;

    let mut store = Store::new(&engine, SkillHostState {
        log_buf: Vec::new(),
        last_tool_result: Vec::new(),
    });

    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| format!("[skill:{skill_name}] instantiate error: {e}"))?;

    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or_else(|| format!("[skill:{skill_name}] no exported 'memory'"))?;

    // Write args into memory at 0x1000
    let args_bytes = args_json.as_bytes();
    const ARGS_OFFSET: usize = 0x1000;
    memory
        .write(&mut store, ARGS_OFFSET, args_bytes)
        .map_err(|e| format!("[skill:{skill_name}] memory write: {e}"))?;

    // Try `invoke` first (skills compiled by bonsai-skill-compiler), then fall
    // back to `handle_message` (legacy plugin format).
    let result_str = if let Ok(invoke) = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "invoke")
    {
        let result_len = invoke
            .call(&mut store, (ARGS_OFFSET as i32, args_bytes.len() as i32))
            .map_err(|e| format!("[skill:{skill_name}] invoke error: {e}"))?;
        if result_len > 0 {
            const RES_OFFSET: usize = 0x6000;
            let data = memory.data(&store);
            let end = (RES_OFFSET + result_len as usize).min(data.len());
            std::str::from_utf8(&data[RES_OFFSET..end])
                .unwrap_or("")
                .to_string()
        } else {
            String::new()
        }
    } else if let Ok(handle) = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "handle_message")
    {
        let result_ptr = handle
            .call(&mut store, (ARGS_OFFSET as i32, args_bytes.len() as i32))
            .map_err(|e| format!("[skill:{skill_name}] handle_message error: {e}"))?;
        let data = memory.data(&store);
        let start = result_ptr as usize;
        let end = data[start..].iter().position(|&b| b == 0).map(|p| start + p).unwrap_or(start);
        std::str::from_utf8(&data[start..end]).unwrap_or("").to_string()
    } else {
        return Err(format!("[skill:{skill_name}] no callable export (invoke or handle_message)"));
    };

    let logs = store.data().log_buf.join("\n");
    if logs.is_empty() { Ok(result_str) } else { Ok(format!("{logs}\n{result_str}")) }
}

/// Helper: read a UTF-8 string from WASM linear memory.
#[inline]
fn read_str(data: &[u8], ptr: i32, len: i32) -> String {
    let start = ptr as usize;
    let end = start.saturating_add(len as usize).min(data.len());
    std::str::from_utf8(&data[start..end]).unwrap_or("").to_string()
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
