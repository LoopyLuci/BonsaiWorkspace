/// Universe hooks — bridges SystemEventBus events and direct file mutations
/// into the append-only Universe event ledger.
///
/// Architecture:
///   - `start_event_bridge`: spawns a task that subscribes to the SystemEventBus
///     and converts each event into a UniverseEvent. Covers training, swarm,
///     extensions, credits, and all other subsystems automatically.
///   - `emit_file_change`: called directly from write_file/delete_file commands.
use std::sync::Arc;

use universe::{EventCategory, EventSource, Universe, UniverseEvent};
use tracing::{debug, warn};

use crate::system_event_bus::{SharedEventBus, SystemEvent};

/// Subscribe to the system event bus and forward every event to the Universe
/// ledger as a typed UniverseEvent. This covers all subsystems that already
/// publish to the bus (training, swarm, extensions, credits, etc.) without
/// touching each subsystem individually.
pub fn start_event_bridge(bus: SharedEventBus, universe: Arc<Universe>) {
    tokio::spawn(async move {
        let mut rx = bus.subscribe();
        loop {
            match rx.recv().await {
                Ok(sys_event) => {
                    // Special case: CheckpointRequested creates a real snapshot
                    if let SystemEvent::CheckpointRequested { label, trigger } = &sys_event {
                        let uni = universe.clone();
                        let label = label.clone();
                        let trigger = trigger.clone();
                        tokio::spawn(async move {
                            if let Err(e) = uni.snapshots.take_snapshot(Some(label), trigger).await {
                                warn!("Checkpoint failed: {}", e);
                            }
                        });
                        continue;
                    }
                    let ev = convert_system_event(&sys_event, &universe);
                    if let Some(ev) = ev {
                        universe.emitter.emit(ev);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!("UniverseEventBridge lagged by {} events", n);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

fn convert_system_event(sys: &SystemEvent, universe: &Universe) -> Option<UniverseEvent> {
    let device = universe.store.device_id().to_string();
    let source = EventSource::System { component: "SystemEventBus".into() };

    let (category, summary, target) = match sys {
        SystemEvent::BuildStarted { crate_name, triggered_by } => (
            EventCategory::AgentAction,
            format!("CI build started: {}", crate_name),
            format!("crate:{}", crate_name),
        ),
        SystemEvent::BuildFailed { crate_name, error, .. } => (
            EventCategory::SurvivalEvent,
            format!("CI build FAILED: {} — {}", crate_name, &error[..error.len().min(80)]),
            format!("crate:{}", crate_name),
        ),
        SystemEvent::BuildPassed { crate_name, artifact_hash, .. } => (
            EventCategory::AgentAction,
            format!("CI build passed: {}", crate_name),
            format!("crate:{}", crate_name),
        ),
        SystemEvent::TestFailed { suite, failures, .. } => (
            EventCategory::SurvivalEvent,
            format!("Test failure in {} ({} failures)", suite, failures.len()),
            format!("suite:{}", suite),
        ),
        SystemEvent::TestPassed { suite, count, .. } => (
            EventCategory::AgentAction,
            format!("Tests passed: {} ({} tests)", suite, count),
            format!("suite:{}", suite),
        ),
        SystemEvent::CrashDetected { component, .. } => (
            EventCategory::SurvivalEvent,
            format!("Crash detected: {}", component),
            format!("component:{}", component),
        ),
        SystemEvent::CrashResolved { component, fix_strategy, .. } => (
            EventCategory::SurvivalEvent,
            format!("Crash resolved: {} — {}", component, fix_strategy),
            format!("component:{}", component),
        ),
        SystemEvent::TrainingStarted { phase, dataset } => (
            EventCategory::ModelChange,
            format!("Training started: phase {}", phase),
            format!("training:{}", phase),
        ),
        SystemEvent::TrainingComplete { phase, adapter_path, .. } => (
            EventCategory::ModelChange,
            format!("Training complete: phase {} → {}", phase, adapter_path),
            format!("adapter:{}", adapter_path),
        ),
        SystemEvent::TrainingFailed { phase, error, epoch } => (
            EventCategory::ModelChange,
            format!("Training FAILED: phase {} at epoch {} — {}", phase, epoch, &error[..error.len().min(60)]),
            format!("training:{}", phase),
        ),
        SystemEvent::UpgradeReady { component, version, .. } => (
            EventCategory::ModelChange,
            format!("Upgrade ready: {} v{}", component, version),
            format!("component:{}", component),
        ),
        SystemEvent::UpgradeDeployed { component, version, .. } => (
            EventCategory::ModelChange,
            format!("Upgrade deployed: {} v{}", component, version),
            format!("component:{}", component),
        ),
        SystemEvent::UpgradeRolledBack { component, reason, .. } => (
            EventCategory::SurvivalEvent,
            format!("Upgrade rolled back: {} — {}", component, reason),
            format!("component:{}", component),
        ),
        SystemEvent::ProofVerificationFailed { tool, reason } => (
            EventCategory::SurvivalEvent,
            format!("Proof verification failed for tool {}: {}", tool, reason),
            format!("tool:{}", tool),
        ),
        SystemEvent::ExtensionInstalled { extension_id, version, .. } => (
            EventCategory::ExtensionEvent,
            format!("Extension installed: {} v{}", extension_id, version),
            format!("extension:{}", extension_id),
        ),
        SystemEvent::ExtensionUpdated { extension_id, old_version, new_version } => (
            EventCategory::ExtensionEvent,
            format!("Extension updated: {} {} → {}", extension_id, old_version, new_version),
            format!("extension:{}", extension_id),
        ),
        SystemEvent::ExtensionUninstalled { extension_id, reason } => (
            EventCategory::ExtensionEvent,
            format!("Extension uninstalled: {} ({})", extension_id, reason),
            format!("extension:{}", extension_id),
        ),
        SystemEvent::SwarmSpawned { swarm_id, template, agent_count } => (
            EventCategory::SwarmEvent,
            format!("Swarm spawned: {} ({} agents, template: {})", swarm_id, agent_count, template),
            format!("swarm:{}", swarm_id),
        ),
        SystemEvent::SwarmCompleted { swarm_id, tasks_completed, .. } => (
            EventCategory::SwarmEvent,
            format!("Swarm completed: {} ({} tasks)", swarm_id, tasks_completed),
            format!("swarm:{}", swarm_id),
        ),
        SystemEvent::SwarmFailed { swarm_id, failed_task, error } => (
            EventCategory::SwarmEvent,
            format!("Swarm FAILED: {} at {} — {}", swarm_id, failed_task, &error[..error.len().min(60)]),
            format!("swarm:{}", swarm_id),
        ),
        SystemEvent::CreditEarned { amount, project_id, .. } => (
            EventCategory::CreditTransaction,
            format!("Credit earned: +{:.4} URV for {}", amount, project_id),
            format!("project:{}", project_id),
        ),
        SystemEvent::CreditSpent { amount, project_id, .. } => (
            EventCategory::CreditTransaction,
            format!("Credit spent: -{:.4} URV for {}", amount, project_id),
            format!("project:{}", project_id),
        ),
        SystemEvent::IssueCreated { issue_id, title, .. } => (
            EventCategory::AgentAction,
            format!("Issue created: {}", title),
            format!("issue:{}", issue_id),
        ),
        SystemEvent::IssueResolved { issue_id, resolution, .. } => (
            EventCategory::AgentAction,
            format!("Issue resolved: {} — {}", issue_id, resolution),
            format!("issue:{}", issue_id),
        ),
        SystemEvent::ModelHotReloaded { model_path, previous_model } => (
            EventCategory::ModelChange,
            format!("Model hot-reloaded: {}", model_path),
            format!("model:{}", model_path),
        ),
        SystemEvent::DreamCycleComplete { nodes_consolidated, .. } => (
            EventCategory::ModelChange,
            format!("Dream cycle complete: {} nodes consolidated", nodes_consolidated),
            "dream:consolidation".to_string(),
        ),
        SystemEvent::ConfigChanged { key, old_value, new_value } => (
            EventCategory::ConfigChange,
            format!("Config changed: {}", key),
            format!("config:{}", key),
        ),
        // Events that don't need universe recording (high-frequency progress, heartbeats)
        SystemEvent::TrainingProgress { .. }
        | SystemEvent::ComponentHealthDegraded { .. }
        | SystemEvent::ComponentHealthRestored { .. }
        | SystemEvent::ResourceThresholdAlert { .. }
        | SystemEvent::UserFeedback { .. }
        | SystemEvent::FeatureRequest { .. }
        | SystemEvent::SecurityReviewRequested { .. }
        | SystemEvent::PRMerged { .. }
        | SystemEvent::ExternalPush { .. }
        | SystemEvent::IssueAssigned { .. }
        | SystemEvent::SecurityReviewComplete { .. }
        | SystemEvent::UpgradeDeploying { .. }
        | SystemEvent::CheckpointRequested { .. }
        | SystemEvent::TestStarted { .. }
        | SystemEvent::DreamCycleStarted { .. }
        | SystemEvent::UiPanelGenerated { .. }
        | SystemEvent::UiPanelReloadRequested { .. } => return None,
    };

    let mut ev = UniverseEvent::new(source, category, summary, target, device);
    ev.metadata = serde_json::to_value(sys).unwrap_or(serde_json::Value::Null);
    Some(ev)
}

/// Emit a file-change UniverseEvent. Called from write_file / delete_file commands.
/// Non-blocking — queues the event and returns immediately.
pub fn emit_file_change(
    universe: &Universe,
    operation: &str,
    path: &str,
    before_hash: Option<String>,
    after_hash: Option<String>,
    peer_id: String,
) {
    let device = universe.store.device_id().to_string();
    let ev = UniverseEvent::new(
        EventSource::User { peer_id },
        EventCategory::FileChange,
        format!("{}: {}", operation, path),
        path.to_string(),
        device,
    )
    .with_hashes(before_hash, after_hash)
    .with_metadata(serde_json::json!({ "operation": operation, "path": path }));

    if !universe.emitter.emit(ev) {
        debug!("Universe emitter buffer full — file event dropped for {}", path);
    }
}

/// Compute BLAKE3 hash of file contents. Returns None if the file doesn't exist.
pub fn hash_file_sync(path: &std::path::Path) -> Option<String> {
    std::fs::read(path)
        .ok()
        .map(|data| blake3::hash(&data).to_hex().to_string())
}
