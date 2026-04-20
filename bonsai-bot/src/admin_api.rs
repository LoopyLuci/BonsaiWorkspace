use std::sync::{Arc, RwLock};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use dashmap::DashMap;
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot};
use std::process::Command;
use std::time::Duration;
use tokio::task::JoinHandle;
use tower_http::cors::{Any, CorsLayer};

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
                        if !is_api_healthy("127.0.0.1", p).await {
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
                    if !is_api_healthy("127.0.0.1", p).await {
                        let _ = std::fs::remove_file("bonsai-bot-port.json");
                        tracing::info!("[admin-api] removed stale local port file bonsai-bot-port.json (port={p})");
                    }
                }
            }
        }
    }

    let mut bound = None;
    for delta in 0u16..5 {
        let p = preferred_port.saturating_add(delta);
        match tokio::net::TcpListener::bind(format!("127.0.0.1:{p}")).await {
            Ok(l) => {
                bound = Some((p, l));
                break;
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::AddrInUse {
                    // If a healthy API is already bound here, skip this port.
                    if is_api_healthy("127.0.0.1", p).await {
                        tracing::info!("[admin-api] Port {p} in use by healthy API; skipping");
                        continue;
                    }

                    // Try to reclaim stale listeners on Windows by killing matching processes.
                    if try_reclaim_stale_listener(p) {
                        // give the OS a moment to release the socket
                        tokio::time::sleep(Duration::from_millis(300)).await;
                        if let Ok(l2) = tokio::net::TcpListener::bind(format!("127.0.0.1:{p}")).await {
                            bound = Some((p, l2));
                            break;
                        }
                    }
                    // otherwise continue to next candidate
                    continue;
                }
            }
        }
    }

    let (port, listener) = bound.ok_or_else(|| format!("no admin port available near {}", preferred_port))?;

    let state = AdminState {
        metrics,
        platform_states,
        db,
        broadcast_tx,
        admin_token: Arc::new(RwLock::new(admin_token)),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health",                    get(health))
        .route("/status",                    get(status))
        .route("/sessions",                  get(sessions_handler))
        .route("/broadcast",                 post(broadcast_handler))
        .route("/metrics",                   get(metrics_handler))
        .route("/config/reload",             post(config_reload))
        .route("/config/rotate-admin-token", post(rotate_token))
        .layer(cors)
        .with_state(state);

    tracing::info!("[admin-api] Listening on http://127.0.0.1:{port}");

    // Persist the chosen admin port so other local tooling can discover it.
    // Write to the standard config dir (if available) as `bonsai-bot-port.json`.
    if let Some(dir) = crate::config::config_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("bonsai-bot-port.json");
        let _ = std::fs::write(&path, serde_json::to_string(&json!({ "port": port })).unwrap_or_default());
    } else {
        let _ = std::fs::write("bonsai-bot-port.json", serde_json::to_string(&json!({ "port": port })).unwrap_or_default());
    }

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

async fn is_api_healthy(host: &str, port: u16) -> bool {
    let url = format!("http://{host}:{port}/health");
    match reqwest::Client::builder()
        .timeout(Duration::from_millis(1200))
        .build()
    {
        Ok(client) => client
            .get(url)
            .send()
            .await
            .is_ok_and(|r| r.status().is_success()),
        Err(_) => false,
    }
}

fn try_reclaim_stale_listener(port: u16) -> bool {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = port;
        false
    }

    #[cfg(target_os = "windows")]
    {
        let pids = listening_pids_on_port(port);
        if pids.is_empty() {
            return false;
        }

        let mut killed_any = false;
        for pid in pids {
            let image = process_image_name(pid);
            let img = image.to_ascii_lowercase();
            if img != "bonsai-bot.exe" && img != "bonsai-bot" {
                continue;
            }
            if let Ok(out) = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .output()
            {
                if out.status.success() {
                    killed_any = true;
                }
            }
        }
        killed_any
    }
}

#[cfg(target_os = "windows")]
fn listening_pids_on_port(port: u16) -> Vec<u32> {
    let out = match Command::new("netstat").args(["-ano"]).output() {
        Ok(o) => o,
        Err(_) => return vec![],
    };
    let dump = String::from_utf8_lossy(&out.stdout);
    let mut pids = std::collections::BTreeSet::new();
    let needle = format!(":{port}");

    for line in dump.lines() {
        let l = line.trim();
        if l.is_empty() || !l.contains(&needle) || !l.to_ascii_uppercase().contains("LISTEN") {
            continue;
        }
        let parts: Vec<&str> = l.split_whitespace().collect();
        if let Some(last) = parts.last() {
            if let Ok(pid) = last.parse::<u32>() {
                if pid > 0 {
                    pids.insert(pid);
                }
            }
        }
    }

    pids.into_iter().collect()
}

#[cfg(target_os = "windows")]
fn process_image_name(pid: u32) -> String {
    let out = match Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return String::new(),
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.trim();
    if line.is_empty() || line.contains("No tasks are running") {
        return String::new();
    }
    line.split(',')
        .next()
        .map(|s| s.trim_matches('"').to_string())
        .unwrap_or_default()
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
