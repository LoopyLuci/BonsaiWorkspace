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
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tower_http::cors::{Any, CorsLayer};

use crate::config::keyring_set;
use crate::metrics::SharedMetrics;

/// Per-platform connection state, written by platform adapters, read by /status.
pub type PlatformStates = Arc<DashMap<String, String>>;

#[derive(Clone)]
pub struct AdminState {
    pub metrics:         SharedMetrics,
    pub platform_states: PlatformStates,
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
    admin_token:     String,
) -> Result<AdminHandle, String> {
    let mut bound = None;
    for delta in 0u16..5 {
        let p = preferred_port.saturating_add(delta);
        if let Ok(l) = tokio::net::TcpListener::bind(format!("127.0.0.1:{p}")).await {
            bound = Some((p, l));
            break;
        }
    }

    let (port, listener) = bound.ok_or("no admin port available near 11421")?;

    let state = AdminState {
        metrics,
        platform_states,
        admin_token: Arc::new(RwLock::new(admin_token)),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health",                    get(health))
        .route("/status",                    get(status))
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
