//! Bridges real Custom Swarm runs (`swarm_orchestrator.rs`, which dispatches
//! actual LLM inference) into the Swarm Commander system (`crates/swarm` +
//! `swarm_commands.rs`), whose own execution engine is a stub — its
//! `seed_plan()` hardcodes 3 fake tasks regardless of the real goal, and
//! nothing ever advances them, since no real executor calls
//! `SwarmCommand::TaskCompleted`. Rather than building a second, competing
//! LLM-dispatch engine, this module makes the Commander's hierarchy/ledger/
//! DAG views a real-time mirror of the swarm that's actually running,
//! recorded under the exact same `run_id`/`swarm_id` (both are the same UUID
//! string — see `commands.rs`'s `submit_swarm_chat`).
//!
//! Every function here is called from an existing, already-real emit() site
//! in `swarm_orchestrator.rs` and is purely additive/observational — none of
//! it can affect the actual swarm run's control flow, since it only ever
//! writes to the bridged `SwarmOrchestrator`'s own hierarchy/ledger/dag,
//! never reads back from them to make a real decision.

use tauri::{AppHandle, Manager};
use uuid::Uuid;

use swarm::dag::{TaskNode, TaskStatus};
use swarm::hierarchy::{HierarchyNode, NodeStatus};
use swarm::ledger::LedgerEventKind;
use swarm::orchestrator::{SwarmOrchestrator, SwarmSpec, SwarmStatus};
use swarm::role::SwarmRole;

use crate::agent_store::ResolvedAgent;
use crate::swarm_commands::SwarmState;

/// Deterministic per-slot task id — stable across every bridge call site for
/// the same run without needing to thread shared mutable state through
/// `swarm_orchestrator.rs`'s dispatch/retry/delegation internals.
fn slot_task_id(run_id: &str, slot: i64) -> Uuid {
    Uuid::new_v5(&Uuid::NAMESPACE_OID, format!("{run_id}:task:{slot}").as_bytes())
}

fn parse_agent_uuid(agent_id: &str, run_id: &str, slot: i64) -> Uuid {
    // Agent ids are frontend-generated `crypto.randomUUID()` strings
    // (AgentConfig.id) and should always parse — the slot-based fallback
    // only matters for hand-edited/legacy data that isn't a real UUID.
    Uuid::parse_str(agent_id).unwrap_or_else(|_| slot_task_id(run_id, slot))
}

fn swarm_state(app_handle: &AppHandle) -> Option<tauri::State<'_, SwarmState>> {
    app_handle.try_state::<SwarmState>()
}

/// Mirrors a swarm lifecycle event into the OmniHarness kernel's hash-chained
/// event store (when reachable — see kernel_bridge.rs), under module_source
/// "workspace-swarm". This is the same event stream the Python orchestrator's
/// substrate/swarm.py appends its own runs under ("orchestrator-swarm"), so a
/// kernel client (Clojure orchestrator, or a future dashboard) can observe
/// swarm activity from either engine without either one depending on the
/// other's internals — purely observational, like the rest of this module.
fn mirror_to_kernel(app_handle: &AppHandle, run_id: &str, event_type: &str, payload_json: String) {
    let Some(app_state) = app_handle.try_state::<crate::AppState>() else { return };
    let _ = app_state.kernel_bridge.mirror_sender().send(crate::kernel_bridge::KernelMirrorEvent {
        module_source: "workspace-swarm".to_string(),
        event_type: event_type.to_string(),
        payload_json,
        session_id: run_id.to_string(),
    });
}

/// Called once at the very start of a real swarm run (same touchpoint as the
/// `swarm-started` event). Creates the bridged orchestrator record plus one
/// real hierarchy node per configured agent (leader included).
pub async fn on_swarm_started(
    app_handle: &AppHandle,
    run_id: &str,
    user_prompt: &str,
    workspace_path: Option<&str>,
    enabled_tools: &[String],
    agents: &[ResolvedAgent],
) {
    let Some(state) = swarm_state(app_handle) else { return };
    let Ok(run_uuid) = Uuid::parse_str(run_id) else { return };

    let spec = SwarmSpec {
        name: format!("Custom Swarm — {}", &run_id[..8.min(run_id.len())]),
        goal: user_prompt.to_string(),
        max_workers: agents.len() as u32,
        allowed_tools: enabled_tools.to_vec(),
        timeout_secs: None,
        workspace_path: workspace_path.map(|s| s.to_string()),
    };

    let orch = SwarmOrchestrator::new_bridged(run_uuid, spec, state.registry.capability_registry().clone());
    orch.set_status(SwarmStatus::Running).await;
    orch.mark_started().await;
    orch.ledger.append(LedgerEventKind::SwarmCreated, None).await;

    for agent in agents {
        if !agent.config.enabled {
            continue;
        }
        let role = if agent.config.slot_index == 0 { SwarmRole::ProjectManager } else { SwarmRole::Agent };
        let node_id = parse_agent_uuid(&agent.config.id, run_id, agent.config.slot_index);
        let node = HierarchyNode {
            id: node_id,
            swarm_id: run_uuid,
            parent_id: if agent.config.slot_index == 0 { None } else {
                Some(parse_agent_uuid(&agents.iter().find(|a| a.config.slot_index == 0).map(|a| a.config.id.clone()).unwrap_or_default(), run_id, 0))
            },
            role: role.clone(),
            display_name: agent.config.label.clone(),
            domain: agent.persona.as_ref().map(|p| p.name.clone()).unwrap_or_else(|| "general".into()),
            status: NodeStatus::Idle,
            current_task: None,
            progress: 0.0,
            cpu_load: 0.0,
            ram_mb: agent.ram_required_mb as f64,
            credits_used: 0.0,
            tasks_completed: 0,
            tasks_failed: 0,
            is_remote: false,
            device_label: None,
            spawned_at: chrono::Utc::now(),
            last_heartbeat: chrono::Utc::now(),
        };
        orch.hierarchy.insert(node).await;
        orch.ledger.append(LedgerEventKind::AgentSpawned { role: format!("{role:?}") }, None).await;
    }

    state.registry.register_bridged(orch).await;

    mirror_to_kernel(app_handle, run_id, "swarm:started", serde_json::json!({
        "goal": user_prompt, "agent_count": agents.len(),
    }).to_string());
}

/// Called once the leader's real plan is ready (same touchpoint as
/// `swarm-plan-ready`). Populates the DAG with the real planned subtasks
/// (not the stub `seed_plan()` text) and marks each assigned worker Working.
pub async fn on_plan_ready(app_handle: &AppHandle, run_id: &str, subtasks: &[(i64, String)]) {
    let Some(state) = swarm_state(app_handle) else { return };
    let Ok(run_uuid) = Uuid::parse_str(run_id) else { return };
    let Some(orch) = state.registry.get(run_uuid).await else { return };

    {
        let mut dag = orch.dag.write().await;
        for (slot, task_desc) in subtasks {
            let task = TaskNode {
                id: slot_task_id(run_id, *slot),
                swarm_id: run_uuid,
                description: task_desc.clone(),
                context: String::new(),
                status: TaskStatus::Running { assigned_to: slot_task_id(run_id, *slot) },
                depends_on: vec![],
                required_capabilities: vec![],
                estimated_minutes: 0.0,
                result: None,
                created_at: chrono::Utc::now(),
                started_at: Some(chrono::Utc::now()),
                completed_at: None,
            };
            dag.insert(task);
        }
    }
    orch.ledger.append(LedgerEventKind::PlanCreated { task_count: subtasks.len() }, None).await;

    for (slot, task_desc) in subtasks {
        let node_id = slot_task_id_for_agent(run_id, *slot);
        orch.hierarchy.assign_task(node_id, task_desc.clone()).await;
    }

    mirror_to_kernel(app_handle, run_id, "swarm:plan_ready", serde_json::json!({
        "task_count": subtasks.len(),
    }).to_string());
}

/// Slot->node lookup used once the plan is known — mirrors the same
/// deterministic derivation used when the node's real agent-id couldn't be
/// resolved directly (kept as a separate name for readability at call sites).
fn slot_task_id_for_agent(run_id: &str, slot: i64) -> Uuid {
    slot_task_id(run_id, slot)
}

/// Called when a real worker finishes — success or failure (same touchpoints
/// as `swarm-agent-complete`/`swarm-error`).
pub async fn on_worker_complete(
    app_handle: &AppHandle,
    run_id: &str,
    agent_id: &str,
    slot: i64,
    success: bool,
    detail: &str,
) {
    let Some(state) = swarm_state(app_handle) else { return };
    let Ok(run_uuid) = Uuid::parse_str(run_id) else { return };
    let Some(orch) = state.registry.get(run_uuid).await else { return };

    let node_id = parse_agent_uuid(agent_id, run_id, slot);
    let task_id = slot_task_id(run_id, slot);

    {
        let mut dag = orch.dag.write().await;
        if let Some(task) = dag.nodes.get_mut(&task_id) {
            task.status = if success { TaskStatus::Completed } else { TaskStatus::Failed { reason: detail.to_string() } };
            task.result = Some(detail.to_string());
            task.completed_at = Some(chrono::Utc::now());
        }
    }

    orch.hierarchy.record_task_result(node_id, success).await;
    if !success {
        orch.hierarchy.set_status(node_id, NodeStatus::Error { reason: detail.to_string() }).await;
    }

    let event = if success {
        LedgerEventKind::TaskCompleted { agent_id: node_id }
    } else {
        LedgerEventKind::TaskFailed { agent_id: node_id, reason: detail.to_string() }
    };
    orch.ledger.append(event, Some(task_id)).await;

    mirror_to_kernel(app_handle, run_id, "swarm:worker_complete", serde_json::json!({
        "agent_id": agent_id, "slot": slot, "success": success,
    }).to_string());
}

/// Called once the whole run finishes, successfully or not (same touchpoint
/// as `swarm-complete`/the error path in `commands.rs::submit_swarm_chat`).
pub async fn on_swarm_finished(app_handle: &AppHandle, run_id: &str, success: bool, summary: &str) {
    let Some(state) = swarm_state(app_handle) else { return };
    let Ok(run_uuid) = Uuid::parse_str(run_id) else { return };
    let Some(orch) = state.registry.get(run_uuid).await else { return };

    orch.set_status(if success { SwarmStatus::Completed } else { SwarmStatus::Failed { reason: summary.to_string() } }).await;
    orch.mark_completed(summary).await;
    orch.ledger.append(
        if success { LedgerEventKind::SwarmCompleted } else { LedgerEventKind::SwarmFailed },
        None,
    ).await;

    mirror_to_kernel(app_handle, run_id, "swarm:finished", serde_json::json!({
        "success": success, "summary": summary,
    }).to_string());
}
