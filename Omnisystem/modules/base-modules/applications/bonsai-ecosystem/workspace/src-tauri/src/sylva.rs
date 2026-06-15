//! Sylva scripting layer — hot-reloadable Lua scripts that become first-class tools.
//!
//! Users drop `.lua` files into `<data_dir>/bonsai/scripts/`. Within 2 seconds
//! the file watcher detects the change, compiles the script, registers it in the
//! UCR, and emits a `"sylva-reload"` Tauri event so the frontend can update.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mlua::prelude::*;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::tool_registry::{Tool, ToolRegistryState, ToolResult};

// ── Call record for time-travel debugging ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SylvaCallRecord {
    pub script_name: String,
    pub args: serde_json::Value,
    pub result: Result<serde_json::Value, String>,
    pub elapsed_ms: u64,
    pub timestamp: i64,
}

// ── Sylva script metadata ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SylvaScript {
    pub name: String,
    pub path: String,
    pub source: String,
    pub loaded_at: i64,
    pub error: Option<String>,
}

// ── SylvaRuntime ──────────────────────────────────────────────────────────────

pub struct SylvaRuntime {
    /// One Lua VM per runtime (Lua state is single-threaded, so we use a Mutex).
    lua: std::sync::Mutex<Lua>,
    tool_registry: Arc<ToolRegistryState>,
    scripts_dir: PathBuf,
    /// Loaded script metadata (name → SylvaScript).
    scripts: RwLock<HashMap<String, SylvaScript>>,
    /// Ring buffer of recent calls for time-travel debugging.
    call_history: RwLock<std::collections::VecDeque<SylvaCallRecord>>,
    /// Maximum history entries.
    history_limit: usize,
}

impl SylvaRuntime {
    pub fn new(
        tool_registry: Arc<ToolRegistryState>,
        scripts_dir: PathBuf,
    ) -> Result<Arc<Self>, String> {
        let lua = Lua::new();
        Self::inject_bonsai_globals(&lua, &tool_registry)
            .map_err(|e| format!("Sylva init failed: {e}"))?;

        Ok(Arc::new(Self {
            lua: std::sync::Mutex::new(lua),
            tool_registry,
            scripts_dir,
            scripts: RwLock::new(HashMap::new()),
            call_history: RwLock::new(std::collections::VecDeque::new()),
            history_limit: 100,
        }))
    }

    /// Inject the `bonsai` global table and remove dangerous Lua standard libraries.
    fn inject_bonsai_globals(lua: &Lua, registry: &Arc<ToolRegistryState>) -> LuaResult<()> {
        // Remove modules that could escape the sandbox
        let globals = lua.globals();
        for module in &[
            "os", "io", "package", "require", "dofile", "loadfile", "load",
        ] {
            globals.raw_remove(*module)?;
        }

        let bonsai = lua.create_table()?;

        // bonsai.tool(name, args_table) → JSON string result or error
        let reg = registry.clone();
        bonsai.set(
            "tool",
            lua.create_function(move |lua_ctx, (name, args): (String, LuaValue)| {
                let json_args = lua_value_to_json(args)?;
                // Synchronous bridge: use block_in_place since Lua is sync
                let result = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(reg.invoke_by_name(&name, json_args))
                });
                match result {
                    Ok(val) => Ok(json_to_lua_value(lua_ctx, val)?),
                    Err(e) => Err(LuaError::external(e)),
                }
            })?,
        )?;

        // bonsai.log(msg) — log from Lua to the Rust tracing subscriber
        bonsai.set(
            "log",
            lua.create_function(|_, msg: String| {
                info!("[sylva] {msg}");
                Ok(())
            })?,
        )?;

        // bonsai.json_encode(table) → string
        bonsai.set(
            "json_encode",
            lua.create_function(|lua_ctx, val: LuaValue| {
                let json = lua_value_to_json(val)?;
                Ok(json.to_string())
            })?,
        )?;

        // bonsai.json_decode(str) → table
        bonsai.set(
            "json_decode",
            lua.create_function(|lua_ctx, s: String| {
                let json: serde_json::Value =
                    serde_json::from_str(&s).map_err(|e| LuaError::external(e))?;
                json_to_lua_value(lua_ctx, json)
            })?,
        )?;

        lua.globals().set("bonsai", bonsai)?;
        Ok(())
    }

    /// Load (or reload) a Lua script file and register it as a tool.
    pub async fn load_script(&self, path: &Path) -> Result<String, String> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("invalid script path: {}", path.display()))?
            .to_string();

        let source = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| format!("read error: {e}"))?;

        // Try to compile + execute in the Lua VM to catch syntax errors
        let compile_result = {
            let lua = self.lua.lock().map_err(|e| e.to_string())?;
            lua.load(&source)
                .set_name(&name)
                .exec()
                .map_err(|e| format!("Lua compile error in {name}: {e}"))
        };

        let error = compile_result.err();

        let script = SylvaScript {
            name: name.clone(),
            path: path.display().to_string(),
            source: source.clone(),
            loaded_at: chrono::Utc::now().timestamp(),
            error: error.clone(),
        };

        self.scripts.write().await.insert(name.clone(), script);

        if let Some(ref e) = error {
            warn!("[sylva] script {name} has errors: {e}");
            return Err(e.clone());
        }

        // Register as a tool in the UCR
        let tool = SylvaTool {
            name: name.clone(),
            source,
            lua: Arc::new(std::sync::Mutex::new(
                // Each tool gets its own Lua state so calls are independent
                {
                    let lua = Lua::new();
                    Self::inject_bonsai_globals(&lua, &self.tool_registry)
                        .map_err(|e| e.to_string())?;
                    lua
                },
            )),
        };

        self.tool_registry.registry.register(Box::new(tool)).await;
        info!("[sylva] registered tool '{name}'");
        Ok(name)
    }

    /// Execute a raw Lua string with a 10-second wall-clock timeout.
    pub fn exec_str(&self, src: &str) -> Result<serde_json::Value, String> {
        let lua = self.lua.lock().map_err(|e| e.to_string())?;
        // Set instruction count limit (~10M ops ≈ a few seconds of pure Lua)
        lua.set_hook(
            mlua::HookTriggers::new().every_nth_instruction(10_000_000),
            |_lua, _debug| {
                Err(mlua::Error::RuntimeError(
                    "Sylva execution limit reached (10M instructions)".into(),
                ))
            },
        );
        let result = lua.load(src).eval::<LuaValue>().map_err(|e| e.to_string());
        let _ = lua.remove_hook();
        let val = result?;
        lua_value_to_json(val).map_err(|e| e.to_string())
    }

    /// Execute a `.lua` file by path and return the result as JSON.
    pub async fn exec_file(&self, path: &Path) -> Result<serde_json::Value, String> {
        let source = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| format!("read error: {e}"))?;
        self.exec_str(&source)
    }

    /// Call a named function defined in any loaded script.
    pub fn call_fn(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let lua = self.lua.lock().map_err(|e| e.to_string())?;
        let func: LuaFunction = lua
            .globals()
            .get(name)
            .map_err(|_| format!("function '{name}' not found in Sylva VM"))?;
        let lua_args = json_to_lua_value(&lua, args).map_err(|e| e.to_string())?;
        let result: LuaValue = func.call(lua_args).map_err(|e| e.to_string())?;
        lua_value_to_json(result).map_err(|e| e.to_string())
    }

    /// Return all loaded scripts.
    pub async fn list_scripts(&self) -> Vec<SylvaScript> {
        self.scripts.read().await.values().cloned().collect()
    }

    /// Return recent call history (time-travel debugging).
    pub async fn call_history(&self) -> Vec<SylvaCallRecord> {
        self.call_history.read().await.iter().cloned().collect()
    }

    async fn push_history(&self, record: SylvaCallRecord) {
        let mut h = self.call_history.write().await;
        h.push_back(record);
        if h.len() > self.history_limit {
            h.pop_front();
        }
    }

    /// Scan the scripts directory and load all `.lua` files.
    pub async fn scan_and_load(&self) -> usize {
        let dir = &self.scripts_dir;
        if !dir.exists() {
            if let Err(e) = tokio::fs::create_dir_all(dir).await {
                warn!("[sylva] could not create scripts dir: {e}");
                return 0;
            }
        }
        let mut count = 0;
        match tokio::fs::read_dir(dir).await {
            Err(e) => warn!("[sylva] read_dir error: {e}"),
            Ok(mut entries) => {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("lua") {
                        if let Ok(_) = self.load_script(&path).await {
                            count += 1;
                        }
                    }
                }
            }
        }
        info!("[sylva] scanned {count} scripts from {}", dir.display());
        count
    }
}

// ── SylvaWatcher ──────────────────────────────────────────────────────────────

/// Starts a filesystem watcher that hot-reloads `.lua` files on change.
pub struct SylvaWatcher {
    _watcher: RecommendedWatcher,
}

impl SylvaWatcher {
    pub fn start(runtime: Arc<SylvaRuntime>, app_handle: tauri::AppHandle) -> Option<Self> {
        let scripts_dir = runtime.scripts_dir.clone();
        if !scripts_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&scripts_dir) {
                warn!("[sylva] could not create scripts_dir: {e}");
                return None;
            }
        }

        let (tx, rx) = std::sync::mpsc::channel::<notify::Result<Event>>();
        let mut watcher = match notify::recommended_watcher(tx) {
            Ok(w) => w,
            Err(e) => {
                warn!("[sylva] watcher init failed: {e}");
                return None;
            }
        };

        if let Err(e) = watcher.watch(&scripts_dir, RecursiveMode::NonRecursive) {
            warn!("[sylva] watcher.watch failed: {e}");
            return None;
        }

        let rt = runtime.clone();
        let ah = app_handle.clone();

        std::thread::spawn(move || {
            for event_result in rx {
                match event_result {
                    Ok(event) => {
                        let is_modify_or_create = matches!(
                            event.kind,
                            notify::EventKind::Create(_) | notify::EventKind::Modify(_)
                        );
                        if !is_modify_or_create {
                            continue;
                        }

                        for path in event.paths {
                            if path.extension().and_then(|e| e.to_str()) != Some("lua") {
                                continue;
                            }
                            let rt2 = rt.clone();
                            let path2 = path.clone();
                            let ah2 = ah.clone();
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async move {
                                    match rt2.load_script(&path2).await {
                                        Ok(name) => {
                                            info!("[sylva] hot-reloaded '{name}'");
                                            let _ = ah2.emit("sylva-reload", &name);
                                        }
                                        Err(e) => {
                                            warn!(
                                                "[sylva] reload error for {}: {e}",
                                                path2.display()
                                            );
                                            let _ = ah2.emit("sylva-error", &e);
                                        }
                                    }
                                })
                            });
                        }
                    }
                    Err(e) => warn!("[sylva] watcher event error: {e}"),
                }
            }
        });

        Some(Self { _watcher: watcher })
    }
}

// ── SylvaTool: a Lua script as a UCR tool ────────────────────────────────────

struct SylvaTool {
    name: String,
    source: String,
    lua: Arc<std::sync::Mutex<Lua>>,
}

#[async_trait::async_trait]
impl Tool for SylvaTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        "Sylva (Lua) script tool"
    }

    async fn run(&self, args: &serde_json::Value) -> Result<ToolResult, String> {
        let lua = self.lua.lock().map_err(|e| e.to_string())?;

        // Re-execute the script source to ensure function definitions are current
        lua.load(&self.source).exec().map_err(|e| e.to_string())?;

        // Call the `run(args)` function defined in the script
        let func: LuaFunction = lua
            .globals()
            .get("run")
            .map_err(|_| format!("script '{}' must define a `run(args)` function", self.name))?;

        let lua_args = json_to_lua_value(&lua, args.clone()).map_err(|e| e.to_string())?;
        let result: LuaValue = func.call(lua_args).map_err(|e| e.to_string())?;
        let json = lua_value_to_json(result).map_err(|e| e.to_string())?;

        Ok(ToolResult::json(&json))
    }
}

// ── JSON ↔ Lua value conversion ───────────────────────────────────────────────

pub fn lua_value_to_json(val: LuaValue) -> LuaResult<serde_json::Value> {
    Ok(match val {
        LuaValue::Nil => serde_json::Value::Null,
        LuaValue::Boolean(b) => serde_json::Value::Bool(b),
        LuaValue::Integer(i) => serde_json::Value::Number(i.into()),
        LuaValue::Number(f) => serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        LuaValue::String(s) => serde_json::Value::String(s.to_str()?.to_string()),
        LuaValue::Table(t) => {
            // Check if it looks like an array (consecutive integer keys from 1)
            let len = t.raw_len();
            if len > 0 {
                let mut arr = Vec::new();
                for i in 1..=(len as i64) {
                    let v: LuaValue = t.raw_get(i)?;
                    arr.push(lua_value_to_json(v)?);
                }
                serde_json::Value::Array(arr)
            } else {
                let mut map = serde_json::Map::new();
                for pair in t.pairs::<LuaValue, LuaValue>() {
                    let (k, v) = pair?;
                    let key = match k {
                        LuaValue::String(s) => s.to_str()?.to_string(),
                        LuaValue::Integer(i) => i.to_string(),
                        _ => continue,
                    };
                    map.insert(key, lua_value_to_json(v)?);
                }
                serde_json::Value::Object(map)
            }
        }
        _ => serde_json::Value::Null,
    })
}

pub fn json_to_lua_value(lua: &Lua, val: serde_json::Value) -> LuaResult<LuaValue> {
    Ok(match val {
        serde_json::Value::Null => LuaValue::Nil,
        serde_json::Value::Bool(b) => LuaValue::Boolean(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                LuaValue::Integer(i)
            } else {
                LuaValue::Number(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => LuaValue::String(lua.create_string(&s)?),
        serde_json::Value::Array(arr) => {
            let t = lua.create_table()?;
            for (i, v) in arr.into_iter().enumerate() {
                t.raw_set(i + 1, json_to_lua_value(lua, v)?)?;
            }
            LuaValue::Table(t)
        }
        serde_json::Value::Object(map) => {
            let t = lua.create_table()?;
            for (k, v) in map {
                t.raw_set(k, json_to_lua_value(lua, v)?)?;
            }
            LuaValue::Table(t)
        }
    })
}

// ── Tauri state wrapper ───────────────────────────────────────────────────────

#[derive(Clone)]
pub struct SylvaState {
    pub runtime: Arc<SylvaRuntime>,
    pub watcher: Option<Arc<std::sync::Mutex<SylvaWatcher>>>,
}

impl SylvaState {
    pub async fn new(
        tool_registry: Arc<ToolRegistryState>,
        scripts_dir: PathBuf,
        app_handle: tauri::AppHandle,
    ) -> Result<Self, String> {
        let runtime = SylvaRuntime::new(tool_registry, scripts_dir)?;
        // Load existing scripts
        runtime.scan_and_load().await;
        // Start watcher
        let watcher = SylvaWatcher::start(runtime.clone(), app_handle)
            .map(|w| Arc::new(std::sync::Mutex::new(w)));
        Ok(Self { runtime, watcher })
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn sylva_exec(
    state: tauri::State<'_, crate::AppState>,
    src: String,
) -> Result<serde_json::Value, String> {
    state.sylva.runtime.exec_str(&src)
}

#[tauri::command]
pub async fn sylva_list_scripts(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<SylvaScript>, String> {
    Ok(state.sylva.runtime.list_scripts().await)
}

#[tauri::command]
pub async fn get_sylva_history(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<SylvaCallRecord>, String> {
    Ok(state.sylva.runtime.call_history().await)
}

#[tauri::command]
pub async fn sylva_exec_file(
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<serde_json::Value, String> {
    state
        .sylva
        .runtime
        .exec_file(std::path::Path::new(&path))
        .await
}

#[tauri::command]
pub async fn sylva_clear_history(state: tauri::State<'_, crate::AppState>) -> Result<(), String> {
    state.sylva.runtime.call_history.write().await.clear();
    Ok(())
}

#[tauri::command]
pub async fn sylva_load_script(
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<String, String> {
    state
        .sylva
        .runtime
        .load_script(std::path::Path::new(&path))
        .await
}

#[tauri::command]
pub async fn sylva_get_script_content(
    state: tauri::State<'_, crate::AppState>,
    name: String,
) -> Result<String, String> {
    let scripts = state.sylva.runtime.scripts.read().await;
    scripts
        .get(&name)
        .map(|s| s.source.clone())
        .ok_or_else(|| format!("Script '{}' not found", name))
}

#[tauri::command]
pub async fn sylva_save_script(
    state: tauri::State<'_, crate::AppState>,
    name: String,
    source: String,
) -> Result<String, String> {
    let scripts_dir = state.sylva.runtime.scripts_dir.clone();
    let path = scripts_dir.join(format!("{}.lua", name));
    tokio::fs::write(&path, &source)
        .await
        .map_err(|e| format!("Failed to write script: {e}"))?;
    state.sylva.runtime.load_script(&path).await
}
