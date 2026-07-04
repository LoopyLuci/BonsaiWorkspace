//! Tauri command layer for the next-generation Bonsai Swarm system.
//!
//! Exposes the full `bonsai-swarm` API to the frontend as typed IPC commands.
//! All heavy state lives in `SwarmState`.

use swarm::{
    assistant::AssistantSuggestion,
    hierarchy::{HierarchyNode, HierarchyStats, NodeStatus},
    ledger::LedgerEntry,
    orchestrator::{SwarmCommand, SwarmRegistry, SwarmSnapshot, SwarmSpec},
    registry::{AgentMatch, CapabilityQuery},
    role::AgentProfile,
    role::{Capability, SwarmRole},
};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

// ── App state ─────────────────────────────────────────────────────────────────

pub struct SwarmState {
    pub registry: SwarmRegistry,
}

impl SwarmState {
    pub fn new() -> Self {
        Self { registry: SwarmRegistry::new() }
    }
}

// ── Request / Response DTOs ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateSwarmRequest {
    pub name: String,
    pub goal: String,
    pub max_workers: u32,
    pub allowed_tools: Vec<String>,
    pub timeout_secs: Option<u64>,
    pub workspace_path: Option<String>,
}

#[derive(Serialize)]
pub struct CreateSwarmResponse {
    pub swarm_id: String,
}

#[derive(Deserialize)]
pub struct SpawnAgentRequest {
    pub swarm_id: String,
    pub parent_id: Option<String>,
    pub role: String,
    pub display_name: String,
    pub domain: String,
}

#[derive(Serialize)]
pub struct SpawnAgentResponse {
    pub node_id: String,
}

#[derive(Deserialize)]
pub struct RegisterCapabilityRequest {
    pub agent_id: String,
    pub role: String,
    pub display_name: String,
    pub capabilities: Vec<CapabilityDto>,
    pub cost_per_minute: f64,
    pub reliability: f64,
    pub is_remote: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CapabilityDto {
    pub kind: String,
    pub name: String,
}

impl From<CapabilityDto> for Capability {
    fn from(d: CapabilityDto) -> Self {
        Capability { kind: d.kind, name: d.name }
    }
}

#[derive(Deserialize)]
pub struct CapabilityQueryRequest {
    pub requires_all: Vec<CapabilityDto>,
    pub requires_any: Vec<CapabilityDto>,
    pub max_load: Option<f64>,
    pub max_cost_per_minute: Option<f64>,
    pub min_reliability: Option<f64>,
    pub local_only: bool,
}

#[derive(Serialize)]
pub struct AgentMatchDto {
    pub agent_id: String,
    pub role: String,
    pub display_name: String,
    pub capabilities: Vec<CapabilityDto>,
    pub cost_per_minute: f64,
    pub reliability: f64,
    pub current_load: f64,
    pub is_remote: bool,
    pub score: f64,
}

impl From<AgentMatch> for AgentMatchDto {
    fn from(m: AgentMatch) -> Self {
        AgentMatchDto {
            agent_id: m.profile.agent_id.to_string(),
            role: m.profile.role.to_string(),
            display_name: m.profile.display_name,
            capabilities: m
                .profile
                .capabilities
                .into_iter()
                .map(|c| CapabilityDto { kind: c.kind, name: c.name })
                .collect(),
            cost_per_minute: m.profile.cost_per_minute,
            reliability: m.profile.reliability,
            current_load: m.profile.current_load,
            is_remote: m.profile.is_remote,
            score: m.score,
        }
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

/// Create and start a new swarm. Returns the swarm_id.
#[tauri::command]
pub async fn create_swarm(
    state: State<'_, SwarmState>,
    request: CreateSwarmRequest,
) -> Result<CreateSwarmResponse, String> {
    let spec = SwarmSpec {
        name: request.name,
        goal: request.goal,
        max_workers: request.max_workers,
        allowed_tools: request.allowed_tools,
        timeout_secs: request.timeout_secs,
        workspace_path: request.workspace_path,
    };
    let id = state.registry.create_swarm(spec).await;
    Ok(CreateSwarmResponse { swarm_id: id.to_string() })
}

/// List all active swarms with their current snapshots.
#[tauri::command]
pub async fn list_swarms(state: State<'_, SwarmState>) -> Result<Vec<SwarmSnapshot>, String> {
    Ok(state.registry.list_snapshots().await)
}

/// Get a single swarm snapshot by id.
#[tauri::command]
pub async fn get_swarm(
    state: State<'_, SwarmState>,
    swarm_id: String,
) -> Result<SwarmSnapshot, String> {
    let id: Uuid = swarm_id.parse().map_err(|_| "invalid swarm_id")?;
    let orch = state.registry.get(id).await.ok_or("swarm not found")?;
    Ok(orch.snapshot().await)
}

/// Pause a running swarm.
#[tauri::command]
pub async fn pause_swarm(state: State<'_, SwarmState>, swarm_id: String) -> Result<(), String> {
    let id: Uuid = swarm_id.parse().map_err(|_| "invalid swarm_id")?;
    state.registry.send_command(id, SwarmCommand::Pause).await;
    Ok(())
}

/// Resume a paused swarm.
#[tauri::command]
pub async fn resume_swarm(state: State<'_, SwarmState>, swarm_id: String) -> Result<(), String> {
    let id: Uuid = swarm_id.parse().map_err(|_| "invalid swarm_id")?;
    state.registry.send_command(id, SwarmCommand::Resume).await;
    Ok(())
}

/// Cancel a swarm immediately.
#[tauri::command]
pub async fn swarm_cancel(state: State<'_, SwarmState>, swarm_id: String) -> Result<(), String> {
    let id: Uuid = swarm_id.parse().map_err(|_| "invalid swarm_id")?;
    state.registry.send_command(id, SwarmCommand::Cancel).await;
    Ok(())
}

/// Get the full agent hierarchy tree for a swarm.
#[tauri::command]
pub async fn get_swarm_hierarchy(
    state: State<'_, SwarmState>,
    swarm_id: String,
) -> Result<Vec<HierarchyNode>, String> {
    let id: Uuid = swarm_id.parse().map_err(|_| "invalid swarm_id")?;
    let orch = state.registry.get(id).await.ok_or("swarm not found")?;
    Ok(orch.hierarchy.snapshot().await)
}

/// Manually spawn an agent node under a swarm.
#[tauri::command]
pub async fn spawn_agent_node(
    state: State<'_, SwarmState>,
    request: SpawnAgentRequest,
) -> Result<SpawnAgentResponse, String> {
    use swarm::hierarchy::HierarchyNode;

    let swarm_id: Uuid = request.swarm_id.parse().map_err(|_| "invalid swarm_id")?;
    let parent_id: Option<Uuid> = request
        .parent_id
        .as_deref()
        .map(|s| s.parse().map_err(|_| "invalid parent_id"))
        .transpose()?;

    let role = match request.role.as_str() {
        "project_manager" => SwarmRole::ProjectManager,
        "manager" => SwarmRole::Manager,
        "sub_manager" => SwarmRole::SubManager,
        "agent" => SwarmRole::Agent,
        "sub_agent" => SwarmRole::SubAgent,
        "assistant" => SwarmRole::Assistant,
        other => return Err(format!("unknown role: {other}")),
    };

    let orch = state.registry.get(swarm_id).await.ok_or("swarm not found")?;
    let node = HierarchyNode::new(swarm_id, parent_id, role, request.display_name, request.domain);
    let node_id = orch.hierarchy.insert(node).await;

    Ok(SpawnAgentResponse { node_id: node_id.to_string() })
}

/// Pause an individual agent node.
#[tauri::command]
pub async fn pause_agent(
    state: State<'_, SwarmState>,
    swarm_id: String,
    node_id: String,
) -> Result<(), String> {
    let swarm_uuid: Uuid = swarm_id.parse().map_err(|_| "invalid swarm_id")?;
    let node_uuid: Uuid = node_id.parse().map_err(|_| "invalid node_id")?;
    let orch = state.registry.get(swarm_uuid).await.ok_or("swarm not found")?;
    orch.hierarchy.set_status(node_uuid, NodeStatus::Paused).await;
    Ok(())
}

/// Resume an individual agent node.
#[tauri::command]
pub async fn resume_agent(
    state: State<'_, SwarmState>,
    swarm_id: String,
    node_id: String,
) -> Result<(), String> {
    let swarm_uuid: Uuid = swarm_id.parse().map_err(|_| "invalid swarm_id")?;
    let node_uuid: Uuid = node_id.parse().map_err(|_| "invalid node_id")?;
    let orch = state.registry.get(swarm_uuid).await.ok_or("swarm not found")?;
    orch.hierarchy.set_status(node_uuid, NodeStatus::Idle).await;
    Ok(())
}

/// Stop an individual agent node permanently.
#[tauri::command]
pub async fn stop_agent(
    state: State<'_, SwarmState>,
    swarm_id: String,
    node_id: String,
) -> Result<(), String> {
    let swarm_uuid: Uuid = swarm_id.parse().map_err(|_| "invalid swarm_id")?;
    let node_uuid: Uuid = node_id.parse().map_err(|_| "invalid node_id")?;
    let orch = state.registry.get(swarm_uuid).await.ok_or("swarm not found")?;
    orch.hierarchy.remove(node_uuid).await;
    Ok(())
}

/// Get recent ledger entries for a swarm (last N events).
#[tauri::command]
pub async fn get_swarm_ledger(
    state: State<'_, SwarmState>,
    swarm_id: String,
    last_n: usize,
) -> Result<Vec<LedgerEntry>, String> {
    let id: Uuid = swarm_id.parse().map_err(|_| "invalid swarm_id")?;
    let orch = state.registry.get(id).await.ok_or("swarm not found")?;
    Ok(orch.ledger.tail(last_n).await)
}

/// Register an agent's capabilities in the global registry.
#[tauri::command]
pub async fn register_agent_capabilities(
    state: State<'_, SwarmState>,
    request: RegisterCapabilityRequest,
) -> Result<(), String> {
    use swarm::role::AgentProfile;

    let agent_id: Uuid = request.agent_id.parse().map_err(|_| "invalid agent_id")?;
    let role = match request.role.as_str() {
        "project_manager" => SwarmRole::ProjectManager,
        "manager" => SwarmRole::Manager,
        "sub_manager" => SwarmRole::SubManager,
        "agent" => SwarmRole::Agent,
        "sub_agent" => SwarmRole::SubAgent,
        "assistant" => SwarmRole::Assistant,
        other => return Err(format!("unknown role: {other}")),
    };

    let profile = AgentProfile {
        agent_id,
        role,
        display_name: request.display_name,
        capabilities: request.capabilities.into_iter().map(Into::into).collect(),
        cost_per_minute: request.cost_per_minute,
        reliability: request.reliability,
        current_load: 0.0,
        is_remote: request.is_remote,
    };
    state.registry.capability_registry().register(profile);
    Ok(())
}

/// Query the capability registry for matching agents.
#[tauri::command]
pub async fn query_agent_capabilities(
    state: State<'_, SwarmState>,
    query: CapabilityQueryRequest,
) -> Result<Vec<AgentMatchDto>, String> {
    let cap_query = CapabilityQuery {
        requires_all: query.requires_all.into_iter().map(Into::into).collect(),
        requires_any: query.requires_any.into_iter().map(Into::into).collect(),
        max_load: query.max_load,
        max_cost_per_minute: query.max_cost_per_minute,
        min_reliability: query.min_reliability,
        local_only: query.local_only,
    };
    let matches = state.registry.capability_registry().find(&cap_query);
    Ok(matches.into_iter().map(AgentMatchDto::from).collect())
}

/// List all registered agent profiles.
#[tauri::command]
pub async fn list_agent_profiles(state: State<'_, SwarmState>) -> Result<Vec<AgentMatchDto>, String> {
    let profiles = state.registry.capability_registry().list_all();
    Ok(profiles
        .into_iter()
        .map(|p| AgentMatchDto {
            agent_id: p.agent_id.to_string(),
            role: p.role.to_string(),
            display_name: p.display_name,
            capabilities: p
                .capabilities
                .into_iter()
                .map(|c| CapabilityDto { kind: c.kind, name: c.name })
                .collect(),
            cost_per_minute: p.cost_per_minute,
            reliability: p.reliability,
            current_load: p.current_load,
            is_remote: p.is_remote,
            score: 0.0,
        })
        .collect())
}

/// Spawn a swarm from a named template, creating a time-travel checkpoint first.
#[tauri::command]
pub async fn spawn_swarm_from_template(
    swarm_state: State<'_, SwarmState>,
    app_state: State<'_, crate::AppState>,
    template_name: String,
    params: serde_json::Value,
) -> Result<String, String> {
    // 1. Emit a checkpoint so the pre-swarm state is captured in the timeline
    app_state.event_bus.publish(crate::system_event_bus::SystemEvent::CheckpointRequested {
        label: format!("Pre-swarm: {}", template_name),
        trigger: format!("swarm-dispatch:{}", template_name),
    });

    // 2. Load template from the registry
    let registry = swarm::TemplateRegistry::new();
    registry.load_defaults().await;
    let template = registry.get(&template_name).await
        .ok_or_else(|| format!("Template '{}' not found", template_name))?;

    // 3. Spawn the swarm
    let swarm_id = swarm_state.registry.spawn_from_template(&template, params).await;

    // 4. Notify the bus
    app_state.event_bus.publish(crate::system_event_bus::SystemEvent::SwarmSpawned {
        swarm_id: swarm_id.to_string(),
        template: template_name,
        agent_count: template.max_agents,
    });

    Ok(swarm_id.to_string())
}

/// Get the task DAG for a swarm (for the Gantt chart view).
#[tauri::command]
pub async fn get_swarm_dag(
    state: State<'_, SwarmState>,
    swarm_id: String,
) -> Result<Vec<swarm::dag::TaskNode>, String> {
    let id: Uuid = swarm_id.parse().map_err(|_| "invalid swarm_id")?;
    let orch = state.registry.get(id).await.ok_or("swarm not found")?;
    let dag = orch.dag.read().await;
    Ok(dag.nodes.values().cloned().collect())
}
