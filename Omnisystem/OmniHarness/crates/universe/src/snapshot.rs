use crate::event::{EventCategory, EventSource, SubsystemHashes, UniverseEvent, UniverseSnapshot};
use crate::emitter::UniverseEmitter;
use crate::store::UniverseStore;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::info;

/// Manages periodic snapshot creation and retention pruning.
pub struct SnapshotEngine {
    store: Arc<UniverseStore>,
    emitter: Arc<UniverseEmitter>,
    interval_secs: u64,
    max_snapshots: usize,
}

impl SnapshotEngine {
    pub fn new(store: Arc<UniverseStore>, emitter: Arc<UniverseEmitter>) -> Self {
        Self { store, emitter, interval_secs: 300, max_snapshots: 1000 }
    }

    pub fn with_interval(mut self, secs: u64) -> Self {
        self.interval_secs = secs;
        self
    }

    pub fn with_max_snapshots(mut self, n: usize) -> Self {
        self.max_snapshots = n;
        self
    }

    /// Start the periodic snapshot background task.
    pub fn spawn(self: Arc<Self>) {
        let engine = self.clone();
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(engine.interval_secs));
            loop {
                ticker.tick().await;
                if let Err(e) = engine.take_snapshot(None, "auto".to_string()).await {
                    tracing::warn!("SnapshotEngine: auto-snapshot failed: {}", e);
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

        let snapshot = UniverseSnapshot::new(
            label.clone(),
            trigger.clone(),
            state_hashes,
            count,
        );

        self.store.insert_snapshot(&snapshot).await?;

        // Emit a Checkpoint event so the timeline shows it
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

        info!("SnapshotEngine: snapshot {} created ({} events)", snapshot.snapshot_id, count);
        Ok(snapshot)
    }

    async fn collect_state_hashes(&self) -> SubsystemHashes {
        // Phase 1: placeholder hashes. Phase 3 wires real subsystem queries.
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
