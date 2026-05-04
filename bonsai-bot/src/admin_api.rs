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
// CORS intentionally omitted — admin API is loopback-only (127.0.0.1) and
// must never be accessible from browser origins.
use sha2::Digest;
use std::path::PathBuf;
use std::fs::OpenOptions;
use hex;
use chrono::{self};

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
    /// Allowed ports for reclaim operations — hot-reloadable.
    pub reclaim_allowed_ports: Arc<RwLock<Vec<u16>>>,
    /// Allowed script path prefixes for runtime start requests — hot-reloadable.
    pub allowed_script_paths: Arc<RwLock<Vec<String>>>,
    /// Per-runtime limits — hot-reloadable.
    pub runtime_limits: Arc<RwLock<crate::config::RuntimeLimits>>,
    /// Spawned runtime child processes keyed by generated id.
    pub runtime_children: Arc<Mutex<HashMap<String, RuntimeInfo>>>,
    /// Buddy API base URL — used by /health/full
    pub buddy_api_url: String,
    /// Workspace API base URL — used by /health/full
    pub workspace_api_url: String,
    /// Swarm peer list — used by /health/full
    pub swarm_peers: Vec<crate::config::SwarmPeer>,
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
        reclaim_allowed_ports: Arc::new(RwLock::new(cfg.reclaim_allowed_ports.clone())),
        allowed_script_paths: Arc::new(RwLock::new(cfg.allowed_script_paths.clone())),
        runtime_limits: Arc::new(RwLock::new(cfg.runtime_limits.clone())),
        runtime_children: Arc::new(Mutex::new(HashMap::new())),
        buddy_api_url:    cfg.buddy_api_url.clone(),
        workspace_api_url: cfg.workspace_api_url.clone(),
        swarm_peers:      cfg.swarm_peers.clone(),
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
                    let started_dt = chrono::DateTime::from_timestamp(started_at, 0)
                        .unwrap_or_else(chrono::Utc::now);
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
        .route("/runtime/create-skill",      post(create_skill))
        .route("/skills",                    get(list_skills_handler))
        .route("/skills/:id/toggle",         post(toggle_skill_handler))
        .route("/skills/:id",               axum::routing::delete(delete_skill_handler))
        .route("/health/full",               get(health_full))
        .route("/audit-log",                 get(audit_log_handler))
        .route("/metrics/prometheus",        get(prometheus_handler))
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
    let allowed_ports = {
        let rp = s.reclaim_allowed_ports.read().unwrap();
        rp.clone()
    };
    let allowed_ports = if !allowed_ports.is_empty() {
        allowed_ports
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
                                    let dt = chrono::DateTime::from_timestamp(since.as_secs() as i64, 0)
                                        .unwrap_or_else(chrono::Utc::now);
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
    let script_paths_snap = s.allowed_script_paths.read().unwrap().clone();
    if !script_paths_snap.is_empty() {
        for p in script_paths_snap.iter() {
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
    let (max_instances, max_secs) = {
        let rl = s.runtime_limits.read().unwrap();
        (rl.max_instances_per_user, rl.max_runtime_secs)
    };
    if let Some(user) = &body.user {
        if let Some(max) = max_instances {
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
            if let Some(max) = max_secs {
                if t > max {
                    return (StatusCode::BAD_REQUEST, Json(json!({"error": "requested timeout exceeds configured maximum"}))).into_response();
                }
            }
            Some(t)
        }
        None => max_secs,
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
    let cfg = crate::config::load_config();
    {
        *s.reclaim_allowed_ports.write().unwrap() = cfg.reclaim_allowed_ports.clone();
        *s.allowed_script_paths.write().unwrap()  = cfg.allowed_script_paths.clone();
        *s.runtime_limits.write().unwrap()         = cfg.runtime_limits.clone();
    }
    tracing::info!("[admin-api] Config reloaded from disk (reclaim_ports={}, script_paths={}, max_secs={:?})",
        cfg.reclaim_allowed_ports.len(),
        cfg.allowed_script_paths.len(),
        cfg.runtime_limits.max_runtime_secs,
    );
    Json(json!({ "status": "reloaded" })).into_response()
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
            reclaim_allowed_ports: Arc::new(RwLock::new(vec![])),
            allowed_script_paths: Arc::new(RwLock::new(vec![])),
            runtime_limits: Arc::new(RwLock::new(crate::config::RuntimeLimits::default())),
            runtime_children: Arc::new(Mutex::new(HashMap::new())),
            buddy_api_url:    "http://127.0.0.1:11420".into(),
            workspace_api_url: "http://127.0.0.1:11369".into(),
            swarm_peers:      vec![],
        };

        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer s3cr3t".parse().unwrap());
        assert!(check_auth(&headers, &state));

        let headers2 = HeaderMap::new();
        assert!(!check_auth(&headers2, &state));
    }
}

// ── Skill creation ─────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct CreateSkillBody {
    name:        String,
    description: String,
    script:      String,
    language:    String,
}

async fn create_skill(
    State(s): State<AdminState>,
    headers: HeaderMap,
    Json(body): Json<CreateSkillBody>,
) -> impl IntoResponse {
    if !check_auth(&headers, &s) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))).into_response();
    }

    // Validate language
    let ext = match body.language.as_str() {
        "python"   => "py",
        "clojure"  => "clj",
        "babashka" => "clj",
        other => return (StatusCode::BAD_REQUEST, Json(json!({"error": format!("unsupported language: {other}")}))).into_response(),
    };

    // Sanitize name to safe filename (alphanumeric + underscore)
    let safe_name: String = body.name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    if safe_name.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "name is required"}))).into_response();
    }

    // Determine skills directory (first allowed_script_paths entry, else config_dir/skills)
    let skills_dir: std::path::PathBuf = {
        let paths = s.allowed_script_paths.read().unwrap();
        if let Some(first) = paths.first() {
            std::path::PathBuf::from(first).join("skills")
        } else if let Some(dir) = crate::config::config_dir() {
            dir.join("bonsai").join("skills")
        } else {
            std::path::PathBuf::from("skills")
        }
    };

    if let Err(e) = std::fs::create_dir_all(&skills_dir) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("mkdir: {e}")}))).into_response();
    }

    let skill_id = uuid::Uuid::new_v4().to_string();
    let filename = format!("{safe_name}_{}.{ext}", &skill_id[..8]);
    let script_path = skills_dir.join(&filename);

    if let Err(e) = std::fs::write(&script_path, &body.script) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("write: {e}")}))).into_response();
    }

    // Persist skill manifest alongside the script
    let manifest = json!({
        "id":          &skill_id,
        "name":        &body.name,
        "description": &body.description,
        "language":    &body.language,
        "script_path": script_path.to_string_lossy(),
        "created_at":  chrono::Utc::now().to_rfc3339(),
    });
    let manifest_path = script_path.with_extension("skill.json");
    let _ = std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap_or_default());

    tracing::info!("[admin-api] Skill created: id={skill_id} name={} lang={}", body.name, body.language);

    // Also persist the skill record in SQLite so it survives restarts
    let now = chrono::Utc::now().timestamp();
    let skill_rec = crate::session::SkillRecord {
        id:          skill_id.clone(),
        name:        body.name.clone(),
        description: body.description.clone(),
        language:    body.language.clone(),
        script_path: script_path.to_string_lossy().to_string(),
        version:     1,
        enabled:     true,
        created_at:  now,
        updated_at:  now,
    };
    let _ = crate::session::upsert_skill(&s.db, skill_rec).await;

    (StatusCode::CREATED, Json(json!({
        "status":      "created",
        "skill_id":    skill_id,
        "script_path": script_path.to_string_lossy(),
    }))).into_response()
}

// ── Skill registry endpoints ──────────────────────────────────────────────────

async fn list_skills_handler(
    State(s): State<AdminState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !check_auth(&headers, &s) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))).into_response();
    }
    let skills = crate::session::list_skills(&s.db).await;
    Json(json!({ "skills": skills })).into_response()
}

#[derive(serde::Deserialize)]
struct ToggleBody { enabled: bool }

async fn toggle_skill_handler(
    State(s): State<AdminState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<ToggleBody>,
) -> impl IntoResponse {
    if !check_auth(&headers, &s) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))).into_response();
    }
    match crate::session::toggle_skill(&s.db, id.clone(), body.enabled).await {
        Ok(()) => {
            tracing::info!("[admin-api] Skill {id} enabled={}", body.enabled);
            Json(json!({ "status": "updated", "id": id, "enabled": body.enabled })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

async fn delete_skill_handler(
    State(s): State<AdminState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    if !check_auth(&headers, &s) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))).into_response();
    }
    match crate::session::delete_skill(&s.db, id.clone()).await {
        Ok(()) => {
            tracing::info!("[admin-api] Skill {id} deleted");
            Json(json!({ "status": "deleted", "id": id })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

// ── Full health dashboard ─────────────────────────────────────────────────────

async fn health_full(
    State(s): State<AdminState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !check_auth(&headers, &s) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))).into_response();
    }

    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .build()
        .unwrap_or_default();

    // Buddy API check
    let buddy_check = {
        let url = format!("{}/health", s.buddy_api_url);
        let t0  = std::time::Instant::now();
        let ok  = tokio::time::timeout(Duration::from_secs(3), http.get(&url).send())
            .await.ok().and_then(|r| r.ok()).map(|r| r.status().is_success()).unwrap_or(false);
        json!({ "healthy": ok, "url": s.buddy_api_url, "latency_ms": t0.elapsed().as_millis() })
    };

    // Workspace / llama-server check
    let llama_check = {
        let status_url = format!("{}/v1/orchestrator/status", s.workspace_api_url);
        let t0 = std::time::Instant::now();
        match tokio::time::timeout(Duration::from_secs(3), http.get(&status_url).send()).await {
            Ok(Ok(resp)) if resp.status().is_success() => {
                if let Ok(body) = resp.json::<Value>().await {
                    let slots = body["slots"].as_array().cloned().unwrap_or_default();
                    let ready = slots.iter().find(|s| s["state"]["state"] == "ready");
                    let (slot_idx, model_id, loaded) = ready
                        .map(|s| (
                            s["index"].as_u64().unwrap_or(0),
                            s["state"]["model_id"].as_str().unwrap_or("").to_string(),
                            true,
                        ))
                        .unwrap_or((0, String::new(), false));
                    json!({ "healthy": loaded, "slot": slot_idx, "model": model_id, "loaded": loaded, "latency_ms": t0.elapsed().as_millis() })
                } else {
                    json!({ "healthy": false, "error": "parse error" })
                }
            }
            _ => json!({ "healthy": false, "error": "workspace unreachable" }),
        }
    };

    // Platform checks
    let mut platform_checks = serde_json::Map::new();
    for name in &["discord", "telegram", "email", "matrix"] {
        let state_str = s.platform_states.get(*name).map(|v| v.clone()).unwrap_or_else(|| "not configured".to_string());
        let healthy = matches!(state_str.as_str(), "connected" | "polling" | "syncing");
        platform_checks.insert(name.to_string(), json!({ "healthy": healthy, "state": state_str }));
    }

    // Swarm peer checks
    let mut peer_results = Vec::new();
    for peer in &s.swarm_peers {
        let url = format!("{}/health", peer.admin_url);
        let ok = tokio::time::timeout(Duration::from_secs(2),
            http.get(&url).header("authorization", format!("Bearer {}", peer.token)).send())
            .await.ok().and_then(|r| r.ok()).map(|r| r.status().is_success()).unwrap_or(false);
        peer_results.push(json!({ "name": peer.name, "healthy": ok, "url": peer.admin_url }));
    }

    // Scheduler check
    let scheduler_check = {
        let path = crate::config::config_dir()
            .map(|d| d.join("bonsai").join("scheduled_tasks.json"))
            .unwrap_or_else(|| std::path::PathBuf::from("scheduled_tasks.json"));
        let (count, exists) = if path.exists() {
            let n = std::fs::read_to_string(&path).ok()
                .and_then(|s| serde_json::from_str::<Vec<Value>>(&s).ok())
                .map(|v| v.iter().filter(|t| t["enabled"].as_bool().unwrap_or(false)).count())
                .unwrap_or(0);
            (n, true)
        } else { (0, false) };
        json!({ "healthy": exists, "enabled_tasks": count })
    };

    // Aggregate
    let buddy_ok = buddy_check["healthy"].as_bool().unwrap_or(false);
    let llama_ok = llama_check["healthy"].as_bool().unwrap_or(false);
    let overall  = if buddy_ok && llama_ok { "healthy" } else if buddy_ok || llama_ok { "degraded" } else { "unhealthy" };

    Json(json!({
        "status": overall,
        "checks": {
            "buddy_api":    buddy_check,
            "llama_server": llama_check,
            "platforms":    Value::Object(platform_checks),
            "swarm_peers":  peer_results,
            "scheduler":    scheduler_check,
        }
    })).into_response()
}

// ── Audit log ─────────────────────────────────────────────────────────────────

use axum::response::Response;

async fn audit_log_handler(
    State(s): State<AdminState>,
    headers: HeaderMap,
) -> Response {
    if !check_auth(&headers, &s) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }
    let entries = crate::session::list_audit(&s.db, 200).await;
    let rows: Vec<_> = entries.iter().map(|e| json!({
        "id":       e.id,
        "ts":       e.ts,
        "event":    e.event,
        "platform": e.platform,
        "user_id":  e.user_id,
        "detail":   e.detail,
    })).collect();
    Json(json!({ "entries": rows })).into_response()
}

// ── Prometheus metrics ────────────────────────────────────────────────────────

async fn prometheus_handler(
    State(s): State<AdminState>,
    headers: HeaderMap,
) -> Response {
    if !check_auth(&headers, &s) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }
    let m = &s.metrics;
    use std::sync::atomic::Ordering::Relaxed;
    let body = format!(
        "# HELP bonsai_messages_inbound Total inbound messages\n\
         # TYPE bonsai_messages_inbound counter\n\
         bonsai_messages_inbound {}\n\
         # HELP bonsai_messages_processed Total processed messages\n\
         # TYPE bonsai_messages_processed counter\n\
         bonsai_messages_processed {}\n\
         # HELP bonsai_buddy_requests Total buddy API requests\n\
         # TYPE bonsai_buddy_requests counter\n\
         bonsai_buddy_requests {}\n\
         # HELP bonsai_buddy_errors Total buddy API errors\n\
         # TYPE bonsai_buddy_errors counter\n\
         bonsai_buddy_errors {}\n\
         # HELP bonsai_rate_limit_hits Total rate limit hits\n\
         # TYPE bonsai_rate_limit_hits counter\n\
         bonsai_rate_limit_hits {}\n\
         # HELP bonsai_dedup_hits Total dedup cache hits\n\
         # TYPE bonsai_dedup_hits counter\n\
         bonsai_dedup_hits {}\n\
         # HELP bonsai_allowlist_denials Total allowlist denials\n\
         # TYPE bonsai_allowlist_denials counter\n\
         bonsai_allowlist_denials {}\n\
         # HELP bonsai_confirms_resolved Total confirmations resolved\n\
         # TYPE bonsai_confirms_resolved counter\n\
         bonsai_confirms_resolved {}\n\
         # HELP bonsai_messages_queued_full Total messages dropped due to full queue\n\
         # TYPE bonsai_messages_queued_full counter\n\
         bonsai_messages_queued_full {}\n",
        m.messages_inbound.load(Relaxed),
        m.messages_processed.load(Relaxed),
        m.buddy_requests.load(Relaxed),
        m.buddy_errors.load(Relaxed),
        m.rate_limit_hits.load(Relaxed),
        m.dedup_hits.load(Relaxed),
        m.allowlist_denials.load(Relaxed),
        m.confirms_resolved.load(Relaxed),
        m.messages_queued_full.load(Relaxed),
    );
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4")],
        body,
    ).into_response()
}
