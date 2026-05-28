//! bonsai-daemon — headless Bonsai service.
//!
//! Starts a WebSocket JSON-RPC server on a dynamic local port. Writes the
//! assigned port and a one-time auth token to the platform data directory so
//! external clients (VSCode extension, CLI tools) can connect without any
//! manual configuration.
//!
//! # Protocol
//! All messages are JSON objects. Clients must authenticate first:
//!   → `{"type":"auth","token":"<token>"}`
//!   ← `{"type":"auth_ok"}` or `{"type":"auth_fail","reason":"..."}`
//!
//! After auth, JSON-RPC 2.0 style:
//!   → `{"jsonrpc":"2.0","id":1,"method":"identity.get","params":{}}`
//!   ← `{"jsonrpc":"2.0","id":1,"result":{...}}`  or `{"jsonrpc":"2.0","id":1,"error":{...}}`
//!
//! # Port / Token files
//! Written to `{data_dir}/bonsai/` on startup:
//!   `daemon_port`   — decimal TCP port
//!   `daemon_token`  — 64-char hex token

mod rpc;
mod state;
mod panic_hook;
mod health_monitor;
mod checkpoint_impl;
pub mod binary_swap;

use std::sync::Arc;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State as AxumState,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use rand::RngCore;
use tokio::net::TcpListener;
use tracing::{info, warn};

use bonsai_actors::supervisor::{Supervisor, ChildSpec};
use bonsai_cas::CasStore;
use bonsai_skills;
use bonsai_skill_compiler;
use state::DaemonState;

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("bonsai_daemon=info".parse()?))
        .init();

    panic_hook::install_panic_hook();

    // Generate random auth token
    let mut token_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut token_bytes);
    let token = hex::encode(token_bytes);

    // Write port + token to the platform data dir
    let base = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("bonsai");
    tokio::fs::create_dir_all(&base).await?;

    // Open CAS store for checkpointing
    let cas = Arc::new(
        CasStore::open(
            &base.join("cas.db"),
            &base.join("cas-blobs"),
        ).await?
    );

    // Build shared daemon state
    let daemon_state = Arc::new(DaemonState::new(token.clone()));

    // Load initial skills from compiled_skills_dir() and start the file watcher.
    // The watcher is kept alive by binding it to `_skills_watcher`.
    let skills_dir = bonsai_skill_compiler::compiled_skills_dir();
    tokio::fs::create_dir_all(&skills_dir).await?;
    let _ = bonsai_skills::load_initial(&daemon_state.tools, &skills_dir);
    let _skills_watcher = bonsai_skills::watch_skills_dir(daemon_state.tools.clone(), &skills_dir)
        .inspect_err(|e| tracing::warn!("skills watcher unavailable: {e}"))
        .ok();

    // Spawn health monitor under a one-for-one supervisor
    {
        let state_clone = daemon_state.clone();
        let cas_clone   = cas.clone();
        let specs = vec![
            ChildSpec::new("health-monitor", move || {
                let s = state_clone.clone();
                let c = cas_clone.clone();
                async move { health_monitor::run_health_monitor(s, c).await }
            }),
        ];
        tokio::spawn(Supervisor::run(specs));
    }

    // Bind to a random available local port
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    tokio::fs::write(base.join("daemon_port"),  port.to_string()).await?;
    tokio::fs::write(base.join("daemon_token"), &token).await?;

    info!("bonsai-daemon listening on 127.0.0.1:{port}");
    println!("BONSAI_DAEMON_READY port={port}");

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(daemon_state);

    axum::serve(listener, app).await?;
    Ok(())
}

// ── WebSocket upgrade ─────────────────────────────────────────────────────────

async fn ws_handler(
    ws: WebSocketUpgrade,
    AxumState(state): AxumState<Arc<DaemonState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<DaemonState>) {
    let mut authenticated = false;

    while let Some(msg_result) = socket.recv().await {
        let msg = match msg_result {
            Ok(m) => m,
            Err(e) => { warn!("ws recv error: {e}"); break; }
        };

        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            _ => continue,
        };

        let req: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // ── Authentication handshake ──────────────────────────────────────────
        if req.get("type").and_then(|v| v.as_str()) == Some("auth") {
            let provided = req["token"].as_str().unwrap_or("");
            if provided == state.token {
                authenticated = true;
                let _ = socket.send(Message::Text(
                    r#"{"type":"auth_ok"}"#.to_string().into()
                )).await;
            } else {
                let _ = socket.send(Message::Text(
                    r#"{"type":"auth_fail","reason":"invalid token"}"#.to_string().into()
                )).await;
            }
            continue;
        }

        if !authenticated {
            let _ = socket.send(Message::Text(
                r#"{"type":"auth_fail","reason":"not authenticated"}"#.to_string().into()
            )).await;
            continue;
        }

        // ── JSON-RPC dispatch ─────────────────────────────────────────────────
        let id     = req.get("id").cloned().unwrap_or(serde_json::Value::Null);
        let method = req["method"].as_str().unwrap_or("").to_string();
        let params = req["params"].clone();

        let result = rpc::dispatch(&method, &params, &state).await;

        let response = match result {
            Ok(val) => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": val,
            }),
            Err(err) => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32000, "message": err },
            }),
        };

        if socket.send(Message::Text(response.to_string().into())).await.is_err() {
            break;
        }
    }
}
