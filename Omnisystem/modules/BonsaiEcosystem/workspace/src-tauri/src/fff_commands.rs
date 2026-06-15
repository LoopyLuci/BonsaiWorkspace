use failure_finder::{
    CampaignSpec, CampaignState, F3Orchestrator, FailureReport,
    OrchestratorStats, SurvivalBridge, TargetKind, FuzzStrategy, ResourceBudget,
};
use std::sync::Arc;
use tauri::State;

pub struct F3State {
    pub orchestrator: Arc<F3Orchestrator>,
}

impl F3State {
    pub fn new(kb_path: &str) -> Self {
        let bridge = SurvivalBridge::new(kb_path);
        Self { orchestrator: F3Orchestrator::new(bridge) }
    }
}

// ── Campaign management ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn fff_start_campaign(
    state: State<'_, F3State>,
    spec: CampaignSpec,
) -> Result<String, String> {
    let id = state.orchestrator.clone().start_campaign(spec).await;
    Ok(id.to_string())
}

#[tauri::command]
pub async fn fff_start_preset(
    state: State<'_, F3State>,
    preset: String,
) -> Result<String, String> {
    let spec = match preset.as_str() {
        "tauri_filesystem" => CampaignSpec::tauri_filesystem_fuzz(),
        "swarm_agent"      => CampaignSpec::swarm_agent_fuzz(),
        "crdt"             => CampaignSpec::crdt_concurrency_fuzz(),
        "resource"         => CampaignSpec::resource_exhaustion(),
        other              => return Err(format!("Unknown preset: {}", other)),
    };
    let id = state.orchestrator.clone().start_campaign(spec).await;
    Ok(id.to_string())
}

#[tauri::command]
pub async fn fff_stop_campaign(
    state: State<'_, F3State>,
    campaign_id: String,
) -> Result<bool, String> {
    let id: uuid::Uuid = campaign_id.parse().map_err(|_| "invalid campaign_id")?;
    Ok(state.orchestrator.stop_campaign(id).await)
}

#[tauri::command]
pub async fn fff_list_campaigns(
    state: State<'_, F3State>,
) -> Result<Vec<CampaignState>, String> {
    Ok(state.orchestrator.list_campaigns())
}

#[tauri::command]
pub async fn fff_list_failures(
    state: State<'_, F3State>,
) -> Result<Vec<FailureReport>, String> {
    Ok(state.orchestrator.list_failures())
}

#[tauri::command]
pub async fn fff_stats(
    state: State<'_, F3State>,
) -> Result<OrchestratorStats, String> {
    Ok(state.orchestrator.stats().await)
}

/// Export a failure as a minimal reproduction script (stored in CAS).
#[tauri::command]
pub async fn fff_export_failure(
    state: State<'_, F3State>,
    failure_id: String,
) -> Result<serde_json::Value, String> {
    let failures = state.orchestrator.list_failures();
    let failure = failures.iter()
        .find(|f| f.id == failure_id)
        .ok_or_else(|| format!("Failure {} not found", failure_id))?;

    Ok(serde_json::json!({
        "id":              failure.id,
        "error_pattern":   failure.error_pattern,
        "reproduction_cmd": failure.reproduction_cmd,
        "minimal_input":   failure.minimal_input,
        "auto_fix_script": failure.auto_fix_script,
        "timestamp_ns":    failure.timestamp_ns,
    }))
}

// ── Sandbox Nervous System commands ───────────────────────────────────────────

use sns::{SandboxInfo, SandboxSupervisor};

pub struct SnsState {
    pub supervisor: Arc<SandboxSupervisor>,
}

impl SnsState {
    pub fn new() -> Self {
        Self { supervisor: sns::start_supervisor() }
    }
}

#[tauri::command]
pub async fn sns_list_sandboxes(
    state: State<'_, SnsState>,
) -> Result<Vec<SandboxInfo>, String> {
    Ok(state.supervisor.list_sandboxes())
}

#[tauri::command]
pub async fn sns_list_violations(
    state: State<'_, SnsState>,
) -> Result<Vec<sns::CapabilityViolation>, String> {
    Ok(state.supervisor.all_violations())
}

#[tauri::command]
pub async fn sns_terminate_sandbox(
    state: State<'_, SnsState>,
    sandbox_id: String,
) -> Result<(), String> {
    state.supervisor.terminate(&sandbox_id);
    Ok(())
}
