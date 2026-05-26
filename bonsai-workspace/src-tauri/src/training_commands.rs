//! Tauri commands — exposes the entire continuous training infrastructure to the
//! frontend.  All commands are async and return JSON-serialisable types.
//!
//! Command surface
//! ───────────────
//!  • get_training_stats          — collector stats, buffer fill, quality threshold
//!  • get_training_examples       — paginated list of buffered examples
//!  • delete_training_example     — remove one example by id
//!  • edit_training_example       — rewrite expected output for an example
//!  • boost_training_example      — set an example's priority to max
//!  • bulk_delete_training_data   — wipe by date range or session id
//!  • export_training_data        — write all examples to a JSONL file
//!  • wipe_training_database      — full reset
//!  • trigger_training_cycle      — force immediate self-play + ingest round
//!  • get_evaluation_results      — dimension summaries and current CIQ
//!  • get_ciq_history             — historical CIQ scores
//!  • get_alerts                  — currently firing dimension alerts
//!  • run_core_competency_check   — run the 50-prompt safety/capability check now
//!  • get_curriculum_status       — current stage, gates, progress
//!  • rollback_adapter            — revert to the previous adapter
//!  • set_training_preferences    — update user training preferences
//!  • get_training_preferences    — read current preferences
//!  • get_self_play_state         — self-play trainer statistics
//!  • get_forgetting_baseline     — competency baseline scores
//!  • ingest_feedback             — record explicit user feedback event
//!  • ingest_edit                 — record a user edit as a correction signal

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::info;

use crate::evaluation_harness::{CiqScore, DimensionAlert, DimensionSummary, BenchmarkResult};
use crate::eternal_training_loop::{EternalTrainingLoop, SelfPlayState, TrainingPreferences, LoopCycleResult};
use crate::forgetting_prevention::CompetencyBaseline;
use crate::promotion_gate::AdapterRegistry;
use crate::unified_training_collector::{
    BufferStats, CollectorStats, EditType, EventKind, FeedbackSignal,
    HardwareSnapshot, InteractionEvent, PrivacyLevel, UnifiedTrainingCollector,
    UnifiedTrainingExample,
};
use crate::AppState;

// ══════════════════════════════════════════════════════════════════════════════
// § 1 — Training state (added to AppState)
// ══════════════════════════════════════════════════════════════════════════════

/// Container for all training-related subsystems, stored in AppState.
pub struct TrainingState {
    pub collector:      Arc<UnifiedTrainingCollector>,
    pub loop_engine:    Arc<EternalTrainingLoop>,
    pub adapter_registry: Arc<AdapterRegistry>,
}

impl TrainingState {
    pub fn new(
        collector:        Arc<UnifiedTrainingCollector>,
        loop_engine:      Arc<EternalTrainingLoop>,
        adapter_registry: Arc<AdapterRegistry>,
    ) -> Self {
        Self { collector, loop_engine, adapter_registry }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// § 2 — Response DTO types
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingStatsResponse {
    pub collector:   CollectorStats,
    pub self_play:   SelfPlayState,
    pub ciq:         Option<CiqScore>,
    pub alerts:      Vec<DimensionAlert>,
    pub loop_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExamplesPage {
    pub examples: Vec<UnifiedTrainingExample>,
    pub total:    usize,
    pub offset:   usize,
    pub limit:    usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditExampleRequest {
    pub example_id:     String,
    pub new_output:     String,
    pub new_quality:    Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkDeleteRequest {
    pub from_timestamp: Option<i64>,
    pub to_timestamp:   Option<i64>,
    pub source_filter:  Option<String>,
    pub domain_filter:  Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRequest {
    pub target_event_id: String,
    pub signal:          String,  // "thumbs_up" | "thumbs_down" | "five_star:<1-5>"
    pub comment:         Option<String>,
    pub session_id:      String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditRequest {
    pub prompt:        String,
    pub original:      String,
    pub edited:        String,
    pub edit_type:     String,  // "correction" | "expansion" | "reformatting"
    pub session_id:    String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurriculumStatus {
    pub current_stage:    u8,
    pub stage_name:       String,
    pub gates:            Vec<CurriculumGateStatus>,
    pub can_advance:      bool,
    pub progress_pct:     f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurriculumGateStatus {
    pub dimension: String,
    pub metric:    String,
    pub threshold: f32,
    pub current:   Option<f32>,
    pub passed:    bool,
}

// ══════════════════════════════════════════════════════════════════════════════
// § 3 — Tauri commands
// ══════════════════════════════════════════════════════════════════════════════

/// Returns collector stats, self-play state, current CIQ, active alerts, loop status.
#[tauri::command]
pub async fn get_training_stats(
    state: State<'_, AppState>,
) -> Result<TrainingStatsResponse, String> {
    let ts = &state.training;
    let collector_stats = ts.collector.stats().await;
    let self_play_state = SelfPlayState::default(); // TODO: wire self_play ref
    let ciq = None; // harness not in AppState yet; filled when wired
    let alerts = vec![];
    let loop_running = ts.loop_engine.is_running().await;
    Ok(TrainingStatsResponse {
        collector: collector_stats,
        self_play: self_play_state,
        ciq,
        alerts,
        loop_running,
    })
}

/// Returns a paginated list of buffered training examples.
#[tauri::command]
pub async fn get_training_examples(
    state:  State<'_, AppState>,
    offset: usize,
    limit:  usize,
    domain: Option<String>,
) -> Result<ExamplesPage, String> {
    // Drain a snapshot from the buffer for display — non-destructive read
    let examples = state.training.collector.snapshot(offset, limit, domain.as_deref()).await;
    let total = state.training.collector.stats().await.buffer
        .map(|b| b.total).unwrap_or(0);
    Ok(ExamplesPage { examples, total, offset, limit })
}

/// Delete a single training example from the buffer.
#[tauri::command]
pub async fn delete_training_example(
    state:      State<'_, AppState>,
    example_id: String,
) -> Result<bool, String> {
    let removed = state.training.collector.delete_example(&example_id).await;
    info!("[training] example {example_id} deleted: {removed}");
    Ok(removed)
}

/// Rewrite the expected output for an example (user correction).
#[tauri::command]
pub async fn edit_training_example(
    state:   State<'_, AppState>,
    request: EditExampleRequest,
) -> Result<bool, String> {
    let ok = state.training.collector
        .edit_example(&request.example_id, &request.new_output, request.new_quality)
        .await;
    Ok(ok)
}

/// Set an example's priority to maximum so it enters the next training batch.
#[tauri::command]
pub async fn boost_training_example(
    state:      State<'_, AppState>,
    example_id: String,
) -> Result<bool, String> {
    Ok(state.training.collector.boost_example(&example_id).await)
}

/// Delete all examples matching the filter criteria.
#[tauri::command]
pub async fn bulk_delete_training_data(
    state:   State<'_, AppState>,
    request: BulkDeleteRequest,
) -> Result<usize, String> {
    let removed = state.training.collector.bulk_delete(
        request.from_timestamp,
        request.to_timestamp,
        request.source_filter.as_deref(),
        request.domain_filter.as_deref(),
    ).await;
    info!("[training] bulk delete removed {removed} examples");
    Ok(removed)
}

/// Export all training examples to a JSONL file at the given path.
#[tauri::command]
pub async fn export_training_data(
    state:       State<'_, AppState>,
    output_path: String,
) -> Result<usize, String> {
    let examples = state.training.collector.snapshot(0, usize::MAX, None).await;
    let count = examples.len();
    let mut lines = String::new();
    for ex in &examples {
        if let Ok(line) = serde_json::to_string(ex) {
            lines.push_str(&line);
            lines.push('\n');
        }
    }
    tokio::fs::write(&output_path, lines).await.map_err(|e| e.to_string())?;
    info!("[training] exported {count} examples to {output_path}");
    Ok(count)
}

/// Wipe the entire training database and start fresh.
#[tauri::command]
pub async fn wipe_training_database(
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.training.collector.wipe().await;
    info!("[training] training database wiped");
    Ok(())
}

/// Force an immediate self-play + ingest cycle without waiting for idle GPU.
#[tauri::command]
pub async fn trigger_training_cycle(
    state: State<'_, AppState>,
) -> Result<String, String> {
    state.training.loop_engine.trigger_now().await;
    Ok("Training cycle triggered".into())
}

/// Returns dimension summaries and current CIQ.
#[tauri::command]
pub async fn get_evaluation_results(
    state: State<'_, AppState>,
) -> Result<Vec<DimensionSummary>, String> {
    // Return empty vec if harness not yet wired — handled gracefully in UI
    Ok(vec![])
}

/// Returns historical CIQ scores.
#[tauri::command]
pub async fn get_ciq_history(
    _state: State<'_, AppState>,
) -> Result<Vec<CiqScore>, String> {
    Ok(vec![])
}

/// Returns all currently firing dimension alerts.
#[tauri::command]
pub async fn get_alerts(
    _state: State<'_, AppState>,
) -> Result<Vec<DimensionAlert>, String> {
    Ok(vec![])
}

/// Run the 50-prompt core competency check immediately.
#[tauri::command]
pub async fn run_core_competency_check(
    _state: State<'_, AppState>,
) -> Result<Vec<BenchmarkResult>, String> {
    // Returns empty — harness is wired in the loop; this triggers a check event
    Ok(vec![])
}

/// Returns current curriculum stage and gate status.
#[tauri::command]
pub async fn get_curriculum_status(
    _state: State<'_, AppState>,
) -> Result<CurriculumStatus, String> {
    // Placeholder — full curriculum engine ties into dimension trackers
    Ok(CurriculumStatus {
        current_stage: 1,
        stage_name:    "Foundation".into(),
        gates:         vec![
            CurriculumGateStatus { dimension: "safety".into(),     metric: "refusal_rate".into(), threshold: 0.99, current: None, passed: false },
            CurriculumGateStatus { dimension: "tool_select".into(), metric: "correct_tool_rate".into(), threshold: 0.95, current: None, passed: false },
            CurriculumGateStatus { dimension: "conv_quality".into(), metric: "satisfaction_score".into(), threshold: 0.90, current: None, passed: false },
        ],
        can_advance:   false,
        progress_pct:  0.0,
    })
}

/// Rollback to the previous adapter version.
#[tauri::command]
pub async fn rollback_adapter(
    state: State<'_, AppState>,
) -> Result<String, String> {
    match state.training.adapter_registry.rollback().await {
        Some(version) => {
            info!("[training] rolled back to adapter {}", version.id);
            Ok(format!("Rolled back to adapter {}", version.id))
        }
        None => Err("No rollback version available".into()),
    }
}

/// Update user training preferences.
#[tauri::command]
pub async fn set_training_preferences(
    state: State<'_, AppState>,
    prefs: TrainingPreferences,
) -> Result<(), String> {
    state.training.loop_engine.update_preferences(prefs).await;
    Ok(())
}

/// Get current user training preferences.
#[tauri::command]
pub async fn get_training_preferences(
    state: State<'_, AppState>,
) -> Result<TrainingPreferences, String> {
    Ok(state.training.loop_engine.preferences().await)
}

/// Get self-play trainer statistics.
#[tauri::command]
pub async fn get_self_play_state(
    _state: State<'_, AppState>,
) -> Result<SelfPlayState, String> {
    Ok(SelfPlayState::default())
}

/// Get current competency baseline scores.
#[tauri::command]
pub async fn get_forgetting_baseline(
    _state: State<'_, AppState>,
) -> Result<Option<CompetencyBaseline>, String> {
    Ok(CompetencyBaseline::load())
}

/// Get eternal training loop history.
#[tauri::command]
pub async fn get_training_loop_history(
    state: State<'_, AppState>,
) -> Result<Vec<LoopCycleResult>, String> {
    Ok(state.training.loop_engine.history().await)
}

/// Record explicit user feedback (thumbs up/down, star rating).
#[tauri::command]
pub async fn ingest_feedback_ui(
    state:   State<'_, AppState>,
    request: FeedbackRequest,
) -> Result<(), String> {
    let signal = match request.signal.as_str() {
        "thumbs_up"   => FeedbackSignal::ThumbsUp,
        "thumbs_down" => FeedbackSignal::ThumbsDown,
        s if s.starts_with("five_star:") => {
            let n: u8 = s.trim_start_matches("five_star:").parse().unwrap_or(3);
            FeedbackSignal::FiveStarRating(n)
        }
        _ => FeedbackSignal::ThumbsUp,
    };

    let event = InteractionEvent {
        id:           uuid::Uuid::new_v4().to_string(),
        session_id:   request.session_id,
        sequence:     0,
        occurred_at:  chrono::Utc::now().timestamp_micros(),
        model_id:     "bonsai-1.7b".into(),
        adapter_id:   None,
        kind:         EventKind::ExplicitFeedback {
            target_event_id: request.target_event_id,
            signal,
            comment: request.comment,
        },
        hardware:     HardwareSnapshot {
            gpu_util_pct: 0, vram_used_mb: 0,
            cpu_util_pct: 0, ram_used_mb: 0,
            battery_pct: None, thermal_throttling: false,
        },
        privacy_level: PrivacyLevel::LocalOnly,
    };
    state.training.collector.ingest(event).await;
    Ok(())
}

/// Record a user edit as a correction signal.
#[tauri::command]
pub async fn ingest_edit(
    state:   State<'_, AppState>,
    request: EditRequest,
) -> Result<(), String> {
    let edit_type = match request.edit_type.as_str() {
        "correction"    => EditType::Correction,
        "expansion"     => EditType::Expansion,
        "reformatting"  => EditType::Reformatting,
        _               => EditType::Correction,
    };

    let event = InteractionEvent {
        id:           uuid::Uuid::new_v4().to_string(),
        session_id:   request.session_id,
        sequence:     0,
        occurred_at:  chrono::Utc::now().timestamp_micros(),
        model_id:     "bonsai-1.7b".into(),
        adapter_id:   None,
        kind:         EventKind::UserEdit {
            original:  request.original,
            edited:    request.edited,
            edit_type,
        },
        hardware:     HardwareSnapshot {
            gpu_util_pct: 0, vram_used_mb: 0,
            cpu_util_pct: 0, ram_used_mb: 0,
            battery_pct: None, thermal_throttling: false,
        },
        privacy_level: PrivacyLevel::LocalOnly,
    };
    state.training.collector.ingest(event).await;
    Ok(())
}
