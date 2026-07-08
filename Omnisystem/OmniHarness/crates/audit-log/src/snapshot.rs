use crate::event::{EventCategory, EventSource, SubsystemHashes, UniverseEvent, UniverseSnapshot};
use crate::emitter::UniverseEmitter;
use crate::store::UniverseStore;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{info, warn};

/// Per-category retention configuration (in days; 0 = keep forever).
#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    pub file_changes_days: u64,
    pub agent_actions_days: u64,
    pub survival_events_days: u64,
    pub collaboration_events_hours: u64,
    pub compute_events_days: u64,
    /// Categories with 0 retention are kept forever (credit ledger, model versions).
    pub keep_forever: Vec<String>,
    pub max_snapshots: usize,
    pub pruning_interval_hours: u64,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            file_changes_days: 30,
            agent_actions_days: 7,
            survival_events_days: 90,
            collaboration_events_hours: 24,
            compute_events_days: 30,
            keep_forever: vec![
                "CreditTransaction".into(),
                "ModelChange".into(),
                "Checkpoint".into(),
                "Reversion".into(),
            ],
            max_snapshots: 1000,
            pruning_interval_hours: 1,
        }
    }
}

impl RetentionPolicy {
    pub fn from_yaml(path: &std::path::Path) -> Self {
        // Silently fall back to defaults if the config file is absent or malformed.
        let Ok(text) = std::fs::read_to_string(path) else { return Self::default() };
        // Minimal YAML key extraction — avoids pulling in a YAML crate dependency.
        let mut policy = Self::default();
        for line in text.lines() {
            let line = line.trim();
            if let Some(v) = extract_u64(line, "file_changes_days:") { policy.file_changes_days = v; }
            if let Some(v) = extract_u64(line, "agent_actions_days:") { policy.agent_actions_days = v; }
            if let Some(v) = extract_u64(line, "survival_events_days:") { policy.survival_events_days = v; }
            if let Some(v) = extract_u64(line, "collaboration_events_hours:") { policy.collaboration_events_hours = v; }
            if let Some(v) = extract_u64(line, "compute_events_days:") { policy.compute_events_days = v; }
            if let Some(v) = extract_u64(line, "snapshots_keep_last_n:") { policy.max_snapshots = v as usize; }
            if let Some(v) = extract_u64(line, "check_interval_hours:") { policy.pruning_interval_hours = v; }
        }
        policy
    }
}

fn extract_u64(line: &str, key: &str) -> Option<u64> {
    line.strip_prefix(key).and_then(|v| v.trim().parse().ok())
}

/// Manages periodic snapshot creation and retention pruning.
pub struct SnapshotEngine {
    store: Arc<UniverseStore>,
    emitter: Arc<UniverseEmitter>,
    interval_secs: u64,
    pub retention: RetentionPolicy,
}

impl SnapshotEngine {
    pub fn new(store: Arc<UniverseStore>, emitter: Arc<UniverseEmitter>) -> Self {
        Self {
            store,
            emitter,
            interval_secs: 300,
            retention: RetentionPolicy::default(),
        }
    }

    pub fn with_interval(mut self, secs: u64) -> Self {
        self.interval_secs = secs;
        self
    }

    pub fn with_retention(mut self, policy: RetentionPolicy) -> Self {
        self.retention = policy;
        self
    }

    /// Start the periodic snapshot + pruning background task.
    pub fn spawn(self: Arc<Self>) {
        let engine = self.clone();
        tokio::spawn(async move {
            let mut snap_ticker = interval(Duration::from_secs(engine.interval_secs));
            let mut prune_ticker = interval(Duration::from_secs(
                engine.retention.pruning_interval_hours * 3600,
            ));
            loop {
                tokio::select! {
                    _ = snap_ticker.tick() => {
                        if let Err(e) = engine.take_snapshot(None, "auto".to_string()).await {
                            warn!("SnapshotEngine: auto-snapshot failed: {}", e);
                        }
                    }
                    _ = prune_ticker.tick() => {
                        engine.run_pruning().await;
                    }
                }
            }
        });
    }

    pub async fn take_snapshot(
        &self,
        label: Option<String>,
        trigger_event_id: impl Into<String>,
    ) -> Result<UniverseSnapshot, String> {
        let count = self.store.event_count().await;
        let state_hashes = self.collect_state_hashes().await;
        let trigger = trigger_event_id.into();

        let snapshot = UniverseSnapshot::new(label.clone(), trigger, state_hashes, count);
        self.store.insert_snapshot(&snapshot).await?;

        let mut event = UniverseEvent::new(
            EventSource::System { component: "SnapshotEngine".into() },
            EventCategory::Checkpoint,
            format!("Snapshot: {}", label.as_deref().unwrap_or("auto")),
            format!("snapshot:{}", snapshot.snapshot_id),
            self.store.device_id().to_string(),
        );
        event.metadata = serde_json::json!({
            "snapshot_id": snapshot.snapshot_id,
            "event_count": count,
            "label": label,
        });
        self.emitter.emit(event);

        info!("SnapshotEngine: snapshot {} ({} events)", snapshot.snapshot_id, count);
        Ok(snapshot)
    }

    /// Prune events older than their retention window.
    async fn run_pruning(&self) {
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        let day_ns = 86_400_000_000_000u64;
        let hour_ns = 3_600_000_000_000u64;

        let prunable: &[(&str, u64)] = &[
            ("FileChange",         self.retention.file_changes_days * day_ns),
            ("AgentAction",        self.retention.agent_actions_days * day_ns),
            ("SurvivalEvent",      self.retention.survival_events_days * day_ns),
            ("CollaborationEvent", self.retention.collaboration_events_hours * hour_ns),
            ("ComputeEvent",       self.retention.compute_events_days * day_ns),
            ("SwarmEvent",         self.retention.agent_actions_days * day_ns),
            ("ExtensionEvent",     self.retention.file_changes_days * day_ns),
        ];

        let mut total_pruned = 0u64;
        for (category, window_ns) in prunable {
            if *window_ns == 0 { continue; }
            let cutoff = now_ns.saturating_sub(*window_ns);
            let n = self.store.prune_before(cutoff, &[category]).await;
            if n > 0 {
                info!("SnapshotEngine: pruned {} {} events older than cutoff", n, category);
                total_pruned += n;
            }
        }
        if total_pruned > 0 {
            info!("SnapshotEngine: pruning complete — {} events removed", total_pruned);
        }
    }

    async fn collect_state_hashes(&self) -> SubsystemHashes {
        SubsystemHashes {
            workspace: "pending".into(),
            configuration: "pending".into(),
            model_registry: "pending".into(),
            agent_registry: "pending".into(),
            survival_kb: "pending".into(),
            training_state: "pending".into(),
            extension_registry: "pending".into(),
            collaboration_state: "pending".into(),
            compute_fabric_state: "pending".into(),
            credit_ledger: "pending".into(),
            issue_tracker: "pending".into(),
            universe_events_tip: blake3::hash(
                self.store.event_count().await.to_string().as_bytes()
            ).to_hex().to_string(),
        }
    }
}
