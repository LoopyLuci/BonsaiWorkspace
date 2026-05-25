//! REST management API — `/api/v1/` route group.
//!
//! Exposes every major Bonsai capability over HTTP so that agents, CLI tools,
//! and automated scripts can drive the full ecosystem without needing a Tauri
//! webview or IPC channel.
//!
//! Auth: `Authorization: Bearer <pair_token>` (same token shown in Settings).
//!       Pass an empty string or omit the header when `pair_token` is blank
//!       (development / no-auth mode).
//!
//! Routes
//! ──────
//!   GET  /api/v1/models/list       list available GGUF models
//!   POST /api/v1/models/load       load a model by id
//!   POST /api/v1/models/switch     alias for load
//!   GET  /api/v1/queue/status      task-queue depth + slot states
//!   POST /api/v1/swarm/submit      full swarm chat (multi-agent)
//!   POST /api/v1/swarm/cancel      cancel an in-flight swarm run
//!   GET  /api/v1/swarm/metrics     recent swarm run records
//!   POST /api/v1/chat              single-agent chat (non-swarm)
//!   GET  /api/v1/agents/list       list registered agents
//!   POST /api/v1/agents/message    send a message to a named agent
//!   GET  /api/v1/features          read feature flags
//!   POST /api/v1/features          write feature flags
//!   POST /api/v1/tools/run         invoke a single tool by name + args
//!   POST /api/v1/render/block      render rich markdown block to SVG
//!   POST /api/v1/sandbox/run       run Python code in sandboxed venv
//!   POST /api/v1/images/generate   generate image via local SD model
//!   POST /api/v1/tts/speak         synthesize speech via Piper TTS

use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::Emitter;

use crate::agent::{AgentContext, AgentMessage};

// ── Shared state ──────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct MgmtState {
    pub orchestrator:    Arc<crate::model_orchestrator::ModelOrchestrator>,
    pub agent_host:      Arc<crate::agent_host::AgentHost>,
    pub agent_store:     Arc<crate::agent_store::AgentStore>,
    pub task_queue:      Arc<crate::task_queue::TaskQueue>,
    pub swarm_cancels:   Arc<std::sync::Mutex<std::collections::HashMap<String, Vec<Arc<std::sync::atomic::AtomicBool>>>>>,
    pub app_handle:      tauri::AppHandle,
    pub pair_token:      String,
    pub bonsai_core:     Arc<crate::bonsai_core::BonsaiCore>,
    pub telemetry:       Arc<crate::telemetry::TelemetryStore>,
    pub dual_session:    Arc<crate::dual_inference::SessionManager>,
    pub training_loop:   Arc<crate::training_loop::TrainingLoopState>,
    pub self_play:       Arc<crate::self_play::SelfPlayState>,
    pub plugin_host:     Arc<crate::plugin_host::PluginHost>,
    pub tool_registry:   Arc<crate::tool_registry::ToolRegistryState>,
}

// ── Auth helper ───────────────────────────────────────────────────────────────

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.trim())
}

fn authorized(state: &MgmtState, headers: &HeaderMap) -> bool {
    state.pair_token.is_empty() || bearer_token(headers) == Some(state.pair_token.as_str())
}

macro_rules! auth {
    ($state:expr, $headers:expr) => {
        if !authorized(&$state, &$headers) {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "invalid or missing Bearer token"})),
            )
                .into_response();
        }
    };
}

fn err500(e: impl std::fmt::Display) -> (StatusCode, Json<Value>) {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
}

fn err400(e: impl std::fmt::Display) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()})))
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router(state: MgmtState) -> Router {
    Router::new()
        .route("/api/v1/models/list",    get(mgmt_list_models))
        .route("/api/v1/models/load",    post(mgmt_load_model))
        .route("/api/v1/models/switch",  post(mgmt_load_model))  // alias
        .route("/api/v1/queue/status",   get(mgmt_queue_status))
        .route("/api/v1/swarm/submit",   post(mgmt_swarm_submit))
        .route("/api/v1/swarm/cancel",   post(mgmt_swarm_cancel))
        .route("/api/v1/swarm/metrics",  get(mgmt_swarm_metrics))
        .route("/api/v1/chat",           post(mgmt_chat))
        .route("/api/v1/agents/list",    get(mgmt_list_agents))
        .route("/api/v1/agents/message", post(mgmt_agent_message))
        .route("/api/v1/features",       get(mgmt_get_features).post(mgmt_set_features))
        .route("/api/v1/tools/run",      post(mgmt_run_tool))
        .route("/api/v1/core/stats",     get(mgmt_core_stats))
        .route("/api/v1/bonsai/process", post(mgmt_bonsai_process))
        .route("/api/v1/core/shadow",    post(mgmt_set_shadow))
        .route("/api/v1/curator/flush",         post(mgmt_curator_flush))
        .route("/api/v1/telemetry/training",    get(mgmt_telemetry_training))
        .route("/api/v1/telemetry/training/:id",get(mgmt_telemetry_training_run))
        .route("/api/v1/telemetry/inference",   get(mgmt_telemetry_inference))
        .route("/api/v1/telemetry/curated",     get(mgmt_telemetry_curated))
        // training control — mirrors Tauri commands for web clients
        .route("/api/v1/training/status",  get(mgmt_training_status))
        .route("/api/v1/training/history", get(mgmt_training_history))
        // dual model comparison — continuous training loop
        .route("/api/v1/compare",                  post(mgmt_compare_models))
        .route("/api/v1/training/loop/start",      post(mgmt_loop_start))
        .route("/api/v1/training/loop/stop",       post(mgmt_loop_stop))
        .route("/api/v1/training/loop/status",     get(mgmt_loop_status))
        // multi-modal — rich markdown, sandbox, image gen, TTS
        .route("/api/v1/render/block",    post(mgmt_render_block))
        .route("/api/v1/sandbox/run",     post(mgmt_sandbox_run))
        .route("/api/v1/images/generate", post(mgmt_image_generate))
        .route("/api/v1/tts/speak",       post(mgmt_tts_speak))
        // self-play training
        .route("/api/v1/training/self-play/start",  post(mgmt_self_play_start))
        .route("/api/v1/training/self-play/stop",   post(mgmt_self_play_stop))
        .route("/api/v1/training/self-play/status", get(mgmt_self_play_status))
        // plugin host
        .route("/api/v1/plugins/load",    post(mgmt_plugin_load))
        .route("/api/v1/plugins/list",    get(mgmt_plugin_list))
        .route("/api/v1/plugins/execute", post(mgmt_plugin_execute))
        // tool registry (augments existing /api/v1/tools/run)
        .route("/api/v1/tools/list",      get(mgmt_tools_list))
        .with_state(state)
}

// ── Models ────────────────────────────────────────────────────────────────────

async fn mgmt_list_models(
    State(s): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    auth!(s, headers);
    let models = s.orchestrator.list_models().await;
    Json(json!({ "models": models })).into_response()
}

#[derive(Deserialize)]
struct LoadModelBody {
    model_id: String,
}

async fn mgmt_load_model(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(body): Json<LoadModelBody>,
) -> impl IntoResponse {
    auth!(s, headers);
    let rx = s.orchestrator.load(body.model_id.clone());
    match rx.await {
        Err(_) => err500("orchestrator offline").into_response(),
        Ok(Err(e)) => err500(e).into_response(),
        Ok(Ok(())) => Json(json!({ "ok": true, "model_id": body.model_id })).into_response(),
    }
}

// ── Queue ─────────────────────────────────────────────────────────────────────

async fn mgmt_queue_status(
    State(s): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    auth!(s, headers);
    let status = s.task_queue.status().await;
    Json(serde_json::to_value(status).unwrap_or_default()).into_response()
}

// ── Swarm ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SwarmSubmitBody {
    /// Convenience: a single-turn user prompt. If provided, wraps into messages.
    #[serde(default)]
    prompt:         Option<String>,
    #[serde(default)]
    messages:       Vec<MgmtChatMessage>,
    workspace_path: Option<String>,
    enabled_tools:  Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Clone)]
struct MgmtChatMessage {
    role:    String,
    content: String,
}

async fn mgmt_swarm_submit(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(body): Json<SwarmSubmitBody>,
) -> impl IntoResponse {
    auth!(s, headers);

    if !crate::features::FeatureFlags::is_enabled("swarm") {
        return err400("Swarm feature is disabled").into_response();
    }

    // Resolve messages: accept either `prompt` shorthand or explicit `messages` array
    let messages = if !body.messages.is_empty() {
        body.messages.clone()
    } else if let Some(p) = &body.prompt {
        vec![MgmtChatMessage { role: "user".into(), content: p.clone() }]
    } else {
        return err400("Provide either `prompt` or `messages`").into_response();
    };

    let resolved = match s.agent_store.resolve_agents(&s.orchestrator).await {
        Ok(r) => r,
        Err(e) => return err500(e).into_response(),
    };

    let model_url = s.orchestrator.active_slot_url().await;
    let Some(model_url) = model_url else {
        return err400("No model slot is ready — load a model first").into_response();
    };

    // Build the prompt from messages
    let user_prompt = messages.iter().rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();

    let workspace = body.workspace_path.as_deref().unwrap_or(".");

    // Run swarm via the orchestrator — fire workers, collect results
    let run_id: String = {
        use rand::distributions::Alphanumeric;
        use rand::Rng;
        rand::thread_rng().sample_iter(&Alphanumeric).take(12).map(char::from).collect()
    };

    // Use configured DB agents if present; fall back to built-in analyst+critic roles
    // so the REST swarm always runs real workers even with no agents configured in the UI.
    let db_workers: Vec<_> = resolved.iter()
        .filter(|a| a.config.slot_index != 0 && a.config.enabled)
        .cloned()
        .collect();

    #[derive(Clone)]
    struct WorkerSpec { slot: i64, label: String, system: String }

    let worker_specs: Vec<WorkerSpec> = if db_workers.is_empty() {
        vec![
            WorkerSpec {
                slot: 1,
                label: "Analyst".into(),
                system: "You are an analytical expert. Carefully examine the request, identify key issues, strengths, and weaknesses, and provide a detailed technical analysis.".into(),
            },
            WorkerSpec {
                slot: 2,
                label: "Critic".into(),
                system: "You are a constructive critic. Look for edge cases, potential problems, missing considerations, and suggest concrete improvements with examples.".into(),
            },
        ]
    } else {
        db_workers.iter().map(|a| WorkerSpec {
            slot:   a.config.slot_index,
            label:  a.config.label.clone(),
            system: a.system_prompt.clone(),
        }).collect()
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap_or_default();

    let mut worker_outputs: Vec<Value> = Vec::new();

    for spec in &worker_specs {
        let worker_url = format!("{}/v1/chat/completions", model_url.trim_end_matches('/'));
        let req_body = json!({
            "messages": [
                {"role": "system", "content": &spec.system},
                {"role": "user",   "content": &user_prompt}
            ],
            "temperature": 0.4,
            "max_tokens": 1024,
            "stream": false
        });
        let resp = client.post(&worker_url).json(&req_body).send().await;
        let content = match resp {
            Ok(r) if r.status().is_success() => {
                r.json::<Value>().await
                    .ok()
                    .and_then(|v| v["choices"][0]["message"]["content"].as_str().map(str::to_owned))
                    .unwrap_or_else(|| "(worker error: empty response)".into())
            }
            Ok(r) => format!("(worker error: HTTP {})", r.status()),
            Err(e) => format!("(worker error: {e})"),
        };
        worker_outputs.push(json!({
            "slot":    spec.slot,
            "agent":   spec.label,
            "content": content,
        }));
    }

    // Synthesis: ask model to combine worker outputs
    let synthesis_ctx = worker_outputs.iter()
        .map(|w| format!("Worker {} ({}):\n{}", w["slot"], w["agent"], w["content"].as_str().unwrap_or("")))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    let synthesis_prompt = if worker_outputs.is_empty() {
        user_prompt.clone()
    } else {
        format!(
            "You are the synthesis leader. Multiple workers have responded to this request:\n\n\
             Original request: {user_prompt}\n\n\
             Worker responses:\n{synthesis_ctx}\n\n\
             Synthesise the best combined answer, resolving any contradictions and presenting a \
             single coherent response."
        )
    };

    let synthesis_url = format!("{}/v1/chat/completions", model_url.trim_end_matches('/'));
    let synthesis_req = json!({
        "messages": [
            {"role": "system", "content": "You are an expert synthesis leader."},
            {"role": "user",   "content": synthesis_prompt}
        ],
        "temperature": 0.3,
        "max_tokens": 2048,
        "stream": false
    });

    let final_content = match client.post(&synthesis_url).json(&synthesis_req).send().await {
        Ok(r) if r.status().is_success() => {
            r.json::<Value>().await
                .ok()
                .and_then(|v| v["choices"][0]["message"]["content"].as_str().map(str::to_owned))
                .unwrap_or_else(|| "(synthesis error: empty response)".into())
        }
        Ok(r) => format!("(synthesis error: HTTP {})", r.status()),
        Err(e) => format!("(synthesis error: {e})"),
    };

    // Emit event so Chat panel picks it up
    let _ = s.app_handle.emit("agent-output", json!({
        "content": &final_content,
        "actions": [],
    }));

    Json(json!({
        "run_id":         run_id,
        "final_content":  final_content,
        "worker_outputs": worker_outputs,
        "worker_count":   worker_specs.len(),
    })).into_response()
}

#[derive(Deserialize)]
struct SwarmCancelBody {
    run_id: String,
}

async fn mgmt_swarm_cancel(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(body): Json<SwarmCancelBody>,
) -> impl IntoResponse {
    auth!(s, headers);
    if !crate::features::FeatureFlags::is_enabled("swarm") {
        return err400("Swarm feature is disabled").into_response();
    }
    if let Ok(cancels) = s.swarm_cancels.lock() {
        if let Some(flags) = cancels.get(&body.run_id) {
            for f in flags { f.store(true, Ordering::Relaxed); }
        }
    }
    Json(json!({ "ok": true })).into_response()
}

async fn mgmt_swarm_metrics(
    State(s): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    auth!(s, headers);
    if !crate::features::FeatureFlags::is_enabled("swarm") {
        return err400("Swarm feature is disabled").into_response();
    }
    let records = crate::swarm_orchestrator::recent_swarm_runs();
    Json(json!({ "records": records })).into_response()
}

// ── Chat (single-agent) ───────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ChatBody {
    messages: Vec<MgmtChatMessage>,
}

async fn mgmt_chat(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(body): Json<ChatBody>,
) -> impl IntoResponse {
    auth!(s, headers);

    let model_url = s.orchestrator.active_slot_url().await;
    let Some(model_url) = model_url else {
        return err400("No model slot is ready — load a model first").into_response();
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap_or_default();

    let messages: Vec<Value> = body.messages.iter()
        .map(|m| json!({"role": m.role, "content": m.content}))
        .collect();

    let req_body = json!({
        "messages": messages,
        "temperature": 0.7,
        "max_tokens": 2048,
        "stream": false,
    });

    let url = format!("{}/v1/chat/completions", model_url.trim_end_matches('/'));
    match client.post(&url).json(&req_body).send().await {
        Err(e) => err500(format!("model unreachable: {e}")).into_response(),
        Ok(r) if !r.status().is_success() => {
            let status = r.status();
            let text = r.text().await.unwrap_or_default();
            (StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
             Json(json!({"error": text}))).into_response()
        }
        Ok(r) => {
            let v: Value = r.json().await.unwrap_or_default();
            let content = v["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_owned();
            let _ = s.app_handle.emit("agent-output", json!({
                "content": &content,
                "actions": [],
            }));
            Json(json!({ "content": content, "raw": v })).into_response()
        }
    }
}

// ── Agents ────────────────────────────────────────────────────────────────────

async fn mgmt_list_agents(
    State(s): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    auth!(s, headers);
    let agents = s.agent_host.list().await;
    Json(json!({ "agents": agents })).into_response()
}

#[derive(Deserialize)]
struct AgentMessageBody {
    #[serde(alias = "agentId")]
    agent_id: String,
    message:  AgentMessage,
}

async fn mgmt_agent_message(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(body): Json<AgentMessageBody>,
) -> impl IntoResponse {
    auth!(s, headers);
    let ctx = AgentContext {
        model_url: s.orchestrator.active_slot_url().await,
    };
    match s.agent_host.handle(&body.agent_id, ctx, body.message).await {
        Err(e) => err500(e).into_response(),
        Ok(output) => {
            let _ = s.app_handle.emit("agent-output", &output);
            Json(serde_json::to_value(&output).unwrap_or_default()).into_response()
        }
    }
}

// ── Features ──────────────────────────────────────────────────────────────────

async fn mgmt_get_features(
    State(s): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    auth!(s, headers);
    Json(serde_json::to_value(crate::features::FeatureFlags::global()).unwrap_or_default())
        .into_response()
}

async fn mgmt_set_features(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(patch): Json<Value>,
) -> impl IntoResponse {
    auth!(s, headers);
    let mut current = match serde_json::to_value(crate::features::FeatureFlags::global()) {
        Ok(v) => v,
        Err(e) => return err500(e).into_response(),
    };
    if let (Some(obj), Some(patch_obj)) = (current.as_object_mut(), patch.as_object()) {
        for (k, v) in patch_obj { obj.insert(k.clone(), v.clone()); }
    }
    match serde_json::from_value::<crate::features::FeatureFlags>(current) {
        Ok(flags) => { crate::features::FeatureFlags::set_global(flags); }
        Err(e) => return err400(e).into_response(),
    }
    Json(json!({ "ok": true })).into_response()
}

// ── Tools ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ToolRunBody {
    tool: String,
    /// Accept both `args` and `params` as field names
    #[serde(alias = "params")]
    args: Option<Value>,
    workspace: Option<String>,
}

async fn mgmt_run_tool(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(body): Json<ToolRunBody>,
) -> impl IntoResponse {
    auth!(s, headers);
    let workspace = body.workspace.clone();
    let args = body.args.clone().unwrap_or(Value::Object(Default::default()));
    match crate::tools::execute_built_in(&body.tool, &args, workspace.as_deref()).await {
        Ok(result) => Json(json!({ "ok": true, "result": result })).into_response(),
        Err(e)     => err400(e).into_response(),
    }
}

// ── Core stats ────────────────────────────────────────────────────────────────

async fn mgmt_core_stats(
    State(s): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    auth!(s, headers);
    let queue = s.task_queue.status().await;
    let memory_entries = s.bonsai_core.memory.count().await;
    let curator_buffered = s.bonsai_core.curator.buffered().await;
    let curator_total = s.bonsai_core.curator.total_seen().await;
    let adapter_loaded = s.bonsai_core.adapter_loaded().await;
    Json(json!({
        "adapter_loaded": adapter_loaded,
        "avg_latency_ms": 0.0,
        "fallback_rate": 0.0,
        "memory_entries": memory_entries,
        "queue_depth": queue.pending_total,
        "active_tasks": queue.active_total,
        "curator_buffered": curator_buffered,
        "curator_total_seen": curator_total,
    }))
    .into_response()
}

async fn mgmt_curator_flush(
    State(s): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    auth!(s, headers);
    s.bonsai_core.curator.flush().await;
    let total = s.bonsai_core.curator.total_seen().await;
    Json(json!({ "ok": true, "total_seen": total })).into_response()
}

// ── BonsaiCore process ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct BonsaiProcessBody {
    request: String,
    #[serde(default)]
    history: Vec<crate::bonsai_core::ChatMessage>,
}

async fn mgmt_bonsai_process(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(body): Json<BonsaiProcessBody>,
) -> impl IntoResponse {
    auth!(s, headers);
    match s.bonsai_core.process(&body.request, &body.history).await {
        Ok(resp) => Json(json!({ "ok": true, "result": resp })).into_response(),
        Err(e)   => err500(e).into_response(),
    }
}

// ── Shadow mode toggle ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ShadowBody {
    enabled: bool,
}

async fn mgmt_set_shadow(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(body): Json<ShadowBody>,
) -> impl IntoResponse {
    auth!(s, headers);
    s.bonsai_core.set_shadow_mode(body.enabled).await;
    Json(json!({ "shadow_mode": body.enabled })).into_response()
}

// ── Telemetry endpoints ───────────────────────────────────────────────────────

async fn mgmt_telemetry_training(
    State(s): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    auth!(s, headers);
    match s.telemetry.get_training_runs(50).await {
        Ok(runs) => Json(json!({ "runs": runs })).into_response(),
        Err(e)   => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn mgmt_telemetry_training_run(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    auth!(s, headers);
    match s.telemetry.get_training_run(&id).await {
        Ok(Some(run)) => Json(run).into_response(),
        Ok(None)      => (StatusCode::NOT_FOUND, "run not found").into_response(),
        Err(e)        => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn mgmt_telemetry_inference(
    State(s): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    auth!(s, headers);
    match s.telemetry.get_inference_stats(24).await {
        Ok(stats) => Json(stats).into_response(),
        Err(e)    => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn mgmt_telemetry_curated(
    State(s): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    auth!(s, headers);
    let path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".bonsai/curated_examples.jsonl");
    match std::fs::read_to_string(&path) {
        Ok(content) => (
            [(axum::http::header::CONTENT_TYPE, "application/x-ndjson")],
            content,
        ).into_response(),
        Err(_) => Json(json!({ "examples": [] })).into_response(),
    }
}

// ── Training control endpoints ────────────────────────────────────────────────

async fn mgmt_training_status(
    State(s): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    auth!(s, headers);
    match s.telemetry.get_latest_run().await {
        Ok(Some(run)) => Json(run).into_response(),
        Ok(None)      => Json(serde_json::json!({ "status": "idle" })).into_response(),
        Err(e)        => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn mgmt_training_history(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    uri: axum::http::Uri,
) -> impl IntoResponse {
    auth!(s, headers);
    let limit = uri.query()
        .and_then(|q| q.split('&').find(|p| p.starts_with("limit=")))
        .and_then(|p| p.trim_start_matches("limit=").parse::<i64>().ok())
        .unwrap_or(20);
    match s.telemetry.get_training_runs(limit).await {
        Ok(runs) => Json(runs).into_response(),
        Err(e)   => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ── Dual model comparison ─────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct CompareRequest {
    base_model: String,
    bonsai_adapter: String,
    prompt: String,
    gpu_layers: Option<u32>,
}

async fn mgmt_compare_models(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(body): Json<CompareRequest>,
) -> impl IntoResponse {
    auth!(s, headers);
    use crate::dual_inference::{DualModelSession, DualSessionConfig};

    let layers = body.gpu_layers.unwrap_or(35);

    let server = match s
        .dual_session
        .ensure_session(DualSessionConfig {
            base_model_path: body.base_model,
            bonsai_lora_path: Some(body.bonsai_adapter),
            reference_lora_path: None,
            gpu_layers: layers,
            context_size: 2048,
        })
        .await
    {
        Ok(srv) => srv,
        Err(e) => return err500(e).into_response(),
    };

    let session = DualModelSession::new(server, None);
    match session.compare(&body.prompt).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => err500(e).into_response(),
    }
}

// ── Continuous training loop ──────────────────────────────────────────────────

async fn mgmt_loop_start(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(config): Json<crate::training_loop::LoopConfig>,
) -> impl IntoResponse {
    auth!(s, headers);
    match s.training_loop.start(config).await {
        Ok(()) => Json(json!({ "ok": true, "status": "started" })).into_response(),
        Err(e) => err500(e).into_response(),
    }
}

async fn mgmt_loop_stop(
    State(s): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    auth!(s, headers);
    s.training_loop.stop().await;
    Json(json!({ "ok": true, "status": "stopped" })).into_response()
}

async fn mgmt_loop_status(
    State(s): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    auth!(s, headers);
    Json(s.training_loop.status().await).into_response()
}

// ── Rich markdown ─────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct RenderBlockBody {
    block_type: String,
    content: String,
    data: Option<Vec<crate::rich_markdown::ChartDataPoint>>,
}

async fn mgmt_render_block(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(body): Json<RenderBlockBody>,
) -> impl IntoResponse {
    auth!(s, headers);
    match crate::rich_markdown::render_rich_block(body.block_type, body.content, body.data) {
        Ok(svg) => Json(json!({ "svg": svg })).into_response(),
        Err(e)  => err400(e).into_response(),
    }
}

// ── Sandbox execution ─────────────────────────────────────────────────────────

async fn mgmt_sandbox_run(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(req): Json<crate::sandbox_executor::SandboxRequest>,
) -> impl IntoResponse {
    auth!(s, headers);
    match crate::sandbox_executor::run_sandboxed_code(req).await {
        Ok(result) => Json(result).into_response(),
        Err(e)     => err500(e).into_response(),
    }
}

// ── Image generation ──────────────────────────────────────────────────────────

async fn mgmt_image_generate(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(req): Json<crate::image_generation::ImageGenRequest>,
) -> impl IntoResponse {
    auth!(s, headers);
    match crate::image_generation::generate_image(req).await {
        Ok(result) => Json(result).into_response(),
        Err(e)     => err500(e).into_response(),
    }
}

// ── TTS ───────────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct TtsSpeakBody {
    text: String,
    voice: Option<String>,
}

async fn mgmt_tts_speak(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(body): Json<TtsSpeakBody>,
) -> impl IntoResponse {
    auth!(s, headers);
    match crate::tts_engine::synthesize_speech(&body.text, body.voice.as_deref()).await {
        Ok(result) => Json(result).into_response(),
        Err(e)     => err500(e).into_response(),
    }
}

// ── Self-play ─────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize, Default)]
struct SelfPlayStartBody {
    rounds: Option<usize>,
    temperature_high: Option<f32>,
    temperature_low: Option<f32>,
    overlap_threshold: Option<f32>,
}

async fn mgmt_self_play_start(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    body: Option<Json<SelfPlayStartBody>>,
) -> impl IntoResponse {
    auth!(s, headers);
    // Apply any overrides from the request body.
    // SelfPlayState is initialised with defaults; reconfigure before starting.
    let _ = body; // config override not yet exposed; start with defaults
    match s.self_play.start().await {
        Ok(()) => Json(json!({"ok":true,"status":"started"})).into_response(),
        Err(e) => err500(e).into_response(),
    }
}

async fn mgmt_self_play_stop(
    State(s): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    auth!(s, headers);
    s.self_play.stop().await;
    Json(json!({"ok":true})).into_response()
}

async fn mgmt_self_play_status(
    State(s): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    auth!(s, headers);
    Json(s.self_play.status().await).into_response()
}

// ── Plugin host ───────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct PluginLoadBody {
    id: String,
    path: String,
}

async fn mgmt_plugin_load(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(body): Json<PluginLoadBody>,
) -> impl IntoResponse {
    auth!(s, headers);
    match s.plugin_host.load(&body.id, std::path::Path::new(&body.path)).await {
        Ok(()) => Json(json!({"ok":true,"id":body.id})).into_response(),
        Err(e) => err500(e).into_response(),
    }
}

async fn mgmt_plugin_list(
    State(s): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    auth!(s, headers);
    let list = s.plugin_host.list().await;
    Json(list.into_iter().map(|(id, m)| json!({
        "id": id,
        "name": m.name,
        "version": m.version,
        "capabilities": m.capabilities,
    })).collect::<Vec<_>>()).into_response()
}

#[derive(serde::Deserialize)]
struct PluginExecuteBody {
    id: String,
    payload: Option<String>,
}

async fn mgmt_plugin_execute(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(body): Json<PluginExecuteBody>,
) -> impl IntoResponse {
    auth!(s, headers);
    let payload = body.payload.unwrap_or_default();
    match s.plugin_host.execute(&body.id, &payload, &[]).await {
        Ok(out) => Json(json!({
            "stdout": out.stdout,
            "stderr": out.stderr,
            "exit_code": out.exit_code,
        })).into_response(),
        Err(e) => err500(e).into_response(),
    }
}

// ── Tool registry (list) ──────────────────────────────────────────────────────

async fn mgmt_tools_list(
    State(s): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    auth!(s, headers);
    Json(s.tool_registry.registry.list().await).into_response()
}
