use universe::{
    RevertPreview, TimelineFilter, Universe, UniverseEvent, UniverseSnapshot,
    EventCategory,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

pub type UniverseState = Arc<Universe>;

// ── Query commands ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_timeline(
    universe: State<'_, UniverseState>,
    category: Option<String>,
    target_prefix: Option<String>,
    since_ns: Option<u64>,
    until_ns: Option<u64>,
    limit: Option<usize>,
) -> Result<Vec<UniverseEvent>, String> {
    let filter = TimelineFilter {
        category: category.and_then(|c| parse_category(&c)),
        target_prefix,
        since_ns,
        until_ns,
        limit,
    };
    universe.store.query_timeline(filter).await
}

#[tauri::command]
pub async fn get_snapshots(
    universe: State<'_, UniverseState>,
    limit: Option<usize>,
) -> Result<Vec<UniverseSnapshot>, String> {
    universe.store.list_snapshots(limit.unwrap_or(100)).await
}

#[tauri::command]
pub async fn create_snapshot(
    universe: State<'_, UniverseState>,
    label: Option<String>,
) -> Result<UniverseSnapshot, String> {
    universe
        .snapshots
        .take_snapshot(label, "manual".to_string())
        .await
}

#[tauri::command]
pub async fn get_universe_event(
    universe: State<'_, UniverseState>,
    event_id: String,
) -> Result<Option<UniverseEvent>, String> {
    universe.store.get_event(&event_id).await
}

#[tauri::command]
pub async fn universe_event_count(universe: State<'_, UniverseState>) -> Result<u64, String> {
    Ok(universe.store.event_count().await)
}

// ── Revert commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn revert_preview_event(
    universe: State<'_, UniverseState>,
    event_id: String,
) -> Result<RevertPreview, String> {
    let event = universe
        .store
        .get_event(&event_id)
        .await?
        .ok_or_else(|| format!("Event {} not found", event_id))?;

    let now_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);

    let events_since = universe
        .store
        .query_timeline(TimelineFilter {
            since_ns: Some(event.timestamp_ns),
            until_ns: Some(now_ns),
            limit: Some(1000),
            ..Default::default()
        })
        .await?;

    let affected_files: Vec<String> = events_since
        .iter()
        .filter(|e| e.category == EventCategory::FileChange)
        .map(|e| e.target.clone())
        .collect();

    let affected_configs: Vec<String> = events_since
        .iter()
        .filter(|e| e.category == EventCategory::ConfigChange)
        .map(|e| e.target.clone())
        .collect();

    Ok(RevertPreview {
        target_event_id: Some(event_id),
        target_snapshot_id: None,
        affected_files,
        affected_configs,
        model_changes: Vec::new(),
        agent_changes: Vec::new(),
        event_count_to_undo: events_since.len() as u64,
        estimated_duration_ms: (events_since.len() as u64) * 10,
    })
}

#[tauri::command]
pub async fn revert_preview_snapshot(
    universe: State<'_, UniverseState>,
    snapshot_id: String,
) -> Result<RevertPreview, String> {
    let snaps = universe.store.list_snapshots(500).await?;
    let snap = snaps
        .into_iter()
        .find(|s| s.snapshot_id == snapshot_id)
        .ok_or_else(|| format!("Snapshot {} not found", snapshot_id))?;

    let now_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);

    let events_since = universe
        .store
        .query_timeline(TimelineFilter {
            since_ns: Some(snap.timestamp_ns),
            until_ns: Some(now_ns),
            limit: Some(1000),
            ..Default::default()
        })
        .await?;

    let affected_files: Vec<String> = events_since
        .iter()
        .filter(|e| e.category == EventCategory::FileChange)
        .map(|e| e.target.clone())
        .collect();

    Ok(RevertPreview {
        target_event_id: None,
        target_snapshot_id: Some(snapshot_id),
        affected_files,
        affected_configs: Vec::new(),
        model_changes: Vec::new(),
        agent_changes: Vec::new(),
        event_count_to_undo: events_since.len() as u64,
        estimated_duration_ms: (events_since.len() as u64) * 10,
    })
}

/// Confirm a revert operation. Records a Reversion event in the timeline.
/// Full file/state restoration is wired in Phase 2; this records the intent
/// and marks the reversion in the audit trail.
#[tauri::command]
pub async fn revert_confirm(
    universe: State<'_, UniverseState>,
    target_event_id: Option<String>,
    target_snapshot_id: Option<String>,
) -> Result<String, String> {
    use universe::{EventCategory, EventSource, UniverseEvent};

    let target = target_event_id
        .as_deref()
        .or(target_snapshot_id.as_deref())
        .unwrap_or("unknown");

    let mut ev = UniverseEvent::new(
        EventSource::User { peer_id: universe.store.device_id().to_string() },
        EventCategory::Reversion,
        format!("Reverted to: {}", target),
        target.to_string(),
        universe.store.device_id().to_string(),
    );
    ev.metadata = serde_json::json!({
        "target_event_id": target_event_id,
        "target_snapshot_id": target_snapshot_id,
        "status": "recorded",
        "note": "Full restoration wired in Phase 2",
    });

    universe.emitter.emit(ev);

    // Create a post-revert snapshot so the timeline shows the new baseline
    let snap = universe
        .snapshots
        .take_snapshot(Some(format!("Post-revert: {}", target)), target.to_string())
        .await?;

    Ok(snap.snapshot_id)
}

fn parse_category(s: &str) -> Option<EventCategory> {
    match s {
        "FileChange" => Some(EventCategory::FileChange),
        "ConfigChange" => Some(EventCategory::ConfigChange),
        "ModelChange" => Some(EventCategory::ModelChange),
        "AgentAction" => Some(EventCategory::AgentAction),
        "SwarmEvent" => Some(EventCategory::SwarmEvent),
        "CollaborationEvent" => Some(EventCategory::CollaborationEvent),
        "ComputeEvent" => Some(EventCategory::ComputeEvent),
        "ExtensionEvent" => Some(EventCategory::ExtensionEvent),
        "SurvivalEvent" => Some(EventCategory::SurvivalEvent),
        "CreditTransaction" => Some(EventCategory::CreditTransaction),
        "Checkpoint" => Some(EventCategory::Checkpoint),
        "Reversion" => Some(EventCategory::Reversion),
        _ => None,
    }
}
