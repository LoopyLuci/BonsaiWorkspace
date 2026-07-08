use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::auth::apikey::verify_api_key;
use crate::auth::capability::{extract_bearer_token, CapabilityToken};
use crate::protocol::grpc;
use crate::protocol::mcp::passthrough_jsonrpc;
use crate::protocol::rest::to_translated;
use crate::protocol::websocket::stream_events;
use crate::routing::discovery::list_peers;
use crate::routing::{route_to_service, BackendInstance};
use crate::telemetry::{ApiRequestEvent, TelemetryBus};
use crate::transfer_adapter::TransferDaemonAdapter;

const DEFAULT_RATE_LIMIT: usize = 100;
const RATE_WINDOW_SECS: u64 = 60;
const CIRCUIT_FAILURE_THRESHOLD: u32 = 5;
const CIRCUIT_OPEN_SECS: u64 = 15;

#[derive(Clone)]
pub struct BridgeState {
    pub client: reqwest::Client,
    pub telemetry: TelemetryBus,
    pub mcp_jsonrpc_url: String,
    pub rate_limits: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    pub circuits: Arc<Mutex<HashMap<String, CircuitState>>>,
    pub transfer: TransferDaemonAdapter,
}

#[derive(Debug, Clone)]
struct CircuitState {
    failures: u32,
    open_until: Option<Instant>,
}

impl CircuitState {
    fn new() -> Self {
        Self {
            failures: 0,
            open_until: None,
        }
    }

    fn is_open(&self) -> bool {
        if let Some(until) = self.open_until {
            return Instant::now() < until;
        }
        false
    }
}

pub async fn run(host: &str, port: u16, grpc_port: u16) -> Result<()> {
    let state = Arc::new(BridgeState {
        client: reqwest::Client::new(),
        telemetry: TelemetryBus::new(),
        mcp_jsonrpc_url: std::env::var("BONSAI_MCP_JSONRPC_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:11425/mcp".to_string()),
        rate_limits: Arc::new(Mutex::new(HashMap::new())),
        circuits: Arc::new(Mutex::new(HashMap::new())),
        transfer: TransferDaemonAdapter,
    });

    let grpc_state = state.clone();
    let grpc_host = host.to_string();
    tokio::spawn(async move {
        if let Err(e) = grpc::run_grpc_gateway(&grpc_host, grpc_port, grpc_state).await {
            tracing::error!("gRPC gateway failed: {e}");
        }
    });

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/openapi.json", get(openapi_json))
        .route("/api/v1/chat/completions", post(chat_completions))
        .route("/api/v1/inference", post(generic_route))
        .route("/api/v1/file/sync", post(generic_route))
        .route("/api/v1/blockchain/tx", post(generic_route))
        .route("/api/v1/remote/peers", get(remote_peers))
        .route("/mcp/jsonrpc", post(mcp_jsonrpc))
        .route("/ws/events", get(ws_events))
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    tracing::info!("bonsai-api-bridge listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

async fn healthz() -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok": true, "component": "bonsai-api-bridge"}))
}

async fn openapi_json() -> Json<serde_json::Value> {
    let yaml = include_str!("../openapi/api-spec.yaml");
    match serde_yaml::from_str::<serde_json::Value>(yaml) {
        Ok(v) => Json(v),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn ws_events(ws: WebSocketUpgrade, State(state): State<Arc<BridgeState>>) -> Response {
    ws.on_upgrade(move |socket| async move {
        let rx = state.telemetry.subscribe();
        stream_events(socket, rx).await;
    })
}

async fn mcp_jsonrpc(
    State(state): State<Arc<BridgeState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> Response {
    let started = Instant::now();

    let auth = authorize(&headers, "ApiCap:mcp");
    if let Err((status, body)) = auth {
        emit_event(
            &state,
            "POST",
            "/mcp/jsonrpc",
            addr.ip().to_string(),
            status.as_u16(),
            started,
            "ApiCap:mcp",
            None,
        );
        return (status, Json(body)).into_response();
    }

    match passthrough_jsonrpc(&state.mcp_jsonrpc_url, payload).await {
        Ok(v) => {
            emit_event(
                &state,
                "POST",
                "/mcp/jsonrpc",
                addr.ip().to_string(),
                200,
                started,
                "ApiCap:mcp",
                None,
            );
            Json(v).into_response()
        }
        Err(e) => {
            emit_event(
                &state,
                "POST",
                "/mcp/jsonrpc",
                addr.ip().to_string(),
                502,
                started,
                "ApiCap:mcp",
                None,
            );
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

async fn remote_peers(
    State(state): State<Arc<BridgeState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    let started = Instant::now();

    let auth = authorize(&headers, "ApiCap:discovery");
    if let Err((status, body)) = auth {
        emit_event(
            &state,
            "GET",
            "/api/v1/remote/peers",
            addr.ip().to_string(),
            status.as_u16(),
            started,
            "ApiCap:discovery",
            None,
        );
        return (status, Json(body)).into_response();
    }

    let peers = list_peers().await;
    emit_event(
        &state,
        "GET",
        "/api/v1/remote/peers",
        addr.ip().to_string(),
        200,
        started,
        "ApiCap:discovery",
        None,
    );
    Json(serde_json::json!({"peers": peers})).into_response()
}

async fn chat_completions(
    State(state): State<Arc<BridgeState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> Response {
    proxy_route(
        state,
        "POST",
        "/api/v1/chat/completions",
        addr.ip().to_string(),
        headers,
        payload,
    )
    .await
}

async fn generic_route(
    State(state): State<Arc<BridgeState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    uri: axum::http::Uri,
    Json(payload): Json<serde_json::Value>,
) -> Response {
    let path = uri.path();
    proxy_route(
        state,
        "POST",
        path,
        addr.ip().to_string(),
        headers,
        payload,
    )
    .await
}

async fn proxy_route(
    state: Arc<BridgeState>,
    method: &str,
    path: &str,
    client_ip: String,
    headers: HeaderMap,
    payload: serde_json::Value,
) -> Response {
    let started = Instant::now();
    let trace_id = Uuid::new_v4().to_string();

    let translated = match to_translated(path, payload.clone(), trace_id.clone()) {
        Some(v) => v,
        None => {
            emit_event(
                &state,
                method,
                path,
                client_ip,
                404,
                started,
                "ApiCap:unknown",
                None,
            );
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "unknown route"})),
            )
                .into_response();
        }
    };

    let token = match authorize(&headers, &translated.required_capability) {
        Ok(t) => t,
        Err((status, body)) => {
            emit_event(
                &state,
                method,
                path,
                client_ip,
                status.as_u16(),
                started,
                &translated.required_capability,
                None,
            );
            return (status, Json(body)).into_response();
        }
    };

    if let Err(status) = check_rate_limit(&state, token.subject.clone().unwrap_or_else(|| "anonymous".to_string())).await {
        emit_event(
            &state,
            method,
            path,
            client_ip,
            status.as_u16(),
            started,
            &translated.required_capability,
            None,
        );
        return (
            status,
            Json(serde_json::json!({"error": "rate limit exceeded"})),
        )
            .into_response();
    }

    let instances = route_to_service(&translated);
    let Some(best) = crate::routing::load_balancer::select_best(&instances) else {
        emit_event(
            &state,
            method,
            path,
            client_ip,
            503,
            started,
            &translated.required_capability,
            None,
        );
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "no backend instances available"})),
        )
            .into_response();
    };

    if is_circuit_open(&state, &best).await {
        emit_event(
            &state,
            method,
            path,
            client_ip,
            503,
            started,
            &translated.required_capability,
            Some(best.url.clone()),
        );
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "backend circuit open"})),
        )
            .into_response();
    }

    match forward_to_backend(&state, &best, &translated.trace_id, &payload).await {
        Ok(response) => {
            record_success(&state, &best).await;
            emit_event(
                &state,
                method,
                path,
                client_ip,
                200,
                started,
                &translated.required_capability,
                Some(best.url.clone()),
            );
            Json(response).into_response()
        }
        Err(e) => {
            record_failure(&state, &best).await;
            emit_event(
                &state,
                method,
                path,
                client_ip,
                502,
                started,
                &translated.required_capability,
                Some(best.url.clone()),
            );
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

fn authorize(
    headers: &HeaderMap,
    required_capability: &str,
) -> std::result::Result<CapabilityToken, (StatusCode, serde_json::Value)> {
    match extract_bearer_token(headers) {
        Ok(token) => {
            if token.has_capability(required_capability) {
                Ok(token)
            } else {
                Err((
                    StatusCode::FORBIDDEN,
                    serde_json::json!({"error": format!("missing capability {required_capability}")}),
                ))
            }
        }
        Err(_) => {
            if verify_api_key(headers).is_ok() {
                Ok(CapabilityToken {
                    subject: Some("apikey".to_string()),
                    capabilities: vec!["ApiCap:*".to_string()],
                    exp: None,
                    sig: None,
                })
            } else {
                Err((
                    StatusCode::UNAUTHORIZED,
                    serde_json::json!({"error": "unauthorized"}),
                ))
            }
        }
    }
}

async fn check_rate_limit(state: &BridgeState, subject: String) -> std::result::Result<(), StatusCode> {
    let mut guard = state.rate_limits.lock().await;
    let now = Instant::now();
    let window = Duration::from_secs(RATE_WINDOW_SECS);
    let bucket = guard.entry(subject).or_default();

    bucket.retain(|ts| now.duration_since(*ts) <= window);
    if bucket.len() >= DEFAULT_RATE_LIMIT {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    bucket.push(now);
    Ok(())
}

async fn is_circuit_open(state: &BridgeState, backend: &BackendInstance) -> bool {
    let mut guard = state.circuits.lock().await;
    let key = backend.url.clone();
    let entry = guard.entry(key).or_insert_with(CircuitState::new);
    entry.is_open()
}

async fn record_success(state: &BridgeState, backend: &BackendInstance) {
    let mut guard = state.circuits.lock().await;
    let key = backend.url.clone();
    let entry = guard.entry(key).or_insert_with(CircuitState::new);
    entry.failures = 0;
    entry.open_until = None;
}

async fn record_failure(state: &BridgeState, backend: &BackendInstance) {
    let mut guard = state.circuits.lock().await;
    let key = backend.url.clone();
    let entry = guard.entry(key).or_insert_with(CircuitState::new);
    entry.failures = entry.failures.saturating_add(1);
    if entry.failures >= CIRCUIT_FAILURE_THRESHOLD {
        entry.open_until = Some(Instant::now() + Duration::from_secs(CIRCUIT_OPEN_SECS));
    }
}

async fn forward_to_backend(
    state: &BridgeState,
    backend: &BackendInstance,
    trace_id: &str,
    payload: &serde_json::Value,
) -> Result<serde_json::Value> {
    if backend.url.starts_with("memory://discovery") {
        let peers = list_peers().await;
        return Ok(serde_json::json!({"peers": peers}));
    }

    if let Some(stripped) = backend.url.strip_prefix("td://") {
        let mut parts = stripped.splitn(2, '/');
        let peer_id = parts.next().unwrap_or_default();
        let service = parts.next().unwrap_or_default();
        return state
            .transfer
            .call_remote_backend(peer_id, service, trace_id, payload)
            .await;
    }

    let response = state
        .client
        .post(&backend.url)
        .json(payload)
        .send()
        .await
        .map_err(|e| anyhow!("backend send failed: {e}"))?
        .error_for_status()
        .map_err(|e| anyhow!("backend response status error: {e}"))?;

    response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| anyhow!("failed to decode backend json: {e}"))
}

fn emit_event(
    state: &BridgeState,
    method: &str,
    path: &str,
    client_ip: String,
    status_code: u16,
    started: Instant,
    capability: &str,
    peer_id: Option<String>,
) {
    let event = ApiRequestEvent {
        request_id: Uuid::new_v4(),
        method: method.to_string(),
        path: path.to_string(),
        client_ip,
        peer_id,
        status_code,
        duration_ms: started.elapsed().as_millis() as u64,
        capability_used: capability.to_string(),
    };
    state.telemetry.emit_api_event(event);
}

pub async fn dispatch_grpc_request(
    state: Arc<BridgeState>,
    path: &str,
    payload: serde_json::Value,
    token: CapabilityToken,
) -> Result<serde_json::Value> {
    let translated = to_translated(path, payload.clone(), Uuid::new_v4().to_string())
        .ok_or_else(|| anyhow!("unknown route {path}"))?;

    if !token.has_capability(&translated.required_capability) {
        return Err(anyhow!("missing capability {}", translated.required_capability));
    }

    check_rate_limit(&state, token.subject.unwrap_or_else(|| "grpc".to_string()))
        .await
        .map_err(|_| anyhow!("rate limit exceeded"))?;

    let instances = route_to_service(&translated);
    let backend = crate::routing::load_balancer::select_best(&instances)
        .ok_or_else(|| anyhow!("no backend instances available"))?;

    if is_circuit_open(&state, &backend).await {
        return Err(anyhow!("backend circuit open"));
    }

    match forward_to_backend(&state, &backend, &translated.trace_id, &translated.payload).await {
        Ok(v) => {
            record_success(&state, &backend).await;
            Ok(v)
        }
        Err(e) => {
            record_failure(&state, &backend).await;
            Err(e)
        }
    }
}
