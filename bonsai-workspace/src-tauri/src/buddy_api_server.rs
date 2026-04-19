//! OpenAI-compatible HTTP API server for Bonsai Buddy (port 11420).
//!
//! Endpoints:
//!   GET  /health                → liveness + port info
//!   GET  /v1/models             → static model list
//!   POST /v1/chat/completions   → assistant turn, stream or non-stream
//!
//! Bind policy: loopback-only (127.0.0.1) by default.
//! Port conflict: tries preferred_port then +1 through +4.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response, Sse},
    response::sse::{Event, KeepAlive},
    routing::{get, post},
    Json, Router,
};
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::AppHandle;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout, Duration};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tower_http::cors::{Any, CorsLayer};

use crate::{
    assistant_audit_log::AuditLog,
    assistant_manager::run_assistant_turn,
    assistant_policy::{ConfirmationGate, PolicyEngine},
    assistant_store::{AssistantProfile, AssistantStore},
    model_orchestrator::ModelOrchestrator,
    secrets_store::SecretsStore,
};

const BUDDY_DEFAULT_SYSTEM: &str = "\
You are Bonsai Buddy, a helpful, friendly, and knowledgeable personal AI assistant. \
You can help with coding, writing, analysis, creative tasks, and general questions. \
Be concise and direct while remaining warm and supportive.";

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct BuddyApiState {
    pub orchestrator:      Arc<ModelOrchestrator>,
    pub assistant_store:   Arc<AssistantStore>,
    pub policy_engine:     Arc<PolicyEngine>,
    pub confirmation_gate: Arc<ConfirmationGate>,
    pub audit_log:         Arc<AuditLog>,
    pub secrets_store:     Arc<SecretsStore>,
    pub app_handle:        AppHandle,
    pub port:              u16,
}

// ── Handle ────────────────────────────────────────────────────────────────────

pub struct BuddyApiHandle {
    shutdown_tx: Option<oneshot::Sender<()>>,
    join:        JoinHandle<()>,
    pub port:    u16,
}

impl BuddyApiHandle {
    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        let _ = (&mut self.join).await;
    }
}

// ── Startup ───────────────────────────────────────────────────────────────────

pub async fn start(
    orchestrator:      Arc<ModelOrchestrator>,
    assistant_store:   Arc<AssistantStore>,
    policy_engine:     Arc<PolicyEngine>,
    confirmation_gate: Arc<ConfirmationGate>,
    audit_log:         Arc<AuditLog>,
    secrets_store:     Arc<SecretsStore>,
    app_handle:        AppHandle,
    preferred_port:    u16,
) -> Result<BuddyApiHandle, String> {
    // Try preferred_port then +1 … +4
    let mut bound = None;
    let mut last_err = String::new();
    for delta in 0u16..5 {
        let p = preferred_port.saturating_add(delta);
        match tokio::net::TcpListener::bind(format!("127.0.0.1:{p}")).await {
            Ok(l)  => { bound = Some((p, l)); break; }
            Err(e) => { last_err = e.to_string(); }
        }
    }

    let (port, listener) = bound.ok_or_else(|| {
        format!("[buddy-api] no port available near {preferred_port}: {last_err}")
    })?;

    let state = BuddyApiState {
        orchestrator, assistant_store, policy_engine, confirmation_gate,
        audit_log, secrets_store, app_handle, port,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health",              get(health))
        .route("/v1/models",           get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .layer(cors)
        .with_state(state);

    eprintln!("[buddy-api] Bonsai Buddy API listening on http://127.0.0.1:{port}");

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let join = tokio::spawn(async move {
        let server = axum::serve(listener, app).with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });
        if let Err(e) = server.await {
            eprintln!("[buddy-api] server error: {e}");
        }
        eprintln!("[buddy-api] stopped");
    });

    Ok(BuddyApiHandle { shutdown_tx: Some(shutdown_tx), join, port })
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn health(State(s): State<BuddyApiState>) -> impl IntoResponse {
    Json(json!({ "status": "ok", "port": s.port, "buddy": true }))
}

async fn list_models(State(s): State<BuddyApiState>) -> impl IntoResponse {
    let mut data = vec![json!({ "id": "bonsai-buddy", "object": "model", "owned_by": "bonsai" })];
    let models = s.orchestrator.list_models().await;
    data.extend(models.into_iter().map(|m| {
        json!({ "id": m.id, "object": "model", "owned_by": "bonsai" })
    }));
    Json(json!({ "object": "list", "data": data }))
}

#[derive(Deserialize)]
struct ChatRequest {
    #[serde(default)]
    model: Option<String>,
    messages: Vec<Value>,
    #[serde(default)]
    stream: bool,
    #[serde(default = "default_max_tokens")]
    max_tokens: u32,
}

fn default_max_tokens() -> u32 { 2048 }

async fn chat_completions(
    State(s): State<BuddyApiState>,
    Json(req): Json<ChatRequest>,
) -> Response {
    let req_id    = gen_id();
    let session   = format!("buddy-api-{req_id}");
    let cancel    = Arc::new(AtomicBool::new(false));
    let mut profile = load_active_profile(&s.assistant_store).await;
    let model_hint = resolve_model_hint(req.model.as_deref(), &profile, &s.orchestrator).await;

    if let Some(mid) = model_hint.clone() {
        profile.model_id = Some(mid);
    }

    // ── Structured confirmation response (bonsai-bot protocol) ───────────────
    // If the last message carries bonsai_ext.type == "confirm_response", resolve
    // the confirmation gate and return the result without a full inference turn.
    if let Some(last) = req.messages.last() {
        if last.get("bonsai_ext")
            .and_then(|e| e.get("type"))
            .and_then(|t| t.as_str()) == Some("confirm_response")
        {
            let ext      = &last["bonsai_ext"];
            let schema   = ext["schema"].as_u64().unwrap_or(0);
            let token    = ext["token"].as_str().unwrap_or_default();
            let approved = ext["approved"].as_bool().unwrap_or(false);

            if schema != 1 || token.is_empty() {
                return (StatusCode::BAD_REQUEST, Json(json!({
                    "error": { "type": "confirm_invalid", "message": "Invalid confirm_response schema or missing token" }
                }))).into_response();
            }

            if !approved {
                s.confirmation_gate.cancel(token);
                return Json(json!({
                    "id": format!("buddy-{req_id}"),
                    "object": "chat.completion",
                    "choices": [{
                        "index": 0,
                        "message": { "role": "assistant", "content": "Confirmation denied. No action was taken." },
                        "finish_reason": "stop"
                    }]
                })).into_response();
            }

            match s.confirmation_gate.consume(token) {
                Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({
                    "error": { "type": "confirm_expired", "message": e }
                }))).into_response(),
                Ok((_tool, _args)) => {
                    // Tool is now approved — the caller should resubmit the full conversation
                    // without the bonsai_ext field; run_assistant_turn will re-invoke the tool
                    // via the Allow path. Return a receipt so the bot knows to resubmit.
                    return Json(json!({
                        "id": format!("buddy-{req_id}"),
                        "object": "chat.completion",
                        "choices": [{
                            "index": 0,
                            "message": { "role": "assistant", "content": "✅ Confirmed. Processing..." },
                            "finish_reason": "stop"
                        }],
                        "bonsai_ext": { "schema": 1, "type": "confirm_ack", "token": token }
                    })).into_response();
                }
            }
        }
    }

    if ensure_active_slot_url(&s.orchestrator, model_hint.as_deref()).await.is_none() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": {
                    "type": "buddy_error",
                    "message": "No model slot ready. Load a model in Bonsai Workspace and retry.",
                    "code": 503
                }
            })),
        ).into_response();
    }

    if req.stream {
        let (tx, rx) = mpsc::unbounded_channel::<String>();
        let history  = req.messages.clone();
        let s2       = s.clone();
        let p2       = profile.clone();
        let c2       = cancel.clone();
        let sid      = session.clone();
        let rid      = req_id.clone();

        tokio::spawn(async move {
            let err_tx = tx.clone();
            if let Err(e) = run_assistant_turn(
                history, &p2,
                &s2.assistant_store, &s2.policy_engine, &s2.confirmation_gate,
                &s2.orchestrator, &s2.secrets_store, &s2.audit_log,
                &s2.app_handle, c2, Some(tx), &sid,
            ).await {
                // Signal error through the SSE stream so the client knows why it closed.
                let _ = err_tx.send(serde_json::json!({
                    "error": { "type": "buddy_error", "message": e }
                }).to_string());
            }
        });

        let stream = UnboundedReceiverStream::new(rx).map(move |tok| {
            let chunk = json!({
                "id": format!("buddy-{rid}"),
                "object": "chat.completion.chunk",
                "choices": [{"delta": {"content": tok}, "index": 0, "finish_reason": null}]
            });
            Ok::<Event, std::convert::Infallible>(Event::default().data(chunk.to_string()))
        });

        Sse::new(stream).keep_alive(KeepAlive::default()).into_response()
    } else {
        match run_assistant_turn(
            req.messages, &profile,
            &s.assistant_store, &s.policy_engine, &s.confirmation_gate,
            &s.orchestrator, &s.secrets_store, &s.audit_log,
            &s.app_handle, cancel, None, &session,
        ).await {
            Ok(turn) => {
                let (finish_reason, bonsai_ext) = if let Some(cr) = &turn.confirm_token {
                    (
                        "tool_calls_pending_approval",
                        Some(json!({
                            "schema": 1,
                            "type": "confirm_required",
                            "token": cr.token,
                            "tool": cr.tool,
                            "args": cr.args,
                            "prompt": cr.prompt,
                            "expires_at": cr.expires_at,
                        })),
                    )
                } else {
                    ("stop", None)
                };
                let mut resp = json!({
                    "id": format!("buddy-{req_id}"),
                    "object": "chat.completion",
                    "choices": [{
                        "index": 0,
                        "message": { "role": "assistant", "content": turn.reply },
                        "finish_reason": finish_reason
                    }],
                    "usage": { "prompt_tokens": 0, "completion_tokens": 0 }
                });
                if let Some(ext) = bonsai_ext {
                    resp["bonsai_ext"] = ext;
                }
                Json(resp).into_response()
            }

            Err(e) => (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
                "error": { "type": "buddy_error", "message": e, "code": 503 }
            }))).into_response(),
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn load_active_profile(store: &AssistantStore) -> AssistantProfile {
    store.list_profiles().await
        .unwrap_or_default()
        .into_iter()
        .find(|p| p.is_active)
        .map(|mut p| {
            if p.system_prompt.trim().is_empty() {
                p.system_prompt = BUDDY_DEFAULT_SYSTEM.to_string();
            }
            p
        })
        .unwrap_or_else(default_profile)
}

async fn resolve_model_hint(
    requested_model: Option<&str>,
    profile: &AssistantProfile,
    orchestrator: &ModelOrchestrator,
) -> Option<String> {
    if let Some(model) = normalize_requested_model(requested_model) {
        return Some(model);
    }
    if let Some(model) = profile.model_id.clone().filter(|m| !m.trim().is_empty()) {
        return Some(model);
    }
    orchestrator.list_models().await.first().map(|m| m.id.clone())
}

fn normalize_requested_model(raw: Option<&str>) -> Option<String> {
    let model = raw?.trim();
    if model.is_empty() || model.eq_ignore_ascii_case("local") || model.eq_ignore_ascii_case("bonsai-buddy") {
        None
    } else {
        Some(model.to_string())
    }
}

async fn ensure_active_slot_url(
    orchestrator: &ModelOrchestrator,
    model_hint: Option<&str>,
) -> Option<String> {
    if let Some(url) = first_ready_slot_url(orchestrator, model_hint).await {
        return Some(url);
    }

    if let Some(model_id) = model_hint {
        let _ = timeout(Duration::from_secs(45), orchestrator.load(model_id.to_string())).await;
    }

    wait_for_active_slot_url(orchestrator, model_hint, 80, Duration::from_millis(200)).await
}

async fn wait_for_active_slot_url(
    orchestrator: &ModelOrchestrator,
    model_hint: Option<&str>,
    attempts: usize,
    delay: Duration,
) -> Option<String> {
    for _ in 0..attempts {
        if let Some(url) = first_ready_slot_url(orchestrator, model_hint).await {
            return Some(url);
        }
        sleep(delay).await;
    }
    None
}

async fn first_ready_slot_url(orchestrator: &ModelOrchestrator, model_hint: Option<&str>) -> Option<String> {
    let status = orchestrator.status().await;
    for slot in status.slots {
        if !slot.state.is_ready() {
            continue;
        }
        if let Some(mid) = model_hint {
            if slot.state.model_id() != Some(mid) {
                continue;
            }
        }
        return Some(format!("http://127.0.0.1:{}", slot.port));
    }
    None
}

fn default_profile() -> AssistantProfile {
    AssistantProfile {
        id:               "buddy-default".to_string(),
        name:             "Bonsai Buddy".to_string(),
        persona_id:       None,
        avatar_id:        None,
        tts_voice:        "en-us".to_string(),
        tts_speed:        1.0,
        tts_pitch:        1.0,
        tts_enabled:      false,
        wake_word:        None,
        tool_permissions: "{}".to_string(),
        system_prompt:    BUDDY_DEFAULT_SYSTEM.to_string(),
        model_id:         None,
        is_active:        true,
        created_at:       0,
        updated_at:       0,
    }
}

fn gen_id() -> String {
    use rand::Rng;
    let b: [u8; 8] = rand::thread_rng().gen();
    b.iter().map(|x| format!("{x:02x}")).collect()
}
