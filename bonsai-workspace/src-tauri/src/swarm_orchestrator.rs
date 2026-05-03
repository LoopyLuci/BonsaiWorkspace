use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

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
    /// Optional per-subtask tool allow-list.  When Some, the worker receives only
    /// these tools regardless of the global tool set.  When None, the worker gets
    /// the role-appropriate default (read-only for reviewers/analysts; full set for
    /// implementers and deep-specialists).  The leader can override either direction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
}

// ── Public result types ───────────────────────────────────────────────────────

/// Structured self-assessment emitted by each worker inside <worker_assessment> tags.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct WorkerAssessment {
    /// 0-100. 90+ = verified by tools; 70-89 = strong evidence; 50-69 = partial; <50 = inference.
    pub confidence: f64,
    /// Specific observations cited as evidence (e.g. "src/auth.rs:47 — buffer overflow confirmed").
    pub evidence_sources: Vec<String>,
    /// What the worker could not verify or explicitly left open.
    pub gaps: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct AgentOutput {
    pub agent_id:    String,
    pub slot_index:  i64,
    pub subtask:     String,
    pub result:      String,
    pub stats:       InferStats,
    /// Parsed from the worker's <worker_assessment> block; None if worker did not emit one.
    pub assessment:  Option<WorkerAssessment>,
}

#[derive(Serialize)]
pub struct SwarmResult {
    pub final_response: String,
    pub leader_plan:    Option<Value>,
    pub agent_results:  Vec<AgentOutput>,
    pub stats:          InferStats,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
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
    pub chain_strategy: String,
    pub stop_on_first_satisfactory: bool,
    pub satisfaction_threshold: u8,
    pub preferred_primary_slot: usize,
    pub force_all_workers_before_decision: bool,
    pub heavy_work_delegate_mode: String,
    pub configured_heavy_worker_slot: usize,
    pub heavy_work_delegate_auto_fallback: bool,
    pub auto_repair_delegate_routing: bool,
    pub agent_chain_policies: Vec<AgentChainPolicy>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AgentChainPolicy {
    pub slot_index: usize,
    pub execution_tier: usize,
    pub always_run: bool,
    pub can_be_early_exit_gate: bool,
    pub early_exit_confidence_threshold: u8,
    pub response_weight: usize,
    pub can_review_from_slots: Vec<usize>,
    pub can_delegate_to_slots: Vec<usize>,
    pub allow_heavy_work: bool,
}

impl Default for AgentChainPolicy {
    fn default() -> Self {
        Self {
            slot_index: 0,
            execution_tier: 0,
            always_run: false,
            can_be_early_exit_gate: false,
            early_exit_confidence_threshold: 78,
            response_weight: 1,
            can_review_from_slots: vec![],
            can_delegate_to_slots: vec![],
            allow_heavy_work: false,
        }
    }
}

impl Default for SwarmRuntimeSettings {
    fn default() -> Self {
        Self {
            leader_plan_required: true,
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
            chain_strategy: "parallel_then_delegate".to_string(),
            stop_on_first_satisfactory: false,
            satisfaction_threshold: 78,
            preferred_primary_slot: 1,
            force_all_workers_before_decision: true,
            heavy_work_delegate_mode: "selected".to_string(),
            configured_heavy_worker_slot: 2,
            heavy_work_delegate_auto_fallback: false,
            auto_repair_delegate_routing: false,
            agent_chain_policies: vec![],
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

    let leader = agents
        .iter()
        .find(|a| a.config.slot_index == 0)
        .unwrap_or(&agents[0]);
    let slot_to_agent: HashMap<usize, &ResolvedAgent> = agents
        .iter()
        .map(|a| (a.config.slot_index as usize, a))
        .collect();

    if runtime_settings.emit_debug_events {
        let enabled_slots: Vec<i64> = agents
            .iter()
            .filter(|a| a.config.enabled)
            .map(|a| a.config.slot_index)
            .collect();
        let delegate_policy_health: Vec<Value> = runtime_settings
            .agent_chain_policies
            .iter()
            .map(|policy| {
                let checks: Vec<Value> = policy
                    .can_delegate_to_slots
                    .iter()
                    .map(|candidate| {
                        let status = if *candidate == policy.slot_index {
                            "self"
                        } else if !slot_to_agent.contains_key(candidate) {
                            "out_of_range"
                        } else if !slot_to_agent.get(candidate).map(|a| a.config.enabled).unwrap_or(false) {
                            "disabled"
                        } else if !policy_for_slot(&runtime_settings, *candidate).allow_heavy_work {
                            "heavy_off"
                        } else {
                            "ok"
                        };
                        json!({
                            "candidate_slot": *candidate,
                            "status": status,
                        })
                    })
                    .collect();

                let invalid_count = checks
                    .iter()
                    .filter(|entry| entry.get("status").and_then(|s| s.as_str()) != Some("ok"))
                    .count();
                json!({
                    "slot_index": policy.slot_index,
                    "candidate_count": checks.len(),
                    "invalid_count": invalid_count,
                    "checks": checks,
                })
            })
            .collect();

        let _ = app_handle.emit("swarm-debug", json!({
            "run_id": &run_id,
            "phase": "run.start",
            "chain_strategy": &runtime_settings.chain_strategy,
            "stop_on_first_satisfactory": runtime_settings.stop_on_first_satisfactory,
            "satisfaction_threshold": runtime_settings.satisfaction_threshold,
            "force_all_workers_before_decision": runtime_settings.force_all_workers_before_decision,
            "heavy_work_delegate_mode": &runtime_settings.heavy_work_delegate_mode,
            "configured_heavy_worker_slot": runtime_settings.configured_heavy_worker_slot,
            "heavy_work_delegate_auto_fallback": runtime_settings.heavy_work_delegate_auto_fallback,
            "auto_repair_delegate_routing": runtime_settings.auto_repair_delegate_routing,
            "agent_count": agents.len(),
            "enabled_slots": enabled_slots,
            "delegate_policy_health": delegate_policy_health,
        }));
    }

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

    let workers_summary: String = agents.iter().filter(|a| a.config.slot_index != 0 && a.config.enabled)
        .map(|a| {
            let name = a.persona.as_ref().map(|p| p.name.as_str()).unwrap_or(&a.config.label);
            let desc = a.system_prompt.lines().next().unwrap_or("specialist agent");
            let role = role_profile_for_slot(&runtime_settings, a.config.slot_index as usize);
            format!("  Worker {} ({name}) [{}/{}]: {desc}", a.config.slot_index, role.role_name, role.focus)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let workspace_hint = workspace_path
        .as_deref()
        .map(|p| format!("\nWorkspace path: {p}"))
        .unwrap_or_default();

    let leader_sys_prompt = if agents.len() > 1 {
        format!(
            "{base_prompt}\n\n## Swarm coordination\n\nYou are the Leader in a multi-agent swarm. Available workers:\n{workers_summary}{workspace_hint}\n\nDecompose by role, not by duplication:\n- Assign each worker a distinct angle (implementation, verification, risk review, architecture, UX, etc.).\n- Every subtask must include concrete objective, expected output artifact, and constraints.\n- Prefer grounded tasks that reference workspace paths or verifiable checks when relevant.\n\nOutput a plan FIRST when decomposition helps:\n<swarm_plan>\n{{\"subtasks\":[{{\"worker_slot\":1,\"task\":\"objective + deliverable\",\"context\":\"constraints + evidence targets\",\"allowed_tools\":[\"read_file\",\"list_files\"]}},...]}}\n</swarm_plan>\nThe `allowed_tools` field is optional. Omit it to use role defaults (reviewers/analysts get read-only tools; implementers and specialists get full access). Include it only when you need to override the default — e.g. grant a reviewer write access for a specific subtask, or restrict an implementer to read-only research.\nThen optionally add a brief note. If no decomposition needed, skip the tag and reply directly."
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

    // Parse <swarm_plan> tag — with structured error reporting and one repair attempt.
    let parsed_plan = match parse_swarm_plan(&leader_response) {
        Ok(plan) => Some(plan),
        Err(reason) => {
            if runtime_settings.emit_debug_events {
                let _ = app_handle.emit("swarm-debug", json!({
                    "run_id": &run_id,
                    "phase": "leader.plan.parse_failed",
                    "reason": &reason,
                    "snippet": &leader_response[..leader_response.len().min(400)],
                }));
            }

            // One repair pass: show the leader its own output, explain the error,
            // and ask it to re-emit a clean plan block.
            let repair_ctx = vec![
                ctx[0].clone(), // system message (leader_sys_prompt)
                json!({"role": "user",      "content": &user_prompt}),
                json!({"role": "assistant", "content": &leader_raw}),
                json!({"role": "user",      "content": format!(
                    "Your plan could not be parsed ({reason}).\n\
                     Re-emit ONLY a corrected <swarm_plan> block — no prose before or after the tags.\n\
                     Required format:\n\
                     <swarm_plan>\n\
                     {{\"subtasks\":[{{\"worker_slot\":1,\"task\":\"objective + deliverable\",\"context\":\"constraints\"}}]}}\n\
                     </swarm_plan>"
                )}),
            ];

            let repair_result = run_agent_inference(
                &*orchestrator, &app_handle,
                repair_ctx,
                leader_agent_id.clone(), leader_slot, leader_model.clone(),
                Some(leader_cancel.clone()),
                false, // don't stream the repair pass
            ).await;

            match repair_result {
                Ok((repair_raw, _)) => match parse_swarm_plan(&strip_think_tags(&repair_raw)) {
                    Ok(repaired) => {
                        if runtime_settings.emit_debug_events {
                            let _ = app_handle.emit("swarm-debug", json!({
                                "run_id": &run_id,
                                "phase": "leader.plan.repaired",
                            }));
                        }
                        Some(repaired)
                    }
                    Err(repair_reason) => {
                        if runtime_settings.emit_debug_events {
                            let _ = app_handle.emit("swarm-debug", json!({
                                "run_id": &run_id,
                                "phase": "leader.plan.repair_failed",
                                "reason": &repair_reason,
                            }));
                        }
                        None // fall through to role-framed fallback subtasks
                    }
                },
                Err(_) => None,
            }
        }
    };
    let enabled_worker_slots = enabled_worker_slots(&agents);

    if enabled_worker_slots.is_empty() {
        // Single-agent fallback: return leader response directly.
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

    let fallback_subtasks = enabled_worker_slots
        .iter()
        .map(|slot| {
            let profile = role_profile_for_slot(&runtime_settings, *slot);
            SubtaskSpec {
                worker_slot: *slot,
                task: format!(
                    "As the {}, address the following request: {}",
                    profile.role_name,
                    user_prompt,
                ),
                context: format!(
                    "Your role: {}. Your focus: {}. Required deliverable: {}.\n\
                     Only use workspace tools when the request requires inspecting files or machine state. \
                     Answer knowledge questions (language, math, science, history, definitions) directly without tools.",
                    profile.role_name,
                    profile.focus,
                    profile.deliverable,
                ),
                allowed_tools: None,
            }
        })
        .collect::<Vec<_>>();
    let effective_plan = parsed_plan.unwrap_or(LeaderPlan { subtasks: fallback_subtasks });

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
        effective_plan,
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
    let slot_to_agent: HashMap<usize, ResolvedAgent> = agents
        .iter()
        .cloned()
        .map(|a| (a.config.slot_index as usize, a))
        .collect();
    let enabled_worker_slots = enabled_worker_slots(&agents);

    let mut tools = tools::all_tools(workspace_path.as_deref());
    if let Some(ref enabled) = enabled_tools {
        let allow: std::collections::HashSet<String> = enabled.iter().cloned().collect();
        tools.retain(|t| allow.contains(&t.name));
    }

    let mut planned_subtasks = normalize_subtasks_for_active_workers(
        leader_plan.subtasks,
        &enabled_worker_slots,
        &user_prompt,
        &runtime_settings,
    );
    for spec in &mut planned_subtasks {
        let profile = role_profile_for_slot(&runtime_settings, spec.worker_slot);
        let lowered = spec.context.to_lowercase();
        if !lowered.contains("deliverable") {
            spec.context = format!(
                "{}\nRole guidance: {} focus on {}. Deliverable: {}",
                spec.context,
                profile.role_name,
                profile.focus,
                profile.deliverable,
            );
        }
    }
    let min_required = enabled_worker_slots.len();
    let effective_max = runtime_settings.max_worker_subtasks.max(min_required);
    if planned_subtasks.len() > effective_max {
        planned_subtasks.truncate(effective_max);
    }
    let plan_json = serde_json::to_value(&planned_subtasks).unwrap_or(Value::Null);

    let _ = app_handle.emit("swarm-plan-ready", json!({
        "run_id": &run_id,
        "leader_plan": &plan_json,
    }));

    // Chain-of-command scheduling based on per-agent policy + global strategy.
    let mut ordered_subtasks = planned_subtasks.clone();
    ordered_subtasks.sort_by_key(|spec| {
        let policy = policy_for_slot(&runtime_settings, spec.worker_slot);
        let primary_bias = if spec.worker_slot == runtime_settings.preferred_primary_slot { 0usize } else { 1usize };
        let always_bias = if policy.always_run { 0usize } else { 1usize };
        (always_bias, policy.execution_tier, primary_bias, spec.worker_slot)
    });

    let chain_strategy = runtime_settings.chain_strategy.to_lowercase();
    // Derive execution mode from chain_strategy — previously this was hardcoded to true,
    // which made sequential strategies dead code.
    let run_parallel = !matches!(
        chain_strategy.as_str(),
        "sequential_gate" | "sequential_then_delegate"
    );

    let mut worker_outputs: Vec<AgentOutput> = Vec::new();

    if run_parallel {
        let mut worker_handles = Vec::new();
        for spec in ordered_subtasks.clone() {
            let slot_idx = spec.worker_slot;
            if slot_idx == 0 && !runtime_settings.allow_leader_as_worker {
                continue;
            }
            if !slot_to_agent.contains_key(&slot_idx) {
                continue;
            }

            let worker = slot_to_agent.get(&slot_idx).cloned().unwrap();
            let worker_cancel = cancel_flags.get(slot_idx).cloned().unwrap_or_else(|| global_cancel.clone());
            let ah_clone = app_handle.clone();
            let orch_clone = orchestrator.clone();
            let run_id_clone = run_id.clone();
            let settings_clone = runtime_settings.clone();
            let tools_clone = tools.clone();
            let workspace_clone = workspace_path.clone();
            let original_prompt = user_prompt.clone();

            worker_handles.push(tokio::spawn(async move {
                run_worker_subtask(
                    &ah_clone,
                    &*orch_clone,
                    &run_id_clone,
                    &settings_clone,
                    &tools_clone,
                    workspace_clone,
                    &original_prompt,
                    &worker,
                    &spec,
                    worker_cancel,
                    String::new(),
                ).await
            }));
        }

        worker_outputs.extend(
            futures::future::join_all(worker_handles)
                .await
                .into_iter()
                .filter_map(|r| r.ok())
        );
    } else {
        for spec in ordered_subtasks.clone() {
            let slot_idx = spec.worker_slot;
            if slot_idx == 0 && !runtime_settings.allow_leader_as_worker {
                continue;
            }
            if !slot_to_agent.contains_key(&slot_idx) {
                continue;
            }

            let policy = policy_for_slot(&runtime_settings, slot_idx);
            let allowed_prior = worker_outputs
                .iter()
                .filter(|o| policy.can_review_from_slots.contains(&(o.slot_index as usize)))
                .map(|o| format!("Worker {} prior result:\n{}", o.slot_index, o.result))
                .collect::<Vec<_>>()
                .join("\n\n");

            let worker = slot_to_agent.get(&slot_idx).cloned().unwrap();
            let worker_cancel = cancel_flags.get(slot_idx).cloned().unwrap_or_else(|| global_cancel.clone());

            let out = run_worker_subtask(
                &app_handle,
                &*orchestrator,
                &run_id,
                &runtime_settings,
                &tools,
                workspace_path.clone(),
                &user_prompt,
                &worker,
                &spec,
                worker_cancel,
                allowed_prior,
            ).await;
            let score = score_agent_output(&out, &policy);
            worker_outputs.push(out);

            let threshold = f64::from(policy.early_exit_confidence_threshold.min(100));
            let can_early_exit = runtime_settings.stop_on_first_satisfactory
                && policy.can_be_early_exit_gate
                && !runtime_settings.force_all_workers_before_decision;
            if can_early_exit && score >= threshold {
                if runtime_settings.emit_debug_events {
                    let _ = app_handle.emit("swarm-debug", json!({
                        "run_id": &run_id,
                        "phase": "early_exit.gate_triggered",
                        "slot": slot_idx,
                        "score": score,
                        "threshold": threshold,
                    }));
                }
                break;
            }
        }
    }

    let enable_heavy_delegate = matches!(chain_strategy.as_str(), "parallel_then_delegate" | "sequential_then_delegate")
        && runtime_settings.heavy_work_delegate_mode != "none"
        && !worker_outputs.is_empty();

    if enable_heavy_delegate {
        let selected_slot = if runtime_settings.heavy_work_delegate_mode == "configured" {
            runtime_settings.configured_heavy_worker_slot
        } else {
            let mut scored = worker_outputs
                .iter()
                .map(|o| {
                    let policy = policy_for_slot(&runtime_settings, o.slot_index as usize);
                    (o.slot_index as usize, score_agent_output(o, &policy))
                })
                .collect::<Vec<_>>();
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            scored.first().map(|(slot, _)| *slot).unwrap_or(runtime_settings.preferred_primary_slot)
        };

        let base_policy = policy_for_slot(&runtime_settings, selected_slot);
        let mut target_slot = selected_slot;
        let validate_delegate_slot = |slot: usize| -> Option<&'static str> {
            if !slot_to_agent.contains_key(&slot) {
                return Some("target_out_of_range");
            }
            if !slot_to_agent.get(&slot).map(|a| a.config.enabled).unwrap_or(false) {
                return Some("target_agent_disabled");
            }
            if !policy_for_slot(&runtime_settings, slot).allow_heavy_work {
                return Some("target_policy_disallows_heavy_work");
            }
            None
        };

        if !base_policy.can_delegate_to_slots.is_empty() {
            for candidate in &base_policy.can_delegate_to_slots {
                if *candidate == selected_slot {
                    if runtime_settings.emit_debug_events {
                        let _ = app_handle.emit("swarm-debug", json!({
                            "run_id": &run_id,
                            "phase": "delegate.skip_candidate",
                            "candidate_slot": *candidate,
                            "reason": "self_delegate_forbidden",
                        }));
                    }
                    continue;
                }
                if !slot_to_agent.contains_key(candidate) {
                    if runtime_settings.emit_debug_events {
                        let _ = app_handle.emit("swarm-debug", json!({
                            "run_id": &run_id,
                            "phase": "delegate.skip_candidate",
                            "candidate_slot": *candidate,
                            "reason": "out_of_range",
                        }));
                    }
                    continue;
                }

                if !slot_to_agent.get(candidate).map(|a| a.config.enabled).unwrap_or(false) {
                    if runtime_settings.emit_debug_events {
                        let _ = app_handle.emit("swarm-debug", json!({
                            "run_id": &run_id,
                            "phase": "delegate.skip_candidate",
                            "candidate_slot": *candidate,
                            "reason": "agent_disabled",
                        }));
                    }
                    continue;
                }

                if !policy_for_slot(&runtime_settings, *candidate).allow_heavy_work {
                    if runtime_settings.emit_debug_events {
                        let _ = app_handle.emit("swarm-debug", json!({
                            "run_id": &run_id,
                            "phase": "delegate.skip_candidate",
                            "candidate_slot": *candidate,
                            "reason": "policy_disallows_heavy_work",
                        }));
                    }
                    continue;
                }

                target_slot = *candidate;
                break;
            }
        }

        let mut target_invalid_reason = validate_delegate_slot(target_slot);
        if target_invalid_reason.is_some() && runtime_settings.heavy_work_delegate_auto_fallback {
            let mut scored = worker_outputs
                .iter()
                .map(|o| {
                    let policy = policy_for_slot(&runtime_settings, o.slot_index as usize);
                    (o.slot_index as usize, score_agent_output(o, &policy))
                })
                .collect::<Vec<_>>();
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            let from_slot = target_slot;
            let from_reason = target_invalid_reason.unwrap_or("unknown").to_string();
            for (slot, _) in scored {
                if slot == from_slot {
                    continue;
                }
                if let Some(reason) = validate_delegate_slot(slot) {
                    if runtime_settings.emit_debug_events {
                        let _ = app_handle.emit("swarm-debug", json!({
                            "run_id": &run_id,
                            "phase": "delegate.skip_candidate",
                            "candidate_slot": slot,
                            "reason": reason,
                            "source": "fallback_scan",
                        }));
                    }
                    continue;
                }

                target_slot = slot;
                target_invalid_reason = None;
                if runtime_settings.emit_debug_events {
                    let _ = app_handle.emit("swarm-debug", json!({
                        "run_id": &run_id,
                        "phase": "delegate.fallback_applied",
                        "from_slot": from_slot,
                        "to_slot": target_slot,
                        "reason": from_reason,
                    }));
                }
                break;
            }
        }

        if target_invalid_reason.is_none() && slot_to_agent.contains_key(&target_slot) {
            let delegate_worker = slot_to_agent.get(&target_slot).cloned().unwrap();
            let delegate_policy = policy_for_slot(&runtime_settings, target_slot);
            if !delegate_worker.config.enabled {
                if runtime_settings.emit_debug_events {
                    let _ = app_handle.emit("swarm-debug", json!({
                        "run_id": &run_id,
                        "phase": "delegate.skipped",
                        "target_slot": target_slot,
                        "reason": "target_agent_disabled",
                    }));
                }
            } else if delegate_policy.allow_heavy_work {
                let worker_cancel = cancel_flags.get(target_slot).cloned().unwrap_or_else(|| global_cancel.clone());
                let best_summary = worker_outputs
                    .iter()
                    .map(|o| format!("Worker {}:\n{}", o.slot_index, o.result))
                    .collect::<Vec<_>>()
                    .join("\n\n");

                let delegate_spec = SubtaskSpec {
                    worker_slot: target_slot,
                    task: "Perform the heavy implementation/deep reasoning pass now using the chosen strategy and produce implementation-ready output.".to_string(),
                    context: "Heavy-work delegation stage after leader comparison.".to_string(),
                    allowed_tools: None,
                };

                let out = run_worker_subtask(
                    &app_handle,
                    &*orchestrator,
                    &run_id,
                    &runtime_settings,
                    &tools,
                    workspace_path.clone(),
                    &user_prompt,
                    &delegate_worker,
                    &delegate_spec,
                    worker_cancel,
                    best_summary,
                ).await;
                worker_outputs.push(out);
            } else if runtime_settings.emit_debug_events {
                let _ = app_handle.emit("swarm-debug", json!({
                    "run_id": &run_id,
                    "phase": "delegate.skipped",
                    "target_slot": target_slot,
                    "reason": "target_policy_disallows_heavy_work",
                }));
            }
        } else if runtime_settings.emit_debug_events {
            let reason = target_invalid_reason.unwrap_or("target_out_of_range");
            let _ = app_handle.emit("swarm-debug", json!({
                "run_id": &run_id,
                "phase": "delegate.skipped",
                "target_slot": target_slot,
                "reason": reason,
            }));
        }
    }

    // Guarantee visibility of all enabled workers in the result set.
    // If any enabled worker produced no output, add an explicit placeholder.
    let mut reported_slots: HashSet<usize> = worker_outputs
        .iter()
        .map(|o| o.slot_index as usize)
        .collect();
    let mut missing_slots = enabled_worker_slots
        .iter()
        .copied()
        .filter(|slot| !reported_slots.contains(slot))
        .collect::<Vec<_>>();
    missing_slots.sort_unstable();

    if !missing_slots.is_empty() {
        if runtime_settings.emit_debug_events {
            let _ = app_handle.emit("swarm-debug", json!({
                "run_id": &run_id,
                "phase": "workers.retry_missing.start",
                "missing_worker_slots": &missing_slots,
            }));
        }

        let subtask_by_slot: HashMap<usize, SubtaskSpec> = ordered_subtasks
            .iter()
            .map(|spec| (spec.worker_slot, spec.clone()))
            .collect();

        let prior_context = worker_outputs
            .iter()
            .map(|o| format!("Worker {} prior result:\n{}", o.slot_index, o.result))
            .collect::<Vec<_>>()
            .join("\n\n");

        let mut retry_handles = Vec::new();
        for slot in missing_slots.clone() {
            let Some(worker) = slot_to_agent.get(&slot).cloned() else { continue };
            let worker_cancel = cancel_flags.get(slot).cloned().unwrap_or_else(|| global_cancel.clone());
            let spec = subtask_by_slot.get(&slot).cloned().unwrap_or(SubtaskSpec {
                worker_slot: slot,
                task: format!("Retry worker task after initial miss: {}", user_prompt),
                context: "Recovery assignment generated after missing worker output.".to_string(),
                allowed_tools: None,
            });

            let ah_clone = app_handle.clone();
            let orch_clone = orchestrator.clone();
            let run_id_clone = run_id.clone();
            let settings_clone = runtime_settings.clone();
            let tools_clone = tools.clone();
            let workspace_clone = workspace_path.clone();
            let original_prompt = user_prompt.clone();
            let prior_clone = prior_context.clone();

            retry_handles.push(tokio::spawn(async move {
                run_worker_subtask(
                    &ah_clone,
                    &*orch_clone,
                    &run_id_clone,
                    &settings_clone,
                    &tools_clone,
                    workspace_clone,
                    &original_prompt,
                    &worker,
                    &spec,
                    worker_cancel,
                    prior_clone,
                ).await
            }));
        }

        let retry_outputs = futures::future::join_all(retry_handles)
            .await
            .into_iter()
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();
        worker_outputs.extend(retry_outputs);

        reported_slots = worker_outputs
            .iter()
            .map(|o| o.slot_index as usize)
            .collect();
        missing_slots = enabled_worker_slots
            .iter()
            .copied()
            .filter(|slot| !reported_slots.contains(slot))
            .collect::<Vec<_>>();
        missing_slots.sort_unstable();

        if runtime_settings.emit_debug_events {
            let _ = app_handle.emit("swarm-debug", json!({
                "run_id": &run_id,
                "phase": "workers.retry_missing.complete",
                "missing_worker_slots": &missing_slots,
            }));
        }
    }

    for slot in &missing_slots {
        if let Some(worker) = slot_to_agent.get(slot) {
            worker_outputs.push(AgentOutput {
                agent_id: worker.config.id.clone(),
                slot_index: worker.config.slot_index,
                subtask: "[auto-detected missing worker output]".to_string(),
                result: "Error: Worker did not return a final output in this run. Check swarm-debug/swarm-error telemetry for timeout/cancellation/details.".to_string(),
                stats: InferStats::default(),
                assessment: None,
            });

            if runtime_settings.emit_debug_events {
                let _ = app_handle.emit("swarm-error", json!({
                    "run_id": &run_id,
                    "agent_id": worker.config.id,
                    "slot": slot,
                    "error": "missing_worker_output",
                }));
            }
        }
    }

    worker_outputs.sort_by_key(|o| o.slot_index);

    if runtime_settings.emit_debug_events {
        let _ = app_handle.emit("swarm-debug", json!({
            "run_id": &run_id,
            "phase": "workers.completed",
            "worker_count": worker_outputs.len(),
            "expected_worker_count": enabled_worker_slots.len(),
            "missing_worker_slots": missing_slots,
            "parallel": run_parallel,
            "chain_strategy": runtime_settings.chain_strategy,
        }));
    }

    // Build synthesis context — exclude failed workers, surface self-assessments.
    let (successful_outputs, failed_outputs): (Vec<_>, Vec<_>) = worker_outputs
        .iter()
        .partition(|o| !o.result.trim_start().starts_with("Error:") && !o.result.trim().is_empty());

    let synthesis_context = successful_outputs.iter()
        .map(|o| {
            let assessment_line = o.assessment.as_ref().map(|a| {
                let sources = if a.evidence_sources.is_empty() { "none cited".to_string() } else { a.evidence_sources.join("; ") };
                let gaps    = if a.gaps.is_empty() { "none".to_string() } else { a.gaps.join("; ") };
                format!("\n> Self-assessment — confidence: {}%, evidence: [{}], gaps: [{}]", a.confidence as u32, sources, gaps)
            }).unwrap_or_default();

            if runtime_settings.include_worker_summaries {
                format!("### Worker {} result{}\n{}", o.slot_index, assessment_line, o.result)
            } else {
                let summary = o.result.lines().take(3).collect::<Vec<_>>().join(" ");
                format!("### Worker {} summary{}\n{}", o.slot_index, assessment_line, summary)
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let failure_note = if failed_outputs.is_empty() {
        String::new()
    } else {
        let list = failed_outputs.iter()
            .map(|o| format!("  - Worker {}: {}", o.slot_index, &o.result[..o.result.len().min(120)]))
            .collect::<Vec<_>>()
            .join("\n");
        format!("\n\n**Workers excluded (failed or timed out — do not speculate about their output):**\n{list}")
    };

    let cross_review_directive = if runtime_settings.enable_worker_cross_review {
        "\n\nCross-review: explicitly identify disagreements, choose the most evidence-backed claims, and call out uncertain points."
    } else {
        ""
    };

    let strategy_directive = match chain_strategy.as_str() {
        "parallel_vote" => "\n\nVote synthesis: for each key claim, count agreements vs. disagreements across workers. Prefer the majority view and state the tally (e.g. '2 of 3 workers confirmed X'). Note any minority views.",
        _ => "",
    };

    let synthesis_user_msg = format!(
        "## Original request\n{user_prompt}\n\n## Worker results\n{synthesis_context}{failure_note}\n\nSynthesis style: {}{}{}\n\nReconcile before writing:\n- Identify key agreements and disagreements between workers.\n- Workers with higher confidence and cited evidence sources carry more weight.\n- Resolve conflicts by preferring evidence-backed claims; where unresolved, state uncertainty explicitly.\n- Merge complementary findings into one coherent answer — do not concatenate raw worker outputs.\n- Attribute important claims to worker slots when it aids traceability.\n- End with a concrete final answer and actionable next steps.",
        runtime_settings.synthesis_style,
        cross_review_directive,
        strategy_directive,
    );

    let synthesis_sys_prompt = synthesis_system_prompt(workspace_path.as_deref());
    let synthesis_ctx = vec![
        json!({"role": "system", "content": synthesis_sys_prompt}),
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

async fn run_worker_subtask(
    app_handle: &AppHandle,
    orchestrator: &ModelOrchestrator,
    run_id: &str,
    settings: &SwarmRuntimeSettings,
    tools_available: &Vec<tools::ToolDef>,
    workspace_path: Option<String>,
    original_prompt: &str,
    worker: &ResolvedAgent,
    spec: &SubtaskSpec,
    worker_cancel: Arc<AtomicBool>,
    prior_results_context: String,
) -> AgentOutput {
    // Resolve role first — it drives the default tool restriction.
    let role_profile = role_profile_for_slot(settings, worker.config.slot_index as usize);

    let mut worker_tools = tools_available.clone();
    if !settings.allow_worker_tools {
        // Global kill-switch: strip all tools.
        worker_tools.clear();
    } else if let Some(ref explicit) = spec.allowed_tools {
        // Leader explicitly specified a tool allow-list for this subtask.
        let allow: HashSet<&str> = explicit.iter().map(String::as_str).collect();
        worker_tools.retain(|t| allow.contains(t.name.as_str()));
    } else if let Some(role_default) = default_allowed_tools_for_role(role_profile.role_name) {
        // No explicit list — apply the role-appropriate default.
        // Reviewers and analysts get read-only; implementers/specialists keep everything.
        let allow: HashSet<&str> = role_default.iter().map(String::as_str).collect();
        worker_tools.retain(|t| allow.contains(t.name.as_str()));
    }
    // None from default_allowed_tools_for_role → unrestricted (implementer / deep-specialist).
    let worker_role_prompt = format!(
        "## Worker role\nRole: {}\nFocus: {}\nDeliverable: {}\n\n\
         Grounded tool policy:\n\
         - Use tools only when needed to verify facts from the workspace or machine.\n\
         - Do not guess file paths; discover with list tools first when uncertain.\n\
         - For file reads/writes, stay within the open workspace unless explicitly instructed otherwise.\n\n\
         ## Required self-assessment\n\
         At the very end of your response, append EXACTLY this block — no prose before or after the tags:\n\
         <worker_assessment>\n\
         {{\"confidence\": <0-100>, \"evidence_sources\": [\"specific file:line or observation\"], \"gaps\": [\"what you could not verify\"]}}\n\
         </worker_assessment>\n\n\
         Confidence scale: 90-100 = verified by tool results; 70-89 = strong evidence, minor gaps; \
         50-69 = partial evidence; below 50 = inference only.",
        role_profile.role_name,
        role_profile.focus,
        role_profile.deliverable,
    );

    let worker_sys_prompt = if worker.system_prompt.is_empty() {
        tools::system_prompt(&worker_tools, workspace_path.as_deref())
    } else {
        format!("{}\n\n{}", worker.system_prompt, tools::system_prompt(&worker_tools, workspace_path.as_deref()))
    };
    let worker_sys_prompt = format!("{}\n\n{}", worker_sys_prompt, worker_role_prompt);

    let workspace_grounding = workspace_path
        .as_deref()
        .map(|path| format!("\n\n## Workspace grounding\nOpen workspace folder: {path}\nAlways pass this exact string as the 'path' argument for file/directory tool calls. Do not append any tokens or placeholders to it."))
        .unwrap_or_default();

    let ctx_user_base = if settings.include_original_prompt_in_worker_context {
        format!(
            "## Original user request\n{}\n\n## Task\n{}\n\n## Context\n{}{}",
            original_prompt, spec.task, spec.context, workspace_grounding
        )
    } else {
        format!("## Task\n{}\n\n## Context\n{}{}", spec.task, spec.context, workspace_grounding)
    };

    let ctx_user = if prior_results_context.trim().is_empty() {
        ctx_user_base
    } else {
        format!("{}\n\n## Prior worker results allowed by policy\n{}", ctx_user_base, prior_results_context)
    };

    let worker_ctx = vec![
        json!({"role": "system", "content": worker_sys_prompt}),
        json!({"role": "user", "content": ctx_user}),
    ];

    let agent_id = worker.config.id.clone();
    let slot = worker.config.slot_index;
    let model_id = worker.effective_model_id.clone();
    let task_clone = spec.task.clone();

    let _ = app_handle.emit("agent-thinking-start", json!({
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
                orchestrator,
                app_handle,
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

    let max_retries = if settings.retry_failed_workers { 2usize } else { 0usize };
    let mut last_error: Option<String> = None;

    for attempt in 0..=max_retries {
        let result = run_once().await;
        match result {
            Ok((raw, stats)) => {
                // Execute any tool calls the worker generated (mini ReAct loop, max 3 rounds).
                let raw_clone = raw.clone();
                let final_raw = if settings.allow_worker_tools && !worker_tools.is_empty() {
                    execute_worker_tool_calls(
                        app_handle, orchestrator, run_id, settings, &worker_tools,
                        workspace_path.as_deref(), &agent_id, slot, &model_id,
                        worker_ctx.clone(), raw, Some(worker_cancel.clone()),
                    ).await.unwrap_or_else(|e| {
                        if settings.emit_debug_events {
                            let _ = app_handle.emit("swarm-debug", json!({
                                "run_id": run_id, "phase": "worker.tool_exec.error",
                                "agent_id": &agent_id, "slot": slot, "error": e,
                            }));
                        }
                        String::new()
                    })
                } else {
                    strip_think_tags(&raw_clone)
                };
                let mut text = tools::strip_tool_calls(&final_raw);
                if text.is_empty() {
                    text = tools::strip_tool_calls(&strip_think_tags(&raw_clone));
                }
                // Extract structured self-assessment before truncation.
                let assessment = parse_worker_assessment(&text);
                if assessment.is_some() {
                    text = strip_worker_assessment(&text);
                }
                if text.len() > settings.max_worker_response_chars {
                    text = truncate_at_semantic_boundary(&text, settings.max_worker_response_chars);
                }
                let _ = app_handle.emit("swarm-agent-complete", json!({
                    "run_id": run_id,
                    "agent_id": &agent_id,
                    "slot": slot,
                    "result": &text,
                    "confidence": assessment.as_ref().map(|a| a.confidence),
                    "stats": &stats,
                    "attempt": attempt + 1,
                }));
                return AgentOutput { agent_id, slot_index: slot, subtask: task_clone, result: text, stats, assessment };
            }
            Err(e) => {
                last_error = Some(e.clone());

                let cancelled = worker_cancel.load(Ordering::Relaxed);
                let has_retry_left = attempt < max_retries;
                if settings.emit_debug_events {
                    let _ = app_handle.emit("swarm-debug", json!({
                        "run_id": run_id,
                        "phase": "worker.retry.failed_attempt",
                        "agent_id": &agent_id,
                        "slot": slot,
                        "attempt": attempt + 1,
                        "max_attempts": max_retries + 1,
                        "error": &e,
                        "cancelled": cancelled,
                        "has_retry_left": has_retry_left,
                    }));
                }

                if cancelled || !has_retry_left {
                    break;
                }

                let reset_ok = reset_worker_runtime(
                    app_handle,
                    orchestrator,
                    run_id,
                    settings,
                    worker,
                    attempt + 1,
                ).await;

                if settings.emit_debug_events {
                    let _ = app_handle.emit("swarm-debug", json!({
                        "run_id": run_id,
                        "phase": "worker.retry.reset_result",
                        "agent_id": &agent_id,
                        "slot": slot,
                        "attempt": attempt + 1,
                        "reset_ok": reset_ok,
                    }));
                }

                let backoff_ms = 350u64.saturating_mul((attempt + 1) as u64);
                tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
            }
        }
    }

    let final_err = last_error.unwrap_or_else(|| "unknown worker failure".to_string());
    let _ = app_handle.emit("swarm-error", json!({
        "run_id": run_id,
        "agent_id": &agent_id,
        "slot": slot,
        "error": &final_err,
    }));
    AgentOutput {
        agent_id,
        slot_index: slot,
        subtask: task_clone,
        result: format!("Error: {}", final_err),
        stats: InferStats::default(),
        assessment: None,
    }
}

async fn reset_worker_runtime(
    app_handle: &AppHandle,
    orchestrator: &ModelOrchestrator,
    run_id: &str,
    settings: &SwarmRuntimeSettings,
    worker: &ResolvedAgent,
    attempt: usize,
) -> bool {
    let Some(model_id) = worker.effective_model_id.clone() else {
        return false;
    };

    let wait_ms = settings.worker_timeout_ms.clamp(30_000, 180_000) / 2;
    let rx = orchestrator.load(model_id.clone());
    match tokio::time::timeout(std::time::Duration::from_millis(wait_ms), rx).await {
        Ok(Ok(Ok(()))) => true,
        Ok(Ok(Err(err))) => {
            if settings.emit_debug_events {
                let _ = app_handle.emit("swarm-debug", json!({
                    "run_id": run_id,
                    "phase": "worker.retry.reset_failed",
                    "slot": worker.config.slot_index,
                    "agent_id": worker.config.id,
                    "attempt": attempt,
                    "model_id": model_id,
                    "error": err,
                }));
            }
            false
        }
        Ok(Err(_)) | Err(_) => {
            if settings.emit_debug_events {
                let _ = app_handle.emit("swarm-debug", json!({
                    "run_id": run_id,
                    "phase": "worker.retry.reset_timeout",
                    "slot": worker.config.slot_index,
                    "agent_id": worker.config.id,
                    "attempt": attempt,
                    "model_id": model_id,
                    "wait_ms": wait_ms,
                }));
            }
            false
        }
    }
}

fn policy_for_slot(settings: &SwarmRuntimeSettings, slot: usize) -> AgentChainPolicy {
    settings
        .agent_chain_policies
        .iter()
        .find(|p| p.slot_index == slot)
        .cloned()
        .unwrap_or_else(|| AgentChainPolicy {
            slot_index: slot,
            execution_tier: slot,
            always_run: slot == 0,
            can_be_early_exit_gate: slot == 1,
            early_exit_confidence_threshold: settings.satisfaction_threshold,
            response_weight: if slot == 0 { 1 } else { 2 },
            can_review_from_slots: if slot <= 1 { vec![0] } else { vec![0, 1] },
            can_delegate_to_slots: vec![],
            allow_heavy_work: slot != 0,
        })
}

fn score_agent_output(out: &AgentOutput, policy: &AgentChainPolicy) -> f64 {
    let text = out.result.trim();
    if text.is_empty() || text.starts_with("Error:") {
        return 0.0;
    }

    let base = if let Some(ref a) = out.assessment {
        // Structured self-assessment path: use worker-reported confidence with
        // an evidence bonus and a gap penalty.
        let evidence_bonus = (a.evidence_sources.len().min(5) as f64) * 4.0;
        let gap_penalty    = (a.gaps.len().min(3) as f64) * 3.0;
        (a.confidence + evidence_bonus - gap_penalty).clamp(0.0, 100.0)
    } else {
        // Fallback heuristics when worker did not emit an assessment block.
        // Still less reliable than self-assessment, but avoids total score collapse.
        let explicit_conf  = extract_confidence_percent(text).unwrap_or(0.0);
        let length_bonus   = (text.len().min(4000) as f64 / 4000.0) * 30.0;
        let clarity_bonus  = if text.contains("```") || text.contains("1.") { 12.0 } else { 4.0 };
        let explicit_bonus = if explicit_conf > 0.0 { explicit_conf * 0.55 } else { 0.0 };
        (38.0 + explicit_bonus + length_bonus + clarity_bonus).min(100.0)
    };

    let weight = policy.response_weight.max(1) as f64;
    (base * (0.85 + (weight.min(10.0) / 25.0))).min(100.0)
}

fn extract_confidence_percent(text: &str) -> Option<f64> {
    for line in text.lines().take(12) {
        let ll = line.to_lowercase();
        if ll.contains("confidence") {
            let digits = ll
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '.')
                .collect::<String>();
            if let Ok(v) = digits.parse::<f64>() {
                return Some(v.min(100.0));
            }
        }
    }
    None
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
        source: "workspace",
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

// ── Worker mini-ReAct loop ────────────────────────────────────────────────────

/// Returns true for tools whose only side-effect is reading — safe to execute in parallel.
fn is_read_only_tool(name: &str) -> bool {
    matches!(name, "read_file" | "list_files" | "list_all_files" | "search_files" | "grep_files")
}

/// Execute tool calls generated by a worker, then do one follow-up inference with the
/// results so the worker returns actual content rather than empty text.
///
/// Improvements over the original:
/// - Max rounds raised from 3 → 8, allowing workers to do deeper file investigation.
/// - Within each round, read-only tools (read_file, list_files, search_files, grep_files)
///   run in parallel via join_all; mutating/external tools run serially after.
/// - Results are collected in original call order regardless of parallel/serial split.
#[allow(clippy::too_many_arguments)]
async fn execute_worker_tool_calls(
    app_handle:      &AppHandle,
    orchestrator:    &ModelOrchestrator,
    run_id:          &str,
    settings:        &SwarmRuntimeSettings,
    tools_available: &[tools::ToolDef],
    workspace_path:  Option<&str>,
    agent_id:        &str,
    slot:            i64,
    model_id:        &Option<String>,
    mut ctx:         Vec<Value>,
    initial_raw:     String,
    cancel_flag:     Option<Arc<AtomicBool>>,
) -> Result<String, String> {
    let mut current_raw = initial_raw;
    let max_rounds = 8usize;
    let mut discovered_paths: HashSet<String> = HashSet::new();

    for round in 0..max_rounds {
        let parsed = tools::parse_tool_calls(&current_raw);
        if parsed.calls.is_empty() {
            return Ok(strip_think_tags(&current_raw));
        }

        // Append the assistant turn (containing tool calls) to context.
        ctx.push(json!({ "role": "assistant", "content": current_raw }));

        // ── Phase 1: validate all calls, bucket into parallel (read-only) and serial ──
        // Slot N = result for calls[N]; pre-fill with None.
        let mut slot_results: Vec<Option<String>> = vec![None; parsed.calls.len()];
        let mut parallel_batch: Vec<(usize, String, Value)> = Vec::new(); // (index, tool, args)
        let mut serial_batch:   Vec<(usize, &tools::ToolCall)> = Vec::new();

        for (idx, call) in parsed.calls.iter().enumerate() {
            if cancel_flag.as_ref().map(|f| f.load(Ordering::Relaxed)).unwrap_or(false) {
                break;
            }
            match validate_worker_tool_call(call, tools_available, workspace_path, &ctx, &discovered_paths) {
                Err(reason) => {
                    if settings.emit_debug_events {
                        let _ = app_handle.emit("swarm-debug", json!({
                            "run_id": run_id, "phase": "worker.tool_validation.blocked",
                            "agent_id": agent_id, "slot": slot,
                            "tool": &call.tool, "reason": &reason,
                        }));
                    }
                    slot_results[idx] = Some(format!("Tool `{}` blocked: {}", call.tool, reason));
                }
                Ok(()) => {
                    if is_read_only_tool(&call.tool) {
                        parallel_batch.push((idx, call.tool.clone(), call.args.clone()));
                    } else {
                        serial_batch.push((idx, call));
                    }
                }
            }
        }

        // ── Phase 2: execute read-only tools in parallel ─────────────────────────────
        if !parallel_batch.is_empty() {
            if settings.emit_debug_events {
                let _ = app_handle.emit("swarm-debug", json!({
                    "run_id": run_id, "phase": "worker.tools_parallel",
                    "agent_id": agent_id, "slot": slot, "round": round,
                    "count": parallel_batch.len(),
                    "tools": parallel_batch.iter().map(|(_, t, _)| t.as_str()).collect::<Vec<_>>(),
                }));
            }

            let read_futures = parallel_batch.iter().map(|(idx, tool, args)| {
                let tool  = tool.clone();
                let args  = args.clone();
                async move {
                    let result = tokio::time::timeout(
                        std::time::Duration::from_secs(30),
                        tools::execute_built_in(&tool, &args, workspace_path),
                    ).await
                    .unwrap_or_else(|_| Err("Tool timed out".into()))
                    .unwrap_or_else(|e| format!("Tool error: {e}"));
                    (*idx, tool, args, result)
                }
            });

            let par_results = futures::future::join_all(read_futures).await;
            for (idx, tool, args, result) in par_results {
                update_discovered_paths_from_tool_result(&tool, &args, workspace_path, &result, &mut discovered_paths);
                if settings.emit_debug_events {
                    let _ = app_handle.emit("swarm-debug", json!({
                        "run_id": run_id, "phase": "worker.tool_executed",
                        "agent_id": agent_id, "slot": slot, "tool": &tool, "parallel": true,
                    }));
                }
                slot_results[idx] = Some(format!("Tool `{tool}` result:\n{result}"));
            }
        }

        // ── Phase 3: execute mutating/external tools serially ────────────────────────
        for (idx, call) in &serial_batch {
            if cancel_flag.as_ref().map(|f| f.load(Ordering::Relaxed)).unwrap_or(false) {
                break;
            }
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                tools::execute_built_in(&call.tool, &call.args, workspace_path),
            ).await
            .unwrap_or_else(|_| Err("Tool timed out".into()))
            .unwrap_or_else(|e| format!("Tool error: {e}"));

            update_discovered_paths_from_tool_result(&call.tool, &call.args, workspace_path, &result, &mut discovered_paths);
            if settings.emit_debug_events {
                let _ = app_handle.emit("swarm-debug", json!({
                    "run_id": run_id, "phase": "worker.tool_executed",
                    "agent_id": agent_id, "slot": slot, "tool": &call.tool, "parallel": false,
                }));
            }
            slot_results[*idx] = Some(format!("Tool `{}` result:\n{result}", call.tool));
        }

        // ── Collect results in original call order ────────────────────────────────────
        let tool_results: Vec<String> = slot_results.into_iter().flatten().collect();
        if tool_results.is_empty() {
            return Ok(strip_think_tags(&current_raw));
        }

        // Feed all results back in one user turn, ask for next step / final answer.
        let results_text = tool_results.join("\n\n");
        let prompt_suffix = if round + 1 < max_rounds {
            "Continue your investigation or provide your final answer if you have enough information."
        } else {
            "You have reached the tool-use limit. Provide your final answer now based on the results above."
        };
        ctx.push(json!({
            "role": "user",
            "content": format!("{results_text}\n\n{prompt_suffix}")
        }));

        // Inference pass — the worker reasons about results and either calls more tools or answers.
        let followup = run_agent_inference(
            orchestrator, app_handle, ctx.clone(),
            agent_id.to_string(), slot, model_id.clone(),
            cancel_flag.clone(), settings.stream_worker_tokens,
        ).await.map(|(r, _)| r)?;

        current_raw = followup;
    }

    Ok(strip_think_tags(&current_raw))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn enabled_worker_slots(agents: &[ResolvedAgent]) -> Vec<usize> {
    agents
        .iter()
        .filter(|a| a.config.enabled && a.config.slot_index != 0)
        .map(|a| a.config.slot_index as usize)
        .collect::<Vec<_>>()
}

fn normalize_subtasks_for_active_workers(
    mut subtasks: Vec<SubtaskSpec>,
    enabled_worker_slots: &[usize],
    user_prompt: &str,
    settings: &SwarmRuntimeSettings,
) -> Vec<SubtaskSpec> {
    let mut seen_slots = std::collections::HashSet::new();
    subtasks.retain(|spec| {
        if !enabled_worker_slots.contains(&spec.worker_slot) {
            return false;
        }
        seen_slots.insert(spec.worker_slot)
    });

    for slot in enabled_worker_slots {
        if !seen_slots.contains(slot) {
            let profile = role_profile_for_slot(settings, *slot);
            subtasks.push(SubtaskSpec {
                worker_slot: *slot,
                task: format!("As the {}, address the following request: {}", profile.role_name, user_prompt),
                context: format!(
                    "Your role: {}. Your focus: {}. Required deliverable: {}.\n\
                     Auto-assigned because this worker slot was not included in the leader plan.\n\
                     Only use workspace tools when the request requires inspecting files or machine state.",
                    profile.role_name,
                    profile.focus,
                    profile.deliverable,
                ),
                allowed_tools: None,
            });
        }
    }

    subtasks
}

struct WorkerRoleProfile {
    role_name: &'static str,
    focus: &'static str,
    deliverable: &'static str,
}

fn role_profile_for_slot(settings: &SwarmRuntimeSettings, slot: usize) -> WorkerRoleProfile {
    let policy = policy_for_slot(settings, slot);
    if slot == settings.preferred_primary_slot {
        return WorkerRoleProfile {
            role_name: "Primary Implementer",
            focus: "produce the main implementation path and concrete code-level plan",
            deliverable: "implementation-ready steps with exact files/symbols",
        };
    }
    if policy.can_be_early_exit_gate {
        return WorkerRoleProfile {
            role_name: "Gatekeeper Reviewer",
            focus: "assess risk and confidence, identify regressions and missing validation",
            deliverable: "go/no-go assessment with specific evidence",
        };
    }
    if policy.allow_heavy_work {
        return WorkerRoleProfile {
            role_name: "Deep Specialist",
            focus: "handle complex reasoning, edge-cases, and deep technical execution",
            deliverable: "deep-dive analysis with concrete fixes/tests",
        };
    }
    WorkerRoleProfile {
        role_name: "Supporting Analyst",
        focus: "add complementary checks, documentation, and verification context",
        deliverable: "targeted findings that improve synthesis quality",
    }
}

/// Returns the default allowed tool names for a role, or None (unrestricted).
/// Reviewers and analysts are read-only by default; implementers and specialists are unrestricted.
fn default_allowed_tools_for_role(role_name: &str) -> Option<Vec<String>> {
    match role_name {
        "Gatekeeper Reviewer" | "Supporting Analyst" => Some(vec![
            "read_file".to_string(),
            "list_files".to_string(),
            "list_all_files".to_string(),
            "search_files".to_string(),
            "grep_files".to_string(),
        ]),
        _ => None,
    }
}

fn resolve_tool_path(raw_path: &str, workspace_path: Option<&str>) -> PathBuf {
    let candidate = Path::new(raw_path);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else if let Some(ws) = workspace_path {
        Path::new(ws).join(candidate)
    } else {
        candidate.to_path_buf()
    }
}

fn normalize_path_for_compare(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_lowercase()
}

fn is_within_workspace(path: &Path, workspace_path: Option<&str>) -> bool {
    let Some(ws) = workspace_path else { return true; };
    let ws_norm = normalize_path_for_compare(Path::new(ws));
    let path_norm = normalize_path_for_compare(path);
    path_norm == ws_norm || path_norm.starts_with(&(ws_norm + "/"))
}

fn extract_path_arg(call: &tools::ToolCall) -> Option<String> {
    call.args
        .get("path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn context_mentions_path(ctx: &[Value], candidate_path: &str) -> bool {
    let needle = candidate_path.to_lowercase();
    ctx.iter().any(|msg| {
        msg.get("content")
            .and_then(|v| v.as_str())
            .map(|content| content.to_lowercase().contains(&needle))
            .unwrap_or(false)
    })
}

fn validate_worker_tool_call(
    call: &tools::ToolCall,
    tools_available: &[tools::ToolDef],
    workspace_path: Option<&str>,
    ctx: &[Value],
    discovered_paths: &HashSet<String>,
) -> Result<(), String> {
    if !tools_available.iter().any(|t| t.name == call.tool) {
        return Err(format!("tool `{}` is not in the allowed worker tool list", call.tool));
    }

    let path_tool = matches!(
        call.tool.as_str(),
        "read_file" | "list_files" | "list_all_files" | "search_files" | "grep_files" | "write_file" | "edit_file" | "delete_file" | "create_dir"
    );

    if !path_tool {
        return Ok(());
    }

    let Some(raw_path) = extract_path_arg(call) else {
        if matches!(call.tool.as_str(), "list_all_files") {
            return Ok(());
        }
        return Err("missing required path argument".to_string());
    };

    let resolved = resolve_tool_path(&raw_path, workspace_path);
    if !is_within_workspace(&resolved, workspace_path) {
        return Err(format!(
            "path `{}` resolves outside workspace boundary",
            resolved.to_string_lossy()
        ));
    }

    if call.tool == "read_file" {
        let normalized = normalize_path_for_compare(&resolved);
        let discovered = discovered_paths.contains(&normalized);
        let mentioned = context_mentions_path(ctx, &normalized) || context_mentions_path(ctx, &raw_path);
        if !discovered && !mentioned {
            return Err("read_file path not yet grounded; list files first or reference an existing known path".to_string());
        }
    }

    Ok(())
}

fn update_discovered_paths_from_tool_result(
    tool_name: &str,
    args: &Value,
    workspace_path: Option<&str>,
    result: &str,
    discovered_paths: &mut HashSet<String>,
) {
    if tool_name == "read_file" {
        if let Some(raw_path) = args.get("path").and_then(|v| v.as_str()) {
            let resolved = resolve_tool_path(raw_path, workspace_path);
            discovered_paths.insert(normalize_path_for_compare(&resolved));
        }
        return;
    }

    if tool_name != "list_files" && tool_name != "list_all_files" {
        return;
    }

    let parsed: Result<Value, _> = serde_json::from_str(result);
    let Ok(payload) = parsed else { return; };

    let root = payload
        .get("path")
        .and_then(|v| v.as_str())
        .map(|p| resolve_tool_path(p, workspace_path));

    if let Some(root_path) = root {
        discovered_paths.insert(normalize_path_for_compare(&root_path));
        if let Some(entries) = payload.get("entries").and_then(|v| v.as_array()) {
            for entry in entries {
                if let Some(rel) = entry.get("path").and_then(|v| v.as_str()) {
                    let full = root_path.join(rel);
                    discovered_paths.insert(normalize_path_for_compare(&full));
                }
            }
        }
    }
}

/// Parse the <worker_assessment> JSON block from the end of a worker response.
/// Uses rfind so incidental mentions of the tag earlier in the text are ignored.
fn parse_worker_assessment(text: &str) -> Option<WorkerAssessment> {
    let open  = "<worker_assessment>";
    let close = "</worker_assessment>";
    let start = text.rfind(open)?;
    let end   = text[start..].find(close).map(|p| start + p)?;
    let json  = text[start + open.len()..end].trim();
    serde_json::from_str(json).ok()
}

/// Remove the <worker_assessment> block from worker output so it is not shown to users.
fn strip_worker_assessment(text: &str) -> String {
    let open  = "<worker_assessment>";
    let close = "</worker_assessment>";
    if let Some(start) = text.rfind(open) {
        if let Some(rel) = text[start..].find(close) {
            let end = start + rel + close.len();
            return format!("{}{}", text[..start].trim_end(), &text[end..]);
        }
    }
    text.to_string()
}

/// Truncate worker output at a paragraph or sentence boundary rather than mid-character,
/// and append a notice so synthesis knows the output was cut.
fn truncate_at_semantic_boundary(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars { return text.to_string(); }
    let candidate = &text[..max_chars];
    if let Some(pos) = candidate.rfind("\n\n") {
        return format!("{}\n\n[... response truncated at {max_chars} chars]", &text[..pos]);
    }
    if let Some(pos) = candidate.rfind(". ") {
        return format!("{}.\n\n[... response truncated]", &text[..pos]);
    }
    format!("{}\n\n[... response truncated]", candidate)
}

/// Lightweight system prompt for the synthesis pass — no tool definitions, no workspace tree.
/// The leader doesn't call tools during synthesis; injecting the full tool registry wastes
/// ~3 KB of context and creates confusing "should I call a tool?" noise.
fn synthesis_system_prompt(workspace_path: Option<&str>) -> String {
    let ws_line = workspace_path
        .map(|p| format!("\nWorkspace: {p}"))
        .unwrap_or_default();
    format!(
        "You are the synthesis leader in a multi-agent swarm.{ws_line}\n\
         Your task: reconcile the worker findings below into one authoritative, coherent answer.\n\n\
         Rules:\n\
         - Weigh evidence-backed claims (with file/line citations or tool-verified facts) over unsupported assertions.\n\
         - Workers who reported higher confidence and specific evidence sources carry more weight.\n\
         - Explicitly resolve contradictions — do not silently pick one side.\n\
         - Workers excluded due to errors are noted; do not speculate about what they would have said.\n\
         - Do not call any tools. Synthesize only from the provided worker results.\n\
         - End with a concrete final answer and actionable next steps."
    )
}

/// Strip trailing commas before `}` or `]` — the most common JSON mistake from LLMs.
fn clean_plan_json(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ',' {
            // Peek past whitespace — if next non-whitespace is `}` or `]`, drop the comma.
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() { j += 1; }
            if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                i += 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Parse the `<swarm_plan>` tag from the leader response.
/// Uses `rfind` so any explanatory prose the leader writes *before* the tag is ignored.
/// Returns `Err(reason)` with a human-readable description so the caller can attempt repair.
fn parse_swarm_plan(text: &str) -> Result<LeaderPlan, String> {
    let open  = "<swarm_plan>";
    let close = "</swarm_plan>";

    // rfind: pick the *last* opening tag so inline examples earlier in the text are skipped.
    let start = text.rfind(open)
        .ok_or_else(|| "no <swarm_plan> tag in leader response".to_string())?;
    let end = text[start..].find(close)
        .map(|p| start + p)
        .ok_or_else(|| "<swarm_plan> tag was opened but never closed".to_string())?;

    let json_str = text[start + open.len()..end].trim();
    if json_str.is_empty() {
        return Err("plan JSON between tags is empty".to_string());
    }

    let cleaned = clean_plan_json(json_str);
    let plan: LeaderPlan = serde_json::from_str(&cleaned)
        .map_err(|e| format!("plan JSON parse error: {e}"))?;

    if plan.subtasks.is_empty() {
        return Err("plan contains no subtasks".to_string());
    }
    if plan.subtasks.iter().any(|s| s.task.trim().is_empty()) {
        return Err("one or more subtasks have an empty task field".to_string());
    }

    Ok(plan)
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
