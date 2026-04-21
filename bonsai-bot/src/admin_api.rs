use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use dashmap::DashMap;
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio::process::Command;
use tokio::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use sha2::Digest;
use std::path::PathBuf;
use std::fs::OpenOptions;
use hex;
use chrono::{self, TimeZone};

use crate::config::keyring_set;
use crate::metrics::SharedMetrics;
use crate::session::Db;

fn pid_running(pid: i64) -> bool {
    #[cfg(target_os = "windows")]
    {
        match std::process::Command::new("tasklist").args(["/FI", &format!("PID eq {}", pid), "/NH"]).output() {
            Ok(out) => {
                let s = String::from_utf8_lossy(&out.stdout);
                let s = s.trim();
                if s.is_empty() || s.contains("No tasks") { return false; }
                // crude check: if output contains pid number
                s.contains(&pid.to_string())
            }
            Err(_) => false,
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        match std::process::Command::new("ps").arg("-p").arg(pid.to_string()).status() {
            Ok(st) => st.success(),
            Err(_) => false,
        }
    }
}

fn kill_pid(pid: i64) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        match std::process::Command::new("taskkill").args(["/PID", &pid.to_string(), "/T", "/F"]).status() {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(format!("taskkill exit: {}", s)),
            Err(e) => Err(e.to_string()),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        match std::process::Command::new("kill").arg("-TERM").arg(pid.to_string()).status() {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(format!("kill exit: {}", s)),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Per-platform connection state, written by platform adapters, read by /status.
pub type PlatformStates = Arc<DashMap<String, String>>;

/// A broadcast request forwarded from the admin API to the main dispatch loop.
#[derive(Debug, Clone)]
pub struct BroadcastRequest {
    pub message:   String,
    pub platforms: Vec<String>,
}

#[derive(Clone)]
pub struct AdminState {
    pub metrics:         SharedMetrics,
    pub platform_states: PlatformStates,
    pub db:              Db,
    pub broadcast_tx:    mpsc::Sender<BroadcastRequest>,
    /// In-memory token, wrapped for atomic rotation without restart.
    pub admin_token:     Arc<RwLock<String>>,
    /// Allowed ports for reclaim operations. Empty means no ports allowed by default.
    pub reclaim_allowed_ports: Vec<u16>,
    /// Allowed script path prefixes for runtime start requests. Empty -> fallback allowlist.
    pub allowed_script_paths: Vec<String>,
    /// Per-runtime limits (timeout, per-user concurrency, etc.)
    pub runtime_limits: crate::config::RuntimeLimits,
    /// Spawned runtime child processes keyed by generated id.
    pub runtime_children: Arc<Mutex<HashMap<String, RuntimeInfo>>>,
}

#[allow(dead_code)]
pub(crate) struct RuntimeInfo {
    controller: Option<Box<dyn bonsai_runtime::RuntimeController + Send + Sync>>,
    pid: Option<i64>,
    user: Option<String>,
    script: String,
    started_at: chrono::DateTime<chrono::Utc>,
    #[allow(dead_code)]
    timeout_secs: Option<u64>,
}

#[allow(dead_code)]
pub struct AdminHandle {
    shutdown_tx: Option<oneshot::Sender<()>>,
    join:        JoinHandle<()>,
    pub port:    u16,
}

impl AdminHandle {
    #[allow(dead_code)]
    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        let _ = (&mut self.join).await;
        // Remove persisted port file when shutting down.
        if let Some(dir) = crate::config::config_dir() {
            let path = dir.join("bonsai-bot-port.json");
            let _ = std::fs::remove_file(path);
        } else {
            let _ = std::fs::remove_file("bonsai-bot-port.json");
        }
    }
}

pub async fn start(
    preferred_port:  u16,
    metrics:         SharedMetrics,
    platform_states: PlatformStates,
    db:              Db,
    broadcast_tx:    mpsc::Sender<BroadcastRequest>,
    admin_token:     String,
) -> Result<AdminHandle, String> {
    // If a persisted port file exists but points to an unhealthy API, remove it
    if let Some(dir) = crate::config::config_dir() {
        let path = dir.join("bonsai-bot-port.json");
        if path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&contents) {
                    if let Some(p) = val.get("port").and_then(|v| v.as_u64()) {
                        let p = p as u16;
                        if !crate::port_manager::is_api_healthy("127.0.0.1", p).await {
                            let _ = std::fs::remove_file(&path);
                            tracing::info!("[admin-api] removed stale port file {path:?} (port={p})");
                        }
                    }
                }
            }
        }
    } else if std::path::Path::new("bonsai-bot-port.json").exists() {
        if let Ok(contents) = std::fs::read_to_string("bonsai-bot-port.json") {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&contents) {
                if let Some(p) = val.get("port").and_then(|v| v.as_u64()) {
                    let p = p as u16;
                    if !crate::port_manager::is_api_healthy("127.0.0.1", p).await {
                        let _ = std::fs::remove_file("bonsai-bot-port.json");
                        tracing::info!("[admin-api] removed stale local port file bonsai-bot-port.json (port={p})");
                    }
                }
            }
        }
    }
    let (port, listener) = crate::port_manager::allocate_listener(preferred_port, 4).await?;

    // Persist the chosen admin port atomically (with metadata) so other local tooling can discover it.
    if let Err(e) = crate::port_manager::persist_port(port, &admin_token) {
        tracing::warn!("[admin-api] failed to persist admin port: {e}");
    }

    // Load reclaim whitelist from config at startup (can be edited by user via config file).
    let cfg = crate::config::load_config();
    let state = AdminState {
        metrics,
        platform_states,
        db,
        broadcast_tx,
        admin_token: Arc::new(RwLock::new(admin_token)),
        reclaim_allowed_ports: cfg.reclaim_allowed_ports.clone(),
        allowed_script_paths: cfg.allowed_script_paths.clone(),
        runtime_limits: cfg.runtime_limits.clone(),
        runtime_children: Arc::new(Mutex::new(HashMap::new())),
    };

    // Reconcile persisted runtime records on startup; populate in-memory map with metadata
    {
        let state_clone = state.clone();
        let db = state_clone.db.clone();
        let children = state_clone.runtime_children.clone();
        tokio::spawn(async move {
            let records = crate::session::list_runtime_records(&db).await;
            for rec in records.into_iter() {
                if rec["status"] == "running" {
                    let id = rec["id"].as_str().unwrap_or_default().to_string();
                    let pid = rec["pid"].as_i64();
                    let script = rec["script"].as_str().unwrap_or_default().to_string();
                    let user = rec["user"].as_str().map(|s| s.to_string());
                    let started_at = rec["started_at"].as_i64().unwrap_or(0);
                    let started_dt = chrono::Utc.timestamp_opt(started_at, 0).single().unwrap_or_else(|| chrono::Utc::now());
                    // Check whether PID is running; if not, mark as orphan
                    let alive = pid.and_then(|p| Some(pid_running(p))).unwrap_or(false);
                    if !alive {
                        let _ = crate::session::update_runtime_status(&db, &id, "orphan", None).await;
                        continue;
                    }
                    // Insert metadata with no controller handle (cannot reattach)
                    let info = RuntimeInfo {
                        controller: None,
                        pid,
                        user,
                        script,
                        started_at: started_dt,
                        timeout_secs: rec["timeout_secs"].as_i64().map(|v| v as u64),
                    };
                    let mut m = children.lock().await;
                    m.insert(id, info);
                }
            }
        });
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health",                    get(health))
        .route("/reclaim-listener",          post(reclaim_listener))
        .route("/runtime/start",            post(start_runtime))
        .route("/runtime/stop",             post(stop_runtime))
        .route("/runtime/list",             get(list_runtimes))
        .route("/status",                    get(status))
        .route("/sessions",                  get(sessions_handler))
        .route("/broadcast",                 post(broadcast_handler))
        .route("/metrics",                   get(metrics_handler))
        .route("/config/reload",             post(config_reload))
        .route("/config/rotate-admin-token", post(rotate_token))
        .layer(cors)
        .with_state(state);

    tracing::info!("[admin-api] Listening on http://127.0.0.1:{port}");

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let join = tokio::spawn(async move {
        let server = axum::serve(listener, app).with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });
        if let Err(e) = server.await {
            tracing::error!("[admin-api] error: {e}");
        }
    });

    Ok(AdminHandle { shutdown_tx: Some(shutdown_tx), join, port })
}

#[derive(serde::Deserialize)]
struct ReclaimRequest {
    ports: Option<Vec<u16>>,
    #[serde(default)]
    force_kill: bool,
    #[serde(default)]
    use_handle: bool,
}

async fn reclaim_listener(
    State(s): State<AdminState>,
    headers: HeaderMap,
    Json(body): Json<ReclaimRequest>,
) -> impl IntoResponse {
    // extract presented token (for audit) and verify auth
    let presented_token = headers.get("authorization").and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer ")).map(|s| s.to_string());
    let token_hash = presented_token.as_ref().map(|t| {
        let digest = sha2::Sha256::digest(t.as_bytes());
        format!("sha256:{}", hex::encode(digest))
    });

    let authorized = check_auth(&headers, &s);
    // Audit the attempt
    let audit_details = json!({ "ports": body.ports, "force_kill": body.force_kill, "use_handle": body.use_handle });
    let _ = write_admin_audit("reclaim_attempt", token_hash.as_deref(), &audit_details);
    if !authorized {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))).into_response();
    }

    // Try several likely locations for the script: current dir and exe parent.
    let mut candidates = Vec::new();
    if let Ok(cwd) = std::env::current_dir() { candidates.push(cwd.join("scripts").join("reclaim-listener.ps1")); }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(p) = exe.parent() { candidates.push(p.join("scripts").join("reclaim-listener.ps1")); }
    }
    let script = candidates.into_iter().find(|p| p.exists());
    let script = match script {
        Some(p) => p,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "reclaim script not found"}))).into_response(),
    };

    // Enforce whitelist: only allow requested ports that are in the configured reclaim_allowed_ports
    let allowed_ports = if !s.reclaim_allowed_ports.is_empty() {
        s.reclaim_allowed_ports.clone()
    } else {
        // If no explicit allowed list, try to include the persisted bot admin port (if present)
        let mut v = Vec::new();
        if let Some(cfg_dir) = crate::config::config_dir() {
            let path = cfg_dir.join("bonsai").join("bonsai-bot-port.json");
            if path.exists() {
                if let Ok(s2) = std::fs::read_to_string(&path) {
                    if let Ok(vj) = serde_json::from_str::<Value>(&s2) {
                        if let Some(p) = vj.get("port").and_then(|n| n.as_u64()) {
                            v.push(p as u16);
                        }
                    }
                }
            }
        }
        // fallback to local file
        let local = PathBuf::from("bonsai-bot-port.json");
        if local.exists() {
            if let Ok(s3) = std::fs::read_to_string(&local) {
                if let Ok(vj) = serde_json::from_str::<Value>(&s3) {
                    if let Some(p) = vj.get("port").and_then(|n| n.as_u64()) {
                        v.push(p as u16);
                    }
                }
            }
        }
        v
    };

    // Build PowerShell command invocation string.
    let mut ps_cmd = format!("& '{}'", script.to_string_lossy().replace("'", "''"));
    if let Some(ports) = &body.ports {
        if !ports.is_empty() {
            // validate ports against allowed list
            if allowed_ports.is_empty() {
                return (StatusCode::FORBIDDEN, Json(json!({"error": "No ports are allowed to be reclaimed on this host"}))).into_response();
            }
            for p in ports.iter() {
                if !allowed_ports.contains(p) {
                    return (StatusCode::FORBIDDEN, Json(json!({"error": format!("port {} not allowed", p)}))).into_response();
                }
            }
            let ports_str = ports.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(",");
            ps_cmd.push_str(&format!(" -Ports {}", ports_str));
        }
    }
    if body.force_kill { ps_cmd.push_str(" -ForceKill"); }
    if body.use_handle { ps_cmd.push_str(" -UseHandle"); }

    let mut cmd = Command::new("powershell");
    cmd.arg("-NoProfile").arg("-ExecutionPolicy").arg("Bypass").arg("-Command").arg(ps_cmd);

    match cmd.output().await {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let res = json!({ "ok": true, "stdout": stdout, "stderr": stderr, "code": output.status.code() });
            let _ = write_admin_audit("reclaim_result", token_hash.as_deref(), &res);
            Json(res).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

/// Append a small JSON audit line to the admin audit log in the config dir (or local file).
fn write_admin_audit(action: &str, token_hash: Option<&str>, details: &Value) -> Result<(), String> {
    let entry = json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "action": action,
        "token_hash": token_hash,
        "details": details,
    });

    let target_path: PathBuf = match crate::config::config_dir() {
        Some(d) => d.join("bonsai").join("admin-audit.log"),
        None => PathBuf::from("admin-audit.log"),
    };
    if let Some(parent) = target_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // Rotate/prune older audit logs if needed
    if let Err(e) = rotate_and_prune_audit_log(&target_path, 10 * 1024 * 1024, 30) {
        tracing::warn!("[admin-audit] rotation check failed: {}", e);
    }
    let s = serde_json::to_string(&entry).map_err(|e| e.to_string())?;
    let mut f = OpenOptions::new().create(true).append(true).open(&target_path).map_err(|e| e.to_string())?;
    use std::io::Write;
    f.write_all(s.as_bytes()).map_err(|e| e.to_string())?;
    f.write_all(b"\n").map_err(|e| e.to_string())?;
    Ok(())
}

fn rotate_and_prune_audit_log(target: &PathBuf, max_bytes: u64, retain_days: i64) -> Result<(), String> {
    use std::fs;
    if !target.exists() {
        return Ok(());
    }
    // Rotate if file is larger than threshold
    if let Ok(meta) = fs::metadata(target) {
        if meta.len() > max_bytes {
            let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
            let rotated = target.with_extension(format!("log.{}", stamp));
            fs::rename(target, &rotated).map_err(|e| e.to_string())?;
        }
    }
    // Prune files older than retain_days in same directory that match admin-audit.log.*
    if let Some(parent) = target.parent() {
        if let Ok(entries) = fs::read_dir(parent) {
            let cutoff = chrono::Utc::now() - chrono::Duration::days(retain_days);
            for e in entries.flatten() {
                if let Ok(fname) = e.file_name().into_string() {
                    if fname.starts_with("admin-audit.log.") {
                        if let Ok(meta) = e.metadata() {
                            if let Ok(mtime) = meta.modified() {
                                if let Ok(since) = mtime.duration_since(std::time::UNIX_EPOCH) {
                                    let dt = chrono::Utc.timestamp_opt(since.as_secs() as i64, 0).single().unwrap_or(chrono::Utc::now());
                                    if dt < cutoff {
                                        let _ = std::fs::remove_file(e.path());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[derive(serde::Deserialize)]
struct StartRuntimeRequest {
    kind: String,
    script: String,
    port: Option<u16>,
    #[serde(default)]
    user: Option<String>,
    #[serde(default)]
    timeout_secs: Option<u64>,
}

#[derive(serde::Deserialize)]
struct StopRuntimeRequest {
    id: String,
}

async fn start_runtime(
    State(s): State<AdminState>,
    headers: HeaderMap,
    Json(body): Json<StartRuntimeRequest>,
) -> impl IntoResponse {
    if !check_auth(&headers, &s) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))).into_response();
    }

    // Resolve script path and enforce allowed-script-paths policy.
    let script_candidate = PathBuf::from(&body.script);
    let script_abs = if script_candidate.is_absolute() {
        script_candidate
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(script_candidate)
    };
    let script_canon = script_abs.canonicalize().unwrap_or(script_abs.clone());

    // Build allowed base paths list (from config or fallback locations)
    let mut allowed_bases: Vec<PathBuf> = Vec::new();
    if !s.allowed_script_paths.is_empty() {
        for p in s.allowed_script_paths.iter() {
            let pb = PathBuf::from(p);
            let canon = pb.canonicalize().unwrap_or(pb);
            allowed_bases.push(canon);
        }
    } else {
        if let Ok(cwd) = std::env::current_dir() { allowed_bases.push(cwd.join("runtimes")); }
        if let Ok(exe) = std::env::current_exe() { if let Some(p) = exe.parent() { allowed_bases.push(p.join("runtimes")); } }
    }

    let mut allowed = false;
    for base in allowed_bases.iter() {
        if script_canon.starts_with(base) {
            allowed = true;
            break;
        }
    }
    if !allowed {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "script path not allowed"}))).into_response();
    }

    // Enforce per-user concurrency limit (if present)
    if let Some(user) = &body.user {
        if let Some(max) = s.runtime_limits.max_instances_per_user {
            let map = s.runtime_children.lock().await;
            let count = map.values().filter(|info| info.user.as_deref() == Some(user.as_str())).count();
            if (count as u32) >= max {
                return (StatusCode::TOO_MANY_REQUESTS, Json(json!({"error": "user runtime limit reached"}))).into_response();
            }
        }
    }

    // Enforce timeout limit if provided
    let timeout = match body.timeout_secs {
        Some(t) => {
            if let Some(max) = s.runtime_limits.max_runtime_secs {
                if t > max {
                    return (StatusCode::BAD_REQUEST, Json(json!({"error": "requested timeout exceeds configured maximum"}))).into_response();
                }
            }
            Some(t)
        }
        None => s.runtime_limits.max_runtime_secs,
    };

    let id = uuid::Uuid::new_v4().to_string();
    let rm = bonsai_runtime::RuntimeManager::new();
    let controller_res = match body.kind.as_str() {
        "python" => {
            let port = body.port.unwrap_or(0);
            rm.start_python_worker(&script_canon.to_string_lossy(), port).await
        }
        "clojurewasm" => rm.start_clojurewasm_worker(&script_canon.to_string_lossy(), timeout).await,
        "babashka" => rm.start_babashka_worker(&script_canon.to_string_lossy()).await,
        _ => return (StatusCode::BAD_REQUEST, Json(json!({"error": "unknown runtime kind"}))).into_response(),
    };

    match controller_res {
        Ok(controller) => {
            let pid = controller.pid().unwrap_or(0);
            // store controller for lifecycle management and persist metadata
            {
                let mut map = s.runtime_children.lock().await;
                map.insert(id.clone(), RuntimeInfo {
                    controller: Some(controller),
                    pid: Some(pid),
                    user: body.user.clone(),
                    script: script_canon.to_string_lossy().into_owned(),
                    started_at: chrono::Utc::now(),
                    timeout_secs: timeout,
                });
            }
            let _ = crate::session::upsert_runtime_record(
                &s.db,
                &id,
                &body.kind,
                &script_canon.to_string_lossy(),
                body.user.as_deref(),
                Some(pid),
                "running",
                chrono::Utc::now().timestamp(),
                timeout.map(|t| t as i64),
            ).await;

            // If timeout was requested, spawn a monitor that will kill the runtime after timeout
            if let Some(tsec) = timeout {
                let children_map = s.runtime_children.clone();
                let id_clone = id.clone();
                let db_clone = s.db.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(tsec)).await;
                    let mut map = children_map.lock().await;
                    if let Some(mut info) = map.remove(&id_clone) {
                        // kill via controller if available, otherwise by PID
                        if let Some(mut ctrl) = info.controller.take() {
                            let _ = ctrl.kill().await;
                            let _ = ctrl.wait().await;
                        } else if let Some(p) = info.pid {
                            let _ = kill_pid(p);
                        }
                        let _ = write_admin_audit("runtime_timeout", None, &json!({"id": id_clone}));
                        let _ = crate::session::update_runtime_status(&db_clone, &id_clone, "timed_out", None).await;
                    }
                });
            }

            Json(json!({ "id": id, "pid": pid })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

async fn stop_runtime(
    State(s): State<AdminState>,
    headers: HeaderMap,
    Json(body): Json<StopRuntimeRequest>,
) -> impl IntoResponse {
    if !check_auth(&headers, &s) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))).into_response();
    }
    let mut map = s.runtime_children.lock().await;
    if let Some(mut info) = map.remove(&body.id) {
        // attempt graceful kill: prefer controller handle, otherwise kill by PID
        if let Some(mut ctrl) = info.controller.take() {
            if let Err(e) = ctrl.kill().await {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("kill failed: {}", e)}))).into_response();
            }
            match ctrl.wait().await {
                Ok(code_opt) => {
                    let _ = crate::session::update_runtime_status(&s.db, &body.id, "stopped", None).await;
                    return Json(json!({"ok": true, "code": code_opt})).into_response()
                }
                Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
            }
        } else if let Some(pid) = info.pid {
            // Kill by PID via platform command
            if let Err(e) = kill_pid(pid) {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("kill-pid failed: {}", e)}))).into_response();
            }
            let _ = crate::session::update_runtime_status(&s.db, &body.id, "stopped", None).await;
            return Json(json!({"ok": true, "killed_pid": pid})).into_response();
        } else {
            let _ = crate::session::update_runtime_status(&s.db, &body.id, "unknown", None).await;
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "no handle or pid for runtime"}))).into_response();
        }
    } else {
        (StatusCode::NOT_FOUND, Json(json!({"error": "runtime id not found"}))).into_response()
    }
}

async fn list_runtimes(
    State(s): State<AdminState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !check_auth(&headers, &s) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))).into_response();
    }
    let map = s.runtime_children.lock().await;
    let mut out = Vec::new();
    for (id, info) in map.iter() {
        let pid = info.pid.or_else(|| info.controller.as_ref().and_then(|c| c.pid())).unwrap_or(0);
        let controllable = info.controller.is_some();
        out.push(json!({"id": id, "pid": pid, "controllable": controllable, "user": info.user, "script": info.script, "started_at": info.started_at.to_rfc3339()}));
    }
    Json(json!({"runtimes": out})).into_response()
}


fn check_auth(headers: &HeaderMap, state: &AdminState) -> bool {
    let token = state.admin_token.read().unwrap();
    headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|tok| tok == token.as_str())
        .unwrap_or(false)
}

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

async fn status(
    State(s): State<AdminState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !check_auth(&headers, &s) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))).into_response();
    }
    let platforms: serde_json::Map<String, Value> = s.platform_states
        .iter()
        .map(|entry| (entry.key().clone(), Value::String(entry.value().clone())))
        .collect();
    Json(json!({
        "status": "ok",
        "platforms": platforms,
    })).into_response()
}

async fn metrics_handler(
    State(s): State<AdminState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !check_auth(&headers, &s) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))).into_response();
    }
    Json(s.metrics.snapshot()).into_response()
}

async fn config_reload(
    State(s): State<AdminState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !check_auth(&headers, &s) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))).into_response();
    }
    tracing::info!("[admin-api] Config reload requested");
    Json(json!({ "status": "reload_scheduled" })).into_response()
}

async fn rotate_token(
    State(s): State<AdminState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !check_auth(&headers, &s) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))).into_response();
    }
    let new_tok = uuid::Uuid::new_v4().to_string();
    match keyring_set("bot_admin_token", &new_tok) {
        Ok(()) => {
            *s.admin_token.write().unwrap() = new_tok;
            tracing::info!("[admin-api] bot_admin_token rotated and active immediately");
            Json(json!({ "status": "rotated" })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))).into_response(),
    }
}

async fn sessions_handler(
    State(s): State<AdminState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !check_auth(&headers, &s) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))).into_response();
    }
    let sessions = crate::session::list_active_sessions(&s.db).await;
    Json(json!({ "sessions": sessions })).into_response()
}

#[derive(serde::Deserialize)]
struct BroadcastBody {
    message:   String,
    #[serde(default)]
    platforms: Vec<String>,
}

async fn broadcast_handler(
    State(s): State<AdminState>,
    headers: HeaderMap,
    Json(body): Json<BroadcastBody>,
) -> impl IntoResponse {
    if !check_auth(&headers, &s) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))).into_response();
    }
    if body.message.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "message is required"}))).into_response();
    }
    let req = BroadcastRequest {
        message:   body.message,
        platforms: body.platforms,
    };
    match s.broadcast_tx.try_send(req) {
        Ok(()) => Json(json!({ "status": "queued" })).into_response(),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error": "broadcast queue full"}))).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::Metrics;
    use tokio_rusqlite::Connection;
    use std::path::PathBuf;

    #[tokio::test]
    async fn audit_write_and_readable() {
        let details = json!({"x": 1});
        // write an audit entry
        let _ = write_admin_audit("unit_test", Some("sha256:deadbeef"), &details);
        // read back from expected path
        let path = crate::config::config_dir()
            .map(|d| d.join("bonsai").join("admin-audit.log"))
            .unwrap_or_else(|| PathBuf::from("admin-audit.log"));
        let s = std::fs::read_to_string(&path).expect("audit log readable");
        assert!(s.contains("unit_test"));
    }

    #[tokio::test]
    async fn check_auth_success_and_failure() {
        let metrics = Arc::new(Metrics::default());
        let platform_states = Arc::new(DashMap::new());
        let db = Arc::new(Connection::open_in_memory().await.unwrap());
        crate::session::migrate(&db).await.unwrap();
        let (tx, _rx) = mpsc::channel::<BroadcastRequest>(1);
        let state = AdminState {
            metrics,
            platform_states,
            db,
            broadcast_tx: tx,
            admin_token: Arc::new(RwLock::new("s3cr3t".to_string())),
            reclaim_allowed_ports: vec![],
            runtime_children: Arc::new(Mutex::new(HashMap::new())),
        };

        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer s3cr3t".parse().unwrap());
        assert!(check_auth(&headers, &state));

        let headers2 = HeaderMap::new();
        assert!(!check_auth(&headers2, &state));
    }
}
