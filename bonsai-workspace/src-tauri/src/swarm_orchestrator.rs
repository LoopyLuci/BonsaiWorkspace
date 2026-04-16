use std::sync::{atomic::AtomicBool, Arc};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot};

use crate::agent_store::ResolvedAgent;
use crate::model_orchestrator::{InferRequest, InferStats, ModelOrchestrator};
use crate::tools;

// ── Plan protocol ─────────────────────────────────────────────────────────────

#[derive(Deserialize, Clone)]
pub struct LeaderPlan {
    pub subtasks: Vec<SubtaskSpec>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct SubtaskSpec {
    pub worker_slot: usize,
    pub task:        String,
    pub context:     String,
}

// ── Public result types ───────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct AgentOutput {
    pub agent_id:    String,
    pub slot_index:  i64,
    pub subtask:     String,
    pub result:      String,
    pub stats:       InferStats,
}

#[derive(Serialize)]
pub struct SwarmResult {
    pub final_response: String,
    pub leader_plan:    Option<Value>,
    pub agent_results:  Vec<AgentOutput>,
    pub stats:          InferStats,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SwarmRuntimeSettings {
    pub leader_plan_required: bool,
    pub max_worker_subtasks: usize,
    pub allow_worker_tools: bool,
    pub enable_worker_cross_review: bool,
    pub parallel_workers: bool,
    pub include_worker_summaries: bool,
    pub synthesis_style: String,
    pub retry_failed_workers: bool,
    pub worker_timeout_ms: u64,
    pub stream_worker_tokens: bool,
    pub emit_debug_events: bool,
    pub max_worker_response_chars: usize,
    pub include_original_prompt_in_worker_context: bool,
    pub allow_leader_as_worker: bool,
}

impl Default for SwarmRuntimeSettings {
    fn default() -> Self {
        Self {
            leader_plan_required: false,
            max_worker_subtasks: 8,
            allow_worker_tools: true,
            enable_worker_cross_review: false,
            parallel_workers: true,
            include_worker_summaries: true,
            synthesis_style: "balanced".to_string(),
            retry_failed_workers: true,
            worker_timeout_ms: 120_000,
            stream_worker_tokens: true,
            emit_debug_events: true,
            max_worker_response_chars: 5000,
            include_original_prompt_in_worker_context: true,
            allow_leader_as_worker: true,
        }
    }
}

// ── Request ───────────────────────────────────────────────────────────────────

pub struct SwarmRequest {
    pub run_id:        String,
    pub session_id:    Option<String>,
    pub user_prompt:   String,
    pub workspace_path: Option<String>,
    pub enabled_tools: Option<Vec<String>>,
    pub runtime_settings: SwarmRuntimeSettings,
    pub agents:        Vec<ResolvedAgent>,    // index 0 = leader
    pub cancel_flags:  Vec<Arc<AtomicBool>>, // per slot
    pub global_cancel: Arc<AtomicBool>,
    pub resp_tx:       oneshot::Sender<Result<SwarmResult, String>>,
    pub app_handle:    AppHandle,
}

// ── Internal channel ──────────────────────────────────────────────────────────

enum SwarmCmd {
    Run(SwarmRequest),
}

// ── Public handle ─────────────────────────────────────────────────────────────

pub struct SwarmOrchestrator {
    cmd_tx: mpsc::UnboundedSender<SwarmCmd>,
}

impl SwarmOrchestrator {
    pub fn new(orchestrator: Arc<ModelOrchestrator>) -> Self {
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<SwarmCmd>();
        tauri::async_runtime::spawn(async move {
            while let Some(SwarmCmd::Run(req)) = cmd_rx.recv().await {
                let orch = orchestrator.clone();
                tauri::async_runtime::spawn(async move {
                    let result = run_swarm(req.app_handle.clone(), orch, req).await;
                    let _ = result;
                });
            }
        });
        Self { cmd_tx }
    }

    pub fn submit(&self, req: SwarmRequest) -> Result<(), String> {
        self.cmd_tx.send(SwarmCmd::Run(req)).map_err(|_| "swarm orchestrator offline".into())
    }
}

// ── Core swarm logic ──────────────────────────────────────────────────────────

async fn run_swarm(
    app_handle: AppHandle,
    orchestrator: Arc<ModelOrchestrator>,
    req: SwarmRequest,
) -> Result<(), ()> {
    let SwarmRequest {
        run_id, workspace_path, enabled_tools, runtime_settings, agents, cancel_flags, global_cancel, resp_tx, user_prompt, ..
    } = req;

    let leader = &agents[0];

    // Build tool list
    let mut tools = tools::all_tools(workspace_path.as_deref());
    if let Some(ref enabled) = enabled_tools {
        let allow: std::collections::HashSet<String> = enabled.iter().cloned().collect();
        tools.retain(|t| allow.contains(&t.name));
    }

    // Build leader system prompt
    let base_prompt = if leader.system_prompt.is_empty() {
        tools::system_prompt(&tools, workspace_path.as_deref())
    } else {
        format!("{}\n\n{}", leader.system_prompt, tools::system_prompt(&tools, workspace_path.as_deref()))
    };

    let workers_summary: String = agents.iter().skip(1)
        .map(|a| {
            let name = a.persona.as_ref().map(|p| p.name.as_str()).unwrap_or(&a.config.label);
            let desc = a.system_prompt.lines().next().unwrap_or("specialist agent");
            format!("  Worker {} ({name}): {desc}", a.config.slot_index)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let leader_sys_prompt = if agents.len() > 1 {
        format!(
            "{base_prompt}\n\n## Swarm coordination\n\nYou are the Leader in a multi-agent swarm. Available workers:\n{workers_summary}\n\nWhen the request benefits from parallel work, output a plan FIRST:\n<swarm_plan>\n{{\"subtasks\":[{{\"worker_slot\":1,\"task\":\"...\",\"context\":\"...\"}},...]}}\n</swarm_plan>\nThen optionally add a brief note. If no decomposition needed, skip the tag and reply directly."
        )
    } else {
        base_prompt.clone()
    };

    if runtime_settings.emit_debug_events {
        let _ = app_handle.emit("swarm-debug", json!({
            "run_id": &run_id,
            "phase": "leader.plan.start",
            "settings": &runtime_settings,
        }));
    }

    let leader_cancel = cancel_flags.get(0).cloned().unwrap_or_else(|| global_cancel.clone());

    // Leader first pass
    let ctx = vec![
        json!({"role": "system", "content": leader_sys_prompt}),
        json!({"role": "user",   "content": &user_prompt}),
    ];

    let leader_agent_id = leader.config.id.clone();
    let leader_slot = leader.config.slot_index;
    let leader_model = leader.effective_model_id.clone();

    let (leader_raw, leader_stats) = match run_agent_inference(
        &*orchestrator,
        &app_handle,
        ctx.clone(),
        leader_agent_id.clone(),
        leader_slot,
        leader_model.clone(),
        Some(leader_cancel.clone()),
        true,
    ).await {
        Ok(v) => v,
        Err(e) => {
            let _ = resp_tx.send(Err(e));
            return Ok(());
        }
    };

    // Strip <think> tags
    let leader_response = strip_think_tags(&leader_raw);

    // Parse <swarm_plan> tag
    let plan = parse_swarm_plan(&leader_response);

    if plan.is_none() || agents.len() == 1 {
        if runtime_settings.leader_plan_required && agents.len() > 1 {
            let fallback_subtasks = vec![SubtaskSpec {
                worker_slot: 1usize.min(agents.len().saturating_sub(1)),
                task: user_prompt.clone(),
                context: "Fallback plan generated because leader_plan_required=true".to_string(),
            }];
            let fallback_plan = LeaderPlan { subtasks: fallback_subtasks };
            return continue_swarm_with_plan(
                app_handle,
                orchestrator,
                run_id,
                workspace_path,
                enabled_tools,
                runtime_settings,
                agents,
                cancel_flags,
                global_cancel,
                resp_tx,
                user_prompt,
                base_prompt,
                leader_agent_id,
                leader_slot,
                leader_model,
                leader_cancel,
                fallback_plan,
            ).await;
        }
        // Single-agent or no plan: return leader response directly
        let final_text = tools::strip_tool_calls(&leader_response);
        let _ = app_handle.emit("swarm-complete", json!({
            "run_id": &run_id,
            "final_content": &final_text,
            "stats": &leader_stats,
        }));
        let _ = resp_tx.send(Ok(SwarmResult {
            final_response: final_text,
            leader_plan: None,
            agent_results: vec![],
            stats: leader_stats,
        }));
        return Ok(());
    }

    continue_swarm_with_plan(
        app_handle,
        orchestrator,
        run_id,
        workspace_path,
        enabled_tools,
        runtime_settings,
        agents,
        cancel_flags,
        global_cancel,
        resp_tx,
        user_prompt,
        base_prompt,
        leader_agent_id,
        leader_slot,
        leader_model,
        leader_cancel,
        plan.unwrap(),
    ).await
}

#[allow(clippy::too_many_arguments)]
async fn continue_swarm_with_plan(
    app_handle: AppHandle,
    orchestrator: Arc<ModelOrchestrator>,
    run_id: String,
    workspace_path: Option<String>,
    enabled_tools: Option<Vec<String>>,
    runtime_settings: SwarmRuntimeSettings,
    agents: Vec<ResolvedAgent>,
    cancel_flags: Vec<Arc<AtomicBool>>,
    global_cancel: Arc<AtomicBool>,
    resp_tx: oneshot::Sender<Result<SwarmResult, String>>,
    user_prompt: String,
    base_prompt: String,
    leader_agent_id: String,
    leader_slot: i64,
    leader_model: Option<String>,
    leader_cancel: Arc<AtomicBool>,
    leader_plan: LeaderPlan,
) -> Result<(), ()> {
    let mut tools = tools::all_tools(workspace_path.as_deref());
    if let Some(ref enabled) = enabled_tools {
        let allow: std::collections::HashSet<String> = enabled.iter().cloned().collect();
        tools.retain(|t| allow.contains(&t.name));
    }

    let mut planned_subtasks = leader_plan.subtasks;
    if planned_subtasks.len() > runtime_settings.max_worker_subtasks {
        planned_subtasks.truncate(runtime_settings.max_worker_subtasks);
    }
    let plan_json = serde_json::to_value(&planned_subtasks).unwrap_or(Value::Null);

    let _ = app_handle.emit("swarm-plan-ready", json!({
        "run_id": &run_id,
        "leader_plan": &plan_json,
    }));

    // Spawn workers (parallel or sequential based on settings)
    let mut worker_handles = Vec::new();
    let mut worker_outputs_sequential: Vec<AgentOutput> = Vec::new();

    for spec in planned_subtasks.clone() {
        let slot_idx = spec.worker_slot;
        if slot_idx == 0 && !runtime_settings.allow_leader_as_worker {
            continue;
        }
        if slot_idx >= agents.len() {
            continue;
        }

        let worker = agents[slot_idx].clone();
        let worker_cancel = cancel_flags.get(slot_idx).cloned()
            .unwrap_or_else(|| global_cancel.clone());

        let orch_clone = orchestrator.clone();
        let ah_clone = app_handle.clone();
        let run_id_clone = run_id.clone();
        let tools_clone = tools.clone();
        let settings = runtime_settings.clone();
        let original_prompt = user_prompt.clone();
        let workspace_path_for_worker = workspace_path.clone();

        let task = async move {
            let mut worker_tools = tools_clone;
            if !settings.allow_worker_tools {
                worker_tools.clear();
            }

            let worker_sys_prompt = if worker.system_prompt.is_empty() {
                tools::system_prompt(&worker_tools, workspace_path_for_worker.as_deref())
            } else {
                format!("{}\n\n{}", worker.system_prompt, tools::system_prompt(&worker_tools, workspace_path_for_worker.as_deref()))
            };

            let ctx_user = if settings.include_original_prompt_in_worker_context {
                format!("## Original user request\n{}\n\n## Task\n{}\n\n## Context\n{}", original_prompt, spec.task, spec.context)
            } else {
                format!("## Task\n{}\n\n## Context\n{}", spec.task, spec.context)
            };

            let worker_ctx = vec![
                json!({"role": "system", "content": worker_sys_prompt}),
                json!({"role": "user", "content": ctx_user}),
            ];

            let agent_id = worker.config.id.clone();
            let slot = worker.config.slot_index;
            let model_id = worker.effective_model_id.clone();
            let task_clone = spec.task.clone();

            let _ = ah_clone.emit("agent-thinking-start", json!({
                "agent_id": &agent_id,
                "slot": slot,
            }));

            let run_once = || {
                let worker_ctx_attempt = worker_ctx.clone();
                let agent_id_attempt = agent_id.clone();
                let model_id_attempt = model_id.clone();
                let worker_cancel_attempt = worker_cancel.clone();
                async {
                    let infer_fut = run_agent_inference(
                        &*orch_clone,
                        &ah_clone,
                        worker_ctx_attempt,
                        agent_id_attempt,
                        slot,
                        model_id_attempt,
                        Some(worker_cancel_attempt),
                        settings.stream_worker_tokens,
                    );
                    match tokio::time::timeout(std::time::Duration::from_millis(settings.worker_timeout_ms), infer_fut).await {
                        Ok(v) => v,
                        Err(_) => Err(format!("Worker timed out after {}ms", settings.worker_timeout_ms)),
                    }
                }
            };

            let mut result = run_once().await;
            if result.is_err() && settings.retry_failed_workers {
                result = run_once().await;
            }

            match result {
                Ok((raw, stats)) => {
                    let mut text = tools::strip_tool_calls(&strip_think_tags(&raw));
                    if text.len() > settings.max_worker_response_chars {
                        text = text.chars().take(settings.max_worker_response_chars).collect::<String>();
                    }
                    let _ = ah_clone.emit("swarm-agent-complete", json!({
                        "run_id": &run_id_clone,
                        "agent_id": &agent_id,
                        "slot": slot,
                        "result": &text,
                        "stats": &stats,
                    }));
                    AgentOutput { agent_id, slot_index: slot, subtask: task_clone, result: text, stats }
                }
                Err(e) => {
                    let _ = ah_clone.emit("swarm-error", json!({
                        "run_id": &run_id_clone,
                        "agent_id": &agent_id,
                        "error": &e,
                    }));
                    AgentOutput { agent_id, slot_index: slot, subtask: task_clone, result: format!("Error: {e}"), stats: InferStats::default() }
                }
            }
        };

        if runtime_settings.parallel_workers {
            worker_handles.push(tokio::spawn(task));
        } else {
            worker_outputs_sequential.push(task.await);
        }
    }

    let mut worker_outputs: Vec<AgentOutput> = worker_outputs_sequential;
    if !worker_handles.is_empty() {
        worker_outputs.extend(
            futures::future::join_all(worker_handles)
                .await
                .into_iter()
                .filter_map(|r| r.ok())
        );
    }

    if runtime_settings.emit_debug_events {
        let _ = app_handle.emit("swarm-debug", json!({
            "run_id": &run_id,
            "phase": "workers.completed",
            "worker_count": worker_outputs.len(),
            "parallel": runtime_settings.parallel_workers,
        }));
    }

    // Build synthesis context
    let synthesis_context = worker_outputs.iter()
        .map(|o| {
            if runtime_settings.include_worker_summaries {
                format!("### Worker {} result\n{}", o.slot_index, o.result)
            } else {
                let summary = o.result.lines().take(3).collect::<Vec<_>>().join(" ");
                format!("### Worker {} summary\n{}", o.slot_index, summary)
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let cross_review_directive = if runtime_settings.enable_worker_cross_review {
        "\n\nCross-review mode: explicitly identify disagreements between workers, choose the most evidence-backed claims, and call out uncertain points."
    } else {
        ""
    };

    let synthesis_user_msg = format!(
        "## Original request\n{user_prompt}\n\n## Worker results\n{synthesis_context}\n\nSynthesis style: {}{}\nSynthesize a final response from the above.",
        runtime_settings.synthesis_style,
        cross_review_directive,
    );

    let synthesis_ctx = vec![
        json!({"role": "system", "content": base_prompt}),
        json!({"role": "user",   "content": synthesis_user_msg}),
    ];

    let (synth_raw, synth_stats) = match run_agent_inference(
        &*orchestrator,
        &app_handle,
        synthesis_ctx,
        leader_agent_id,
        leader_slot,
        leader_model,
        Some(leader_cancel),
        true,
    ).await {
        Ok(v) => v,
        Err(e) => {
            let _ = resp_tx.send(Err(e));
            return Ok(());
        }
    };

    let final_text = tools::strip_tool_calls(&strip_think_tags(&synth_raw));

    if runtime_settings.emit_debug_events {
        let _ = app_handle.emit("swarm-debug", json!({
            "run_id": &run_id,
            "phase": "leader.synthesis.complete",
            "final_chars": final_text.len(),
        }));
    }

    let _ = app_handle.emit("swarm-complete", json!({
        "run_id": &run_id,
        "final_content": &final_text,
        "stats": &synth_stats,
    }));

    let _ = resp_tx.send(Ok(SwarmResult {
        final_response: final_text,
        leader_plan: Some(plan_json),
        agent_results: worker_outputs,
        stats: synth_stats,
    }));

    Ok(())
}

// ── Agent inference helper ────────────────────────────────────────────────────

async fn run_agent_inference(
    orchestrator: &ModelOrchestrator,
    app_handle:   &AppHandle,
    messages:     Vec<Value>,
    agent_id:     String,
    slot:         i64,
    model_id:     Option<String>,
    cancel_flag:  Option<Arc<AtomicBool>>,
    emit_tokens:  bool,
) -> Result<(String, InferStats), String> {
    let (resp_tx, resp_rx) = oneshot::channel();
    let (stream_tx, mut stream_rx) = mpsc::unbounded_channel::<String>();

    orchestrator.infer(InferRequest {
        model_id,
        messages,
        max_tokens: 4096,
        stream_tx: Some(stream_tx),
        cancel_flag,
        resp_tx,
    })?;

    let ah = app_handle.clone();
    let aid = agent_id.clone();
    tokio::spawn(async move {
        while let Some(tok) = stream_rx.recv().await {
            if emit_tokens {
                let _ = ah.emit("agent-token-stream", json!({
                    "agent_id": &aid,
                    "slot": slot,
                    "token": &tok,
                }));
            }
            // Also forward to backward-compat token-stream if slot 0 (leader synthesis)
            if slot == 0 {
                let _ = ah.emit("token-stream", &tok);
            }
        }
    });

    resp_rx.await.map_err(|_| "Request cancelled".to_string())?
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_swarm_plan(text: &str) -> Option<LeaderPlan> {
    let start = text.find("<swarm_plan>")?;
    let end   = text.find("</swarm_plan>")?;
    if end <= start { return None; }
    let json_str = &text[start + "<swarm_plan>".len()..end];
    serde_json::from_str(json_str).ok()
}

fn strip_think_tags(text: &str) -> String {
    let mut s = text.to_string();
    while let (Some(a), Some(b)) = (s.find("<think>"), s.find("</think>")) {
        if b > a {
            s = format!("{}{}", &s[..a], &s[b + "</think>".len()..]);
        } else {
            break;
        }
    }
    s.trim().to_string()
}
