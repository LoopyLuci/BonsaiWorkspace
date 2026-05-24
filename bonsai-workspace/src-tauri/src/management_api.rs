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
    Json(json!({
        "adapter_loaded": false,
        "avg_latency_ms": 0.0,
        "fallback_rate": 0.0,
        "memory_entries": 0,
        "queue_depth": queue.pending_total,
        "active_tasks": queue.active_total,
    }))
    .into_response()
}
