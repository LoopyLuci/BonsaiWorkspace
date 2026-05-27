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
    http::{HeaderMap, StatusCode, Uri},
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
    pub game_sessions:   Arc<crate::games::GameSessionStore>,
    pub knowledge:       Arc<bonsai_knowledge::KnowledgeGraph>,
    pub reasoning:       Arc<crate::reasoning_engine::ReasoningEngine>,
    pub belief_reviser:  Arc<tokio::sync::RwLock<crate::belief_reviser::BeliefReviser>>,
    pub metacognitive:   Arc<tokio::sync::RwLock<crate::metacognitive_monitor::MetacognitiveMonitor>>,
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
        // ── Chess ──────────────────────────────────────────────────────────────
        .route("/api/v1/chess/new",              post(mgmt_chess_new))
        .route("/api/v1/chess/move",             post(mgmt_chess_move))
        .route("/api/v1/chess/resign",           post(mgmt_chess_resign))
        .route("/api/v1/chess/game/:id",         get(mgmt_chess_status))
        // ── Go ─────────────────────────────────────────────────────────────────
        .route("/api/v1/go/new",                 post(mgmt_go_new))
        .route("/api/v1/go/move",                post(mgmt_go_move))
        .route("/api/v1/go/resign",              post(mgmt_go_resign))
        .route("/api/v1/go/game/:id",            get(mgmt_go_status))
        // ── Puzzle ─────────────────────────────────────────────────────────────
        .route("/api/v1/puzzle/daily",           get(mgmt_puzzle_daily))
        .route("/api/v1/puzzle/check",           post(mgmt_puzzle_check))
        // ── Tournament ─────────────────────────────────────────────────────────
        .route("/api/v1/tournament/list",        get(mgmt_tournament_list))
        .route("/api/v1/tournament/create",      post(mgmt_tournament_create))
        // ── Knowledge & Reasoning (v2) ─────────────────────────────────────────
        .route("/api/v2/knowledge/search",       get(mgmt_knowledge_search))
        .route("/api/v2/knowledge/entities",     post(mgmt_knowledge_add_entity))
        .route("/api/v2/knowledge/beliefs",      post(mgmt_knowledge_add_belief))
        .route("/api/v2/knowledge/stats",        get(mgmt_knowledge_stats))
        .route("/api/v2/reason",                 post(mgmt_reason))
        .route("/api/v2/reason/calibration",     get(mgmt_reason_calibration))
        .route("/api/v2/beliefs/check",          post(mgmt_belief_check))
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

// ── Chess REST handlers ───────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ChessNewReq { player_name: Option<String>, human_color: Option<String>, ai_strength: Option<String> }
#[derive(Deserialize)]
struct ChessMoveReq { game_id: String, notation: String }
#[derive(Deserialize)]
struct ChessResignReq { game_id: String }

async fn mgmt_chess_new(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(req): Json<ChessNewReq>,
) -> impl IntoResponse {
    use bonsai_chess::{ChessGameSession, Player as ChessPlayer, PlayerKind as ChessPlayerKind, ChessColor};
    auth!(s, headers);
    let player_name = req.player_name.unwrap_or_else(|| "BotPlayer".into());
    let color       = req.human_color.as_deref().unwrap_or("white");
    let strength    = req.ai_strength.as_deref().unwrap_or("interactive");
    let (white, black) = if color == "white" {
        let h = ChessPlayer { id: "user".into(), name: player_name, kind: ChessPlayerKind::Human, color: ChessColor::White, elo: None };
        let a = ChessPlayer { id: "bonsai".into(), name: "BonsAI".into(), kind: ChessPlayerKind::BonsAI, color: ChessColor::Black, elo: None };
        (h, a)
    } else {
        let a = ChessPlayer { id: "bonsai".into(), name: "BonsAI".into(), kind: ChessPlayerKind::BonsAI, color: ChessColor::White, elo: None };
        let h = ChessPlayer { id: "user".into(), name: player_name, kind: ChessPlayerKind::Human, color: ChessColor::Black, elo: None };
        (a, h)
    };
    let mut session = ChessGameSession::new(white, black);
    if session.needs_ai_move() {
        crate::games::make_chess_ai_move_inner_pub(&mut session, Some(strength));
    }
    let view = crate::games::ChessGameView::from_session_pub(&session);
    let id = session.id.to_string();
    s.game_sessions.chess.write().await.insert(session.id, session);
    Json(json!({ "game_id": id, "human_color": color, "ai_strength": strength, "view": view })).into_response()
}

async fn mgmt_chess_move(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(req): Json<ChessMoveReq>,
) -> impl IntoResponse {
    auth!(s, headers);
    let id: uuid::Uuid = match req.game_id.parse() {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(json!({"error": "invalid game_id"}))).into_response(),
    };
    let mut sessions = s.game_sessions.chess.write().await;
    let session = match sessions.get_mut(&id) {
        Some(s) => s,
        None => return (StatusCode::NOT_FOUND, Json(json!({"error": "game not found"}))).into_response(),
    };
    if let Err(e) = session.apply_move("user", &req.notation) {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))).into_response();
    }
    if session.needs_ai_move() {
        crate::games::make_chess_ai_move_inner_pub(session, None);
    }
    let view = crate::games::ChessGameView::from_session_pub(session);
    Json(json!({ "fen": view.fen, "ai_move": view.legal_moves.first(), "result": view.result, "view": view })).into_response()
}

async fn mgmt_chess_resign(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(req): Json<ChessResignReq>,
) -> impl IntoResponse {
    auth!(s, headers);
    let id: uuid::Uuid = match req.game_id.parse() {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(json!({"error": "invalid game_id"}))).into_response(),
    };
    let mut sessions = s.game_sessions.chess.write().await;
    if let Some(session) = sessions.get_mut(&id) {
        session.resign("user");
    }
    Json(json!({ "ok": true })).into_response()
}

async fn mgmt_chess_status(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    auth!(s, headers);
    let uid: uuid::Uuid = match id.parse() {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(json!({"error": "invalid id"}))).into_response(),
    };
    let sessions = s.game_sessions.chess.read().await;
    match sessions.get(&uid) {
        None => (StatusCode::NOT_FOUND, Json(json!({"error": "game not found"}))).into_response(),
        Some(session) => {
            let view = crate::games::ChessGameView::from_session_pub(session);
            Json(json!({ "game_id": id, "fen": view.fen, "turn": view.current_player_id, "result": view.result })).into_response()
        }
    }
}

// ── Go REST handlers ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GoNewReq { player_name: Option<String>, human_color: Option<String>, board_size: Option<u8>, komi: Option<f32> }
#[derive(Deserialize)]
struct GoMoveReq { game_id: String, gtp: String }
#[derive(Deserialize)]
struct GoResignReq { game_id: String }

async fn mgmt_go_new(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(req): Json<GoNewReq>,
) -> impl IntoResponse {
    use bonsai_go::{GoGameSession, GoPlayer, GoPlayerKind, GoColor};
    auth!(s, headers);
    let player_name = req.player_name.unwrap_or_else(|| "BotPlayer".into());
    let color  = req.human_color.as_deref().unwrap_or("black");
    let size   = req.board_size.unwrap_or(19);
    let size   = if [9u8, 13, 19].contains(&size) { size } else { 19 };
    let komi   = req.komi.unwrap_or(7.5);
    let (black, white) = if color == "black" {
        let h = GoPlayer { id: "user".into(), name: player_name, kind: GoPlayerKind::Human, color: GoColor::Black, rank: None };
        let a = GoPlayer { id: "bonsai".into(), name: "BonsAI".into(), kind: GoPlayerKind::BonsAI, color: GoColor::White, rank: None };
        (h, a)
    } else {
        let a = GoPlayer { id: "bonsai".into(), name: "BonsAI".into(), kind: GoPlayerKind::BonsAI, color: GoColor::Black, rank: None };
        let h = GoPlayer { id: "user".into(), name: player_name, kind: GoPlayerKind::Human, color: GoColor::White, rank: None };
        (a, h)
    };
    let session = GoGameSession::with_options(black, white, size, komi);
    let id = session.id.to_string();
    s.game_sessions.go.write().await.insert(session.id, session);
    Json(json!({ "game_id": id, "board_size": size, "komi": komi, "human_color": color })).into_response()
}

async fn mgmt_go_move(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(req): Json<GoMoveReq>,
) -> impl IntoResponse {
    use bonsai_go::mcts::{RandomGoEvaluator, go_search};
    use bonsai_go::Stone;
    auth!(s, headers);
    let id: uuid::Uuid = match req.game_id.parse() {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(json!({"error": "invalid game_id"}))).into_response(),
    };
    let mut sessions = s.game_sessions.go.write().await;
    let session = match sessions.get_mut(&id) {
        Some(s) => s,
        None => return (StatusCode::NOT_FOUND, Json(json!({"error": "game not found"}))).into_response(),
    };
    if let Err(e) = session.play("user", &req.gtp) {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))).into_response();
    }
    let ai_move = if matches!(session.result, bonsai_go::GoGameResult::Ongoing) {
        let ai_color: Stone = if session.white.id == "bonsai" { Stone::White } else { Stone::Black };
        let board = session.board.clone();
        let eval  = RandomGoEvaluator;
        let cfg   = bonsai_go::GoMctsConfig::interactive();
        let r     = go_search(&board, ai_color, &eval, &cfg);
        let mv    = r.best_move.clone();
        let _ = session.play("bonsai", &mv);
        mv
    } else { String::new() };
    let result = crate::games::GoGameView::from_session_pub(session).result;
    Json(json!({ "ai_move": ai_move, "result": result })).into_response()
}

async fn mgmt_go_resign(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(req): Json<GoResignReq>,
) -> impl IntoResponse {
    auth!(s, headers);
    let id: uuid::Uuid = match req.game_id.parse() {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(json!({"error": "invalid game_id"}))).into_response(),
    };
    let mut sessions = s.game_sessions.go.write().await;
    if let Some(session) = sessions.get_mut(&id) {
        session.resign("user");
    }
    Json(json!({ "ok": true })).into_response()
}

async fn mgmt_go_status(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    auth!(s, headers);
    let uid: uuid::Uuid = match id.parse() {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(json!({"error": "invalid id"}))).into_response(),
    };
    let sessions = s.game_sessions.go.read().await;
    match sessions.get(&uid) {
        None => (StatusCode::NOT_FOUND, Json(json!({"error": "game not found"}))).into_response(),
        Some(session) => {
            let view = crate::games::GoGameView::from_session_pub(session);
            Json(json!({ "game_id": id, "board_size": view.size, "result": view.result })).into_response()
        }
    }
}

// ── Puzzle REST handlers ──────────────────────────────────────────────────────

async fn mgmt_puzzle_daily(
    State(s): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    auth!(s, headers);
    match s.game_sessions.puzzles.daily() {
        None => (StatusCode::NOT_FOUND, Json(json!({"error": "no puzzle available"}))).into_response(),
        Some(p) => Json(json!({
            "id": p.id,
            "fen": p.fen,
            "theme": p.theme,
            "difficulty": p.difficulty,
            "hint": p.hint,
            "description": p.explanation,
            "game_type": "chess",
        })).into_response(),
    }
}

#[derive(Deserialize)]
struct PuzzleCheckReq { puzzle_id: String, uci_move: String }

async fn mgmt_puzzle_check(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(req): Json<PuzzleCheckReq>,
) -> impl IntoResponse {
    auth!(s, headers);
    use crate::games::PuzzleCheckResult;
    let result = s.game_sessions.puzzles.check_move(&req.puzzle_id, &req.uci_move);
    let body = match result {
        PuzzleCheckResult::Solved { explanation } => json!({"status": "solved", "message": explanation}),
        PuzzleCheckResult::CorrectContinue { next_hint } => json!({"status": "correct", "message": next_hint}),
        PuzzleCheckResult::Wrong { hint } => json!({"status": "wrong", "hint": hint}),
        PuzzleCheckResult::NotFound => json!({"status": "not_found"}),
    };
    Json(body).into_response()
}

// ── Tournament REST handlers ──────────────────────────────────────────────────

async fn mgmt_tournament_list(
    State(s): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    auth!(s, headers);
    let list = s.game_sessions.tournaments.list().await;
    Json(serde_json::to_value(&list).unwrap_or_default()).into_response()
}

#[derive(Deserialize)]
struct TournamentCreateReq { name: String, game_type: Option<String>, agent_ids: Vec<String>, agent_names: Vec<String> }

async fn mgmt_tournament_create(
    State(s): State<MgmtState>,
    headers: HeaderMap,
    Json(req): Json<TournamentCreateReq>,
) -> impl IntoResponse {
    auth!(s, headers);
    let game_type = req.game_type.as_deref().unwrap_or("chess");
    let t = s.game_sessions.tournaments.create(
        req.name, game_type.to_string(), req.agent_ids, req.agent_names,
        crate::games::TournamentFormat::RoundRobin { games_per_pair: 2 },
    ).await;
    let id = t.id.clone();
    Json(json!({ "tournament_id": id.to_string() })).into_response()
}

// ── Knowledge & Reasoning handlers (v2) ──────────────────────────────────────

async fn mgmt_knowledge_search(
    State(state): State<MgmtState>,
    uri: Uri,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !authorized(&state, &headers) { return (StatusCode::UNAUTHORIZED, Json(json!({}))).into_response(); }
    let raw_query = uri.query().unwrap_or("").to_string();
    let params: std::collections::HashMap<String, String> = raw_query.split('&')
        .filter_map(|kv| {
            let mut it = kv.splitn(2, '=');
            let k = it.next()?.to_string();
            let v = it.next().unwrap_or("").to_string();
            if k.is_empty() { None } else { Some((k, v)) }
        })
        .collect();
    let q = params.get("q").map(|s| s.as_str()).unwrap_or("");
    let top_k = params.get("top_k").and_then(|s| s.parse::<usize>().ok()).unwrap_or(10);
    let results = state.knowledge.text_search(q, top_k);
    let items: Vec<serde_json::Value> = results.iter().map(|r| match &r.kind {
        bonsai_knowledge::SearchResultKind::Entity(e) =>
            json!({ "kind": "entity", "id": e.id, "name": e.name, "confidence": e.confidence, "score": r.score }),
        bonsai_knowledge::SearchResultKind::Belief(b) =>
            json!({ "kind": "belief", "id": b.id, "statement": b.statement, "confidence": b.confidence, "score": r.score }),
    }).collect();
    Json(json!({ "results": items, "count": items.len() })).into_response()
}

async fn mgmt_knowledge_add_entity(
    State(state): State<MgmtState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    if !authorized(&state, &headers) { return (StatusCode::UNAUTHORIZED, Json(json!({}))).into_response(); }
    let name = body["name"].as_str().unwrap_or("unnamed");
    let entity = bonsai_knowledge::Entity::new(name, bonsai_knowledge::EntityType::Concept);
    let id = state.knowledge.upsert_entity(entity);
    Json(json!({ "entity_id": id })).into_response()
}

async fn mgmt_knowledge_add_belief(
    State(state): State<MgmtState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    if !authorized(&state, &headers) { return (StatusCode::UNAUTHORIZED, Json(json!({}))).into_response(); }
    let statement = body["statement"].as_str().unwrap_or("");
    let confidence = body["confidence"].as_f64().unwrap_or(0.7) as f32;
    let belief = bonsai_knowledge::Belief::new(statement, confidence);
    let id = state.knowledge.add_belief(belief);
    Json(json!({ "belief_id": id })).into_response()
}

async fn mgmt_knowledge_stats(
    State(state): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !authorized(&state, &headers) { return (StatusCode::UNAUTHORIZED, Json(json!({}))).into_response(); }
    Json(serde_json::to_value(state.knowledge.stats()).unwrap_or_default()).into_response()
}

async fn mgmt_reason(
    State(state): State<MgmtState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    if !authorized(&state, &headers) { return (StatusCode::UNAUTHORIZED, Json(json!({}))).into_response(); }
    let query = body["query"].as_str().unwrap_or("");
    let strategy = body["strategy"].as_str().unwrap_or("auto");
    let result = state.reasoning.reason(query, strategy).await;
    Json(serde_json::to_value(&result).unwrap_or_default()).into_response()
}

async fn mgmt_reason_calibration(
    State(state): State<MgmtState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !authorized(&state, &headers) { return (StatusCode::UNAUTHORIZED, Json(json!({}))).into_response(); }
    let report = state.metacognitive.read().await.reflect();
    Json(serde_json::to_value(&report).unwrap_or_default()).into_response()
}

async fn mgmt_belief_check(
    State(state): State<MgmtState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    if !authorized(&state, &headers) { return (StatusCode::UNAUTHORIZED, Json(json!({}))).into_response(); }
    let statement = body["statement"].as_str().unwrap_or("");
    let all_beliefs = state.knowledge.all_beliefs();
    let result = state.belief_reviser.read().await.check_consistency(&all_beliefs, statement);
    let (status, message) = match &result {
        crate::belief_reviser::ConsistencyResult::Consistent =>
            ("consistent", "No contradictions found".to_string()),
        crate::belief_reviser::ConsistencyResult::Contradicts { conflicting, max_conflict_confidence } =>
            ("contradicts", format!("Contradicts {} beliefs (max confidence {:.2})", conflicting.len(), max_conflict_confidence)),
        crate::belief_reviser::ConsistencyResult::Uncertain =>
            ("uncertain", "Cannot determine from available knowledge".to_string()),
    };
    Json(json!({ "statement": statement, "status": status, "message": message })).into_response()
}
