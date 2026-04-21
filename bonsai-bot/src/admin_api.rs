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
use tower_http::cors::{Any, CorsLayer};
use sha2::Digest;
use std::path::PathBuf;
use std::fs::OpenOptions;
use hex;
use chrono;

use crate::config::keyring_set;
use crate::metrics::SharedMetrics;
use crate::session::Db;

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
    /// Spawned runtime child processes keyed by generated id.
    pub runtime_children: Arc<Mutex<HashMap<String, tokio::process::Child>>>,
}

pub struct AdminHandle {
    shutdown_tx: Option<oneshot::Sender<()>>,
    join:        JoinHandle<()>,
    pub port:    u16,
}

impl AdminHandle {
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
        runtime_children: Arc::new(Mutex::new(HashMap::new())),
    };

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
    let s = serde_json::to_string(&entry).map_err(|e| e.to_string())?;
    let mut f = OpenOptions::new().create(true).append(true).open(&target_path).map_err(|e| e.to_string())?;
    use std::io::Write;
    f.write_all(s.as_bytes()).map_err(|e| e.to_string())?;
    f.write_all(b"\n").map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(serde::Deserialize)]
struct StartRuntimeRequest {
    kind: String,
    script: String,
    port: Option<u16>,
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

    let id = uuid::Uuid::new_v4().to_string();
    let rm = bonsai_runtime::RuntimeManager::new();
    let child_res = match body.kind.as_str() {
        "python" => {
            let port = body.port.unwrap_or(0);
            rm.start_python_worker(&body.script, port).await
        }
        "babashka" => rm.start_babashka_worker(&body.script).await,
        _ => return (StatusCode::BAD_REQUEST, Json(json!({"error": "unknown runtime kind"}))).into_response(),
    };

    match child_res {
        Ok(mut child) => {
            let pid = child.id().unwrap_or(0);
            // store child for lifecycle management
            {
                let mut map = s.runtime_children.lock().await;
                map.insert(id.clone(), child);
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
    if let Some(mut child) = map.remove(&body.id) {
        // attempt graceful kill
        if let Err(e) = child.kill().await {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("kill failed: {}", e)}))).into_response();
        }
        // wait for exit
        match child.wait().await {
            Ok(status) => Json(json!({"ok": true, "code": status.code()})).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
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
    for (id, child) in map.iter() {
        let pid = child.id().unwrap_or(0);
        out.push(json!({"id": id, "pid": pid}));
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
