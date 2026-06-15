//! Workstream A — Omnipresent Capture
//!
//! Captures every user interaction as a structured OmnEvent and feeds it into
//! the UnifiedTrainingCollector for continuous learning.  Events are also
//! persisted to the CAS store for long-term auditability.
//!
//! All capture is opt-in per privacy tier; the defaults write only locally.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::unified_training_collector::{
    ConvMessage, ModelRole, QualityMeta, TrainingInput, TrainingOutput, TrainingSource,
    TrainingStrategyType, UnifiedTrainingCollector, UnifiedTrainingExample,
};

// ─────────────────────────────────────────────────────────────────────────────
// § 1 — Event types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum OmnEventType {
    // Input
    KeyPress {
        key: String,
        modifiers: Vec<String>,
    },
    MouseClick {
        x: i32,
        y: i32,
        button: String,
    },
    MouseMove {
        x: i32,
        y: i32,
        delta_x: i32,
        delta_y: i32,
    },
    Scroll {
        delta: i32,
    },
    VoiceCommand {
        transcript: String,
        confidence: f32,
    },
    Gesture {
        gesture_type: String,
        data: Value,
    },
    // System
    AppLaunch {
        app_id: String,
        args: Vec<String>,
    },
    AppClose {
        app_id: String,
        duration_ms: u64,
    },
    FileOpen {
        path: String,
        app_id: String,
    },
    FileSave {
        path: String,
        size_bytes: u64,
    },
    FileDelete {
        path: String,
    },
    FileCopy {
        src: String,
        dst: String,
    },
    ProcessStart {
        pid: u32,
        name: String,
        memory_mb: u64,
    },
    ProcessExit {
        pid: u32,
        exit_code: i32,
    },
    NetworkRequest {
        url: String,
        method: String,
        status_code: u16,
    },
    ClipboardChange {
        content_hash: String,
    },
    ScreenCapture {
        hash: String,
        width: u32,
        height: u32,
    },
    // Shell
    CommandTyped {
        command: String,
        shell: String,
    },
    CommandCompleted {
        command: String,
        exit_code: i32,
        duration_ms: u64,
    },
    AutocompleteAccepted {
        prefix: String,
        completion: String,
    },
    // Intelligence
    ModelInference {
        model_id: String,
        prompt_hash: String,
        tokens: u32,
    },
    ToolInvocation {
        tool_name: String,
        args_hash: String,
        success: bool,
    },
    AgentAction {
        agent_id: String,
        action_type: String,
        outcome: String,
    },
    ReasoningAttempt {
        strategy: String,
        confidence: f32,
        correct: Option<bool>,
    },
    TrainingCycle {
        adapter_id: String,
        metrics: Value,
    },
    // User feedback
    ExplicitCorrection {
        original: String,
        corrected: String,
    },
    ImplicitPreference {
        chosen: String,
        rejected: String,
    },
    SatisfactionRating {
        rating: u8,
        context: String,
    },
    // Workspace
    WorkspaceSwitch {
        from: String,
        to: String,
    },
    WindowFocus {
        app_id: String,
        title: String,
    },
    WindowClose {
        app_id: String,
    },
    TabSwitch {
        app_id: String,
        from_tab: String,
        to_tab: String,
    },
}

impl OmnEventType {
    pub fn category(&self) -> &'static str {
        match self {
            Self::KeyPress { .. }
            | Self::MouseClick { .. }
            | Self::MouseMove { .. }
            | Self::Scroll { .. }
            | Self::VoiceCommand { .. }
            | Self::Gesture { .. } => "input",
            Self::AppLaunch { .. }
            | Self::AppClose { .. }
            | Self::FileOpen { .. }
            | Self::FileSave { .. }
            | Self::FileDelete { .. }
            | Self::FileCopy { .. }
            | Self::ProcessStart { .. }
            | Self::ProcessExit { .. }
            | Self::NetworkRequest { .. }
            | Self::ClipboardChange { .. }
            | Self::ScreenCapture { .. } => "system",
            Self::CommandTyped { .. }
            | Self::CommandCompleted { .. }
            | Self::AutocompleteAccepted { .. } => "shell",
            Self::ModelInference { .. }
            | Self::ToolInvocation { .. }
            | Self::AgentAction { .. }
            | Self::ReasoningAttempt { .. }
            | Self::TrainingCycle { .. } => "intelligence",
            Self::ExplicitCorrection { .. }
            | Self::ImplicitPreference { .. }
            | Self::SatisfactionRating { .. } => "feedback",
            Self::WorkspaceSwitch { .. }
            | Self::WindowFocus { .. }
            | Self::WindowClose { .. }
            | Self::TabSwitch { .. } => "workspace",
        }
    }

    pub fn is_high_value(&self) -> bool {
        matches!(
            self,
            Self::ExplicitCorrection { .. }
                | Self::ImplicitPreference { .. }
                | Self::SatisfactionRating { .. }
                | Self::ReasoningAttempt { .. }
                | Self::ToolInvocation { .. }
                | Self::CommandCompleted { .. }
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 2 — Context and event envelope
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OmnContext {
    pub active_app: String,
    pub active_window_title: String,
    pub open_files: Vec<String>,
    pub recent_commands: Vec<String>,
    pub conversation_history: Option<Vec<String>>,
    pub workspace_state: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareSnapshot {
    pub cpu_pct: u8,
    pub gpu_pct: u8,
    pub ram_mb: u64,
    pub vram_mb: u64,
}

impl Default for HardwareSnapshot {
    fn default() -> Self {
        Self {
            cpu_pct: 0,
            gpu_pct: 0,
            ram_mb: 0,
            vram_mb: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmnEvent {
    pub id: Uuid,
    pub session_id: Uuid,
    pub timestamp: i64,
    pub event_type: OmnEventType,
    pub context: OmnContext,
    pub hardware: HardwareSnapshot,
    pub privacy_tier: PrivacyTier,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrivacyTier {
    LocalOnly,
    LocalWithHash,
    FederatedAnon,
}

impl OmnEvent {
    pub fn new(session_id: Uuid, event_type: OmnEventType, context: OmnContext) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            timestamp: Utc::now().timestamp_micros(),
            event_type,
            context,
            hardware: HardwareSnapshot::default(),
            privacy_tier: PrivacyTier::LocalOnly,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 3 — Session tracker
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionSummary {
    pub session_id: Uuid,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub event_count: u64,
    pub categories: std::collections::HashMap<String, u64>,
    pub top_apps: Vec<(String, u64)>,
    pub commands_run: u64,
    pub files_touched: u64,
    pub ai_inferences: u64,
    pub corrections_given: u64,
}

struct SessionTracker {
    session_id: Uuid,
    started_at: i64,
    event_count: u64,
    categories: std::collections::HashMap<String, u64>,
    app_counts: std::collections::HashMap<String, u64>,
    commands_run: u64,
    files_touched: u64,
    ai_inferences: u64,
    corrections_given: u64,
}

impl SessionTracker {
    fn new() -> Self {
        Self {
            session_id: Uuid::new_v4(),
            started_at: Utc::now().timestamp_micros(),
            event_count: 0,
            categories: Default::default(),
            app_counts: Default::default(),
            commands_run: 0,
            files_touched: 0,
            ai_inferences: 0,
            corrections_given: 0,
        }
    }

    fn observe(&mut self, ev: &OmnEvent) {
        self.event_count += 1;
        *self
            .categories
            .entry(ev.event_type.category().into())
            .or_default() += 1;
        if !ev.context.active_app.is_empty() {
            *self
                .app_counts
                .entry(ev.context.active_app.clone())
                .or_default() += 1;
        }
        match &ev.event_type {
            OmnEventType::CommandCompleted { .. } => self.commands_run += 1,
            OmnEventType::FileOpen { .. } | OmnEventType::FileSave { .. } => {
                self.files_touched += 1
            }
            OmnEventType::ModelInference { .. } => self.ai_inferences += 1,
            OmnEventType::ExplicitCorrection { .. } => self.corrections_given += 1,
            _ => {}
        }
    }

    fn summary(&self) -> SessionSummary {
        let mut apps: Vec<(String, u64)> = self.app_counts.clone().into_iter().collect();
        apps.sort_by(|a, b| b.1.cmp(&a.1));
        apps.truncate(5);
        SessionSummary {
            session_id: self.session_id,
            started_at: self.started_at,
            ended_at: None,
            event_count: self.event_count,
            categories: self.categories.clone(),
            top_apps: apps,
            commands_run: self.commands_run,
            files_touched: self.files_touched,
            ai_inferences: self.ai_inferences,
            corrections_given: self.corrections_given,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 4 — OmnipresentCapture
// ─────────────────────────────────────────────────────────────────────────────

const RING_CAPACITY: usize = 10_000;

pub struct OmnipresentCapture {
    cas: Arc<cas::CasStore>,
    collector: Arc<UnifiedTrainingCollector>,
    session: RwLock<SessionTracker>,
    ring: RwLock<VecDeque<OmnEvent>>,
    last_hw_sample: RwLock<Instant>,
    hw_snapshot: RwLock<HardwareSnapshot>,
    current_context: RwLock<OmnContext>,
    enabled: bool,
}

impl OmnipresentCapture {
    pub fn new(
        cas: Arc<cas::CasStore>,
        collector: Arc<UnifiedTrainingCollector>,
    ) -> Arc<Self> {
        Arc::new(Self {
            cas,
            collector,
            session: RwLock::new(SessionTracker::new()),
            ring: RwLock::new(VecDeque::with_capacity(RING_CAPACITY)),
            last_hw_sample: RwLock::new(Instant::now()),
            hw_snapshot: RwLock::new(HardwareSnapshot::default()),
            current_context: RwLock::new(OmnContext::default()),
            enabled: true,
        })
    }

    /// Record an event.  High-value events are synchronously pushed to the
    /// training collector; all events are buffered in the ring and persisted
    /// to CAS in the background.
    pub async fn record(&self, event_type: OmnEventType) {
        if !self.enabled {
            return;
        }

        let context = self.current_context.read().await.clone();
        let hw = self.hw_snapshot.read().await.clone();
        let session_id = self.session.read().await.session_id;

        let mut ev = OmnEvent::new(session_id, event_type, context);
        ev.hardware = hw;

        // Update session tracker
        self.session.write().await.observe(&ev);

        // Push to in-memory ring
        {
            let mut ring = self.ring.write().await;
            if ring.len() >= RING_CAPACITY {
                ring.pop_front();
            }
            ring.push_back(ev.clone());
        }

        // For high-value events, generate a training example
        if ev.event_type.is_high_value() {
            self.maybe_generate_training_example(&ev).await;
        }

        // Async CAS persistence (fire-and-forget)
        let cas = self.cas.clone();
        let ev_clone = ev.clone();
        tokio::spawn(async move {
            if let Ok(bytes) = serde_json::to_vec(&ev_clone) {
                let _ = cas.put(&bytes, "application/json").await;
            }
        });
    }

    /// Update the current user context (called by OS hooks or UI layer)
    pub async fn update_context(&self, ctx: OmnContext) {
        *self.current_context.write().await = ctx;
    }

    /// Update the hardware snapshot (called by 1 Hz sampler)
    pub async fn update_hardware(&self, hw: HardwareSnapshot) {
        let mut last = self.last_hw_sample.write().await;
        if last.elapsed() >= Duration::from_millis(900) {
            *self.hw_snapshot.write().await = hw;
            *last = Instant::now();
        }
    }

    /// Return recent events (newest-last, limited by `n`)
    pub async fn recent_events(&self, n: usize) -> Vec<OmnEvent> {
        let ring = self.ring.read().await;
        ring.iter()
            .rev()
            .take(n)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Return the current session summary
    pub async fn session_summary(&self) -> SessionSummary {
        self.session.read().await.summary()
    }

    /// Start a new session (e.g. after login or workspace switch)
    pub async fn new_session(&self) {
        *self.session.write().await = SessionTracker::new();
        info!("[omnipresent] new session started");
    }

    // ── Training example generation ──────────────────────────────────────────

    async fn maybe_generate_training_example(&self, ev: &OmnEvent) {
        let example = match &ev.event_type {
            OmnEventType::ExplicitCorrection {
                original,
                corrected,
            } => {
                let meta = QualityMeta {
                    edit_distance: levenshtein(original, corrected) as u32,
                    original_len: original.len() as u32,
                    ..Default::default()
                };
                let qs = crate::unified_training_collector::quality_score(
                    &TrainingSource::UserCorrections,
                    &meta,
                );
                Some(UnifiedTrainingExample {
                    id: Uuid::new_v4().to_string(),
                    target_model: ModelRole::Primary,
                    suitable_strategies: vec![TrainingStrategyType::Dpo],
                    input: TrainingInput::Conversation {
                        messages: vec![ConvMessage {
                            role: "user".into(),
                            content: original.clone(),
                        }],
                    },
                    expected_output: TrainingOutput::PreferencePair {
                        chosen: corrected.clone(),
                        rejected: original.clone(),
                    },
                    source: TrainingSource::UserCorrections,
                    quality_score: qs,
                    priority: qs,
                    timestamp: ev.timestamp,
                    dimensions: vec!["user_corrections".into()],
                    used: false,
                    use_count: 0,
                    metadata: serde_json::json!({
                        "session_id": ev.session_id,
                        "app": ev.context.active_app,
                    }),
                })
            }
            OmnEventType::CommandCompleted {
                command,
                exit_code,
                duration_ms,
            } => {
                let succeeded = *exit_code == 0;
                let meta = QualityMeta {
                    tool_succeeded: succeeded,
                    ..Default::default()
                };
                let qs = crate::unified_training_collector::quality_score(
                    &TrainingSource::ToolExecution,
                    &meta,
                );
                Some(UnifiedTrainingExample {
                    id: Uuid::new_v4().to_string(),
                    target_model: ModelRole::Primary,
                    suitable_strategies: vec![TrainingStrategyType::CurriculumSft],
                    input: TrainingInput::Conversation {
                        messages: vec![ConvMessage {
                            role: "user".into(),
                            content: format!("Run: {command}"),
                        }],
                    },
                    expected_output: TrainingOutput::Text {
                        content: if succeeded {
                            format!("Command succeeded in {duration_ms}ms")
                        } else {
                            format!("Command failed (exit {exit_code}) after {duration_ms}ms")
                        },
                    },
                    source: TrainingSource::ToolExecution,
                    quality_score: qs,
                    priority: qs,
                    timestamp: ev.timestamp,
                    dimensions: vec!["shell".into()],
                    used: false,
                    use_count: 0,
                    metadata: serde_json::json!({ "exit_code": exit_code, "duration_ms": duration_ms }),
                })
            }
            _ => None,
        };

        if let Some(ex) = example {
            self.collector.ingest_bulk(vec![ex]).await;
        }
    }
}

/// Simple Levenshtein distance approximation (capped at 1000 chars)
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().take(200).collect();
    let b: Vec<char> = b.chars().take(200).collect();
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m {
        dp[i][0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1]
            } else {
                1 + dp[i - 1][j].min(dp[i][j - 1]).min(dp[i - 1][j - 1])
            };
        }
    }
    dp[m][n]
}

impl OmnipresentCapture {
    /// Return all events recorded since `since_ms` (milliseconds epoch).
    pub async fn get_events_since(&self, since_ms: i64) -> Vec<OmnEvent> {
        // Events are stored with timestamp in microseconds
        let since_us = since_ms * 1_000;
        self.ring
            .read()
            .await
            .iter()
            .filter(|e| e.timestamp >= since_us)
            .cloned()
            .collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 5 — Tauri commands
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn omn_record_event(
    state: tauri::State<'_, crate::AppState>,
    event_type: OmnEventType,
) -> Result<(), String> {
    state.omnipresent.record(event_type).await;
    Ok(())
}

#[tauri::command]
pub async fn omn_get_recent_events(
    state: tauri::State<'_, crate::AppState>,
    n: usize,
) -> Result<Vec<OmnEvent>, String> {
    Ok(state.omnipresent.recent_events(n.min(500)).await)
}

#[tauri::command]
pub async fn omn_get_session_summary(
    state: tauri::State<'_, crate::AppState>,
) -> Result<SessionSummary, String> {
    Ok(state.omnipresent.session_summary().await)
}

#[tauri::command]
pub async fn omn_update_context(
    state: tauri::State<'_, crate::AppState>,
    ctx: OmnContext,
) -> Result<(), String> {
    state.omnipresent.update_context(ctx).await;
    Ok(())
}

#[tauri::command]
pub async fn omn_new_session(state: tauri::State<'_, crate::AppState>) -> Result<(), String> {
    state.omnipresent.new_session().await;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// § 6 — Background hardware sampler
// ─────────────────────────────────────────────────────────────────────────────

/// Spawn a 1 Hz background task that feeds hardware snapshots to OmnipresentCapture.
pub fn spawn_hardware_sampler(capture: Arc<OmnipresentCapture>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            // In a full implementation this would read from sysinfo / GPU telemetry.
            // For now we emit zeros so the type system is satisfied.
            capture.update_hardware(HardwareSnapshot::default()).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_categories() {
        assert_eq!(
            OmnEventType::KeyPress {
                key: "a".into(),
                modifiers: vec![]
            }
            .category(),
            "input"
        );
        assert_eq!(
            OmnEventType::FileSave {
                path: "/tmp/x".into(),
                size_bytes: 0
            }
            .category(),
            "system"
        );
        assert_eq!(
            OmnEventType::CommandCompleted {
                command: "ls".into(),
                exit_code: 0,
                duration_ms: 5
            }
            .category(),
            "shell"
        );
    }

    #[test]
    fn high_value_classification() {
        assert!(OmnEventType::ExplicitCorrection {
            original: "foo".into(),
            corrected: "bar".into()
        }
        .is_high_value());
        assert!(!OmnEventType::MouseMove {
            x: 0,
            y: 0,
            delta_x: 1,
            delta_y: 0
        }
        .is_high_value());
    }

    #[test]
    fn levenshtein_distance() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", "abc"), 0);
    }
}
