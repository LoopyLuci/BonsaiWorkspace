use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade, Message}, State},
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::uacs::{UacsState, UacsMode, HeadlessConfig, HITLConfig};

pub struct AppState {
    pub uacs: Arc<UacsState>,
}

pub async fn run_server(host: &str, port: u16) -> anyhow::Result<()> {
    let uacs = Arc::new(UacsState::new(UacsMode::Headless));
    let state = Arc::new(AppState { uacs });

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Universal Agent Control System listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

pub async fn run_uacs_visual(host: &str, port: u16, hitl: HITLConfig) -> anyhow::Result<()> {
    let uacs = Arc::new(UacsState::visual_with_hitl(hitl));
    let state = Arc::new(AppState { uacs });

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Universal Agent Control System (Visual with HITL) listening on {}", addr);
    tracing::info!("Dashboard: http://{}:{}", host, port);
    axum::serve(listener, app).await?;
    Ok(())
}

pub async fn run_uacs_headless(host: &str, port: u16, config: HeadlessConfig, hitl: HITLConfig) -> anyhow::Result<()> {
    let uacs = Arc::new(UacsState::headless_with_config(config, hitl));
    let state = Arc::new(AppState { uacs });

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Universal Agent Control System (Headless with HITL) listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/mcp", post(handle_mcp_request))
        .route("/ws/events", get(ws_events_handler))
        .with_state(state)
}

async fn root() -> &'static str {
    "Universal Agent Control System (UACS)"
}

async fn health() -> &'static str {
    "OK"
}

async fn ws_events_handler(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let uacs = state.uacs.clone();
    ws.on_upgrade(move |socket| handle_websocket(socket, uacs))
}

async fn handle_websocket(mut socket: WebSocket, uacs: Arc<UacsState>) {
    let mut rx = uacs.event_tx.subscribe();
    while let Ok(event) = rx.recv().await {
        let msg = match serde_json::to_string(&event) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if socket.send(Message::Text(msg)).await.is_err() {
            break;
        }
    }
}

async fn handle_mcp_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let method = body["method"].as_str().unwrap_or("");
    let params = body.get("params").cloned().unwrap_or(serde_json::json!({}));
    let id = body.get("id").cloned().unwrap_or(serde_json::json!(null));

    let token = match crate::auth::extract_token(&headers) {
        Ok(t) => t,
        Err(e) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": e.to_string()}))
            ));
        }
    };

    match method {
        "initialize" => Ok(Json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "Universal Agent Control System", "version": "1.0.0"}
            }
        }))),
        "tools/list" => {
            let tools_json: Vec<serde_json::Value> = crate::tools::list_tools()
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "inputSchema": t.input_schema,
                    })
                })
                .collect();
            Ok(Json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {"tools": tools_json}
            })))
        }
        "tools/call" => {
            let tool_name = params["name"].as_str().unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));

            match state.uacs.handle_tool_call(tool_name, arguments, &token.token).await {
                Ok(result) => Ok(Json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {"content": [{"type": "text", "text": result.to_string()}]}
                }))),
                Err(e) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {"code": -32000, "message": e}
                    }))
                )),
            }
        }
        _ => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": "Method not found"}
            }))
        )),
    }
}
