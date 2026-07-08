use crate::campaign::{CampaignSpec, CampaignState, CampaignStatus, FuzzStrategy};
use crate::survival_bridge::SurvivalBridge;
use crate::worker::{FailureReport, FuzzWorker};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorStats {
    pub active_campaigns:    usize,
    pub total_iterations:    u64,
    pub total_crashes:       usize,
    pub rules_added_to_kb:   usize,
}

pub struct F3Orchestrator {
    campaigns:    Arc<DashMap<Uuid, CampaignState>>,
    bridge:       Arc<SurvivalBridge>,
    failures_tx:  mpsc::Sender<FailureReport>,
    stats:        Arc<RwLock<OrchestratorStats>>,
}

impl F3Orchestrator {
    pub fn new(bridge: Arc<SurvivalBridge>) -> Arc<Self> {
        let (tx, mut rx) = mpsc::channel::<FailureReport>(1024);
        let campaigns = Arc::new(DashMap::new());
        let stats = Arc::new(RwLock::new(OrchestratorStats {
            active_campaigns: 0,
            total_iterations: 0,
            total_crashes: 0,
            rules_added_to_kb: 0,
        }));

        let bridge_clone = bridge.clone();
        let stats_clone = stats.clone();

        // Background: receive failure reports and persist to Survival KB
        tokio::spawn(async move {
            while let Some(failure) = rx.recv().await {
                let added = bridge_clone.report_failure(&failure).await;
                let mut s = stats_clone.write().await;
                s.total_crashes += 1;
                if added { s.rules_added_to_kb += 1; }
            }
        });

        Arc::new(Self { campaigns, bridge, failures_tx: tx, stats })
    }

    /// Queue a new campaign. Returns the campaign ID.
    pub async fn start_campaign(self: Arc<Self>, spec: CampaignSpec) -> Uuid {
        let id = spec.id;
        let mut state = CampaignState::new(spec);
        state.status = CampaignStatus::Running;
        state.started_at = Some(chrono::Utc::now());
        self.campaigns.insert(id, state.clone());

        let orch = self.clone();
        tokio::spawn(async move {
            info!("F³ campaign starting: {} ({})", state.spec.name, id);
            orch.run_campaign(state).await;
        });

        id
    }

    async fn run_campaign(&self, state: CampaignState) {
        let spec = &state.spec;
        let timeout = Duration::from_secs(spec.resources.timeout_secs);
        let mut tasks = Vec::new();

        for target in &spec.targets {
            for strategy in &spec.strategies {
                let max_iterations = match strategy {
                    FuzzStrategy::InputFuzzing { iterations, .. } => *iterations,
                    FuzzStrategy::PropertyBased { samples, .. } => *samples,
                    FuzzStrategy::StateFuzzing { sequences, .. } => *sequences,
                    FuzzStrategy::Adversarial { rounds, .. } => *rounds,
                    _ => 1_000,
                };

                let seed = rand::random::<u64>();
                let mut worker = FuzzWorker::new(
                    spec.id.to_string(),
                    target.clone(),
                    strategy.clone(),
                    seed,
                );
                let tx = self.failures_tx.clone();
                let t = timeout;

                let h = tokio::spawn(async move {
                    let failures = worker.run(max_iterations, t).await;
                    for f in failures {
                        let _ = tx.send(f).await;
                    }
                    worker.failure_count()
                });
                tasks.push(h);
            }
        }

        let mut total_crashes = 0;
        let mut total_iters = 0u64;
        for t in tasks {
            if let Ok(crashes) = t.await {
                total_crashes += crashes;
                total_iters += 1_000; // approximate
            }
        }

        {
            let mut s = self.stats.write().await;
            s.total_iterations += total_iters;
            s.active_campaigns = s.active_campaigns.saturating_sub(1);
        }

        if let Some(mut entry) = self.campaigns.get_mut(&state.spec.id) {
            entry.status = CampaignStatus::Completed;
            entry.completed_at = Some(chrono::Utc::now());
            entry.crashes_found = total_crashes;
            entry.iterations_done = total_iters;
        }

        info!("F³ campaign complete: {} — {} crashes found", state.spec.name, total_crashes);
    }

    pub async fn stop_campaign(&self, id: Uuid) -> bool {
        if let Some(mut entry) = self.campaigns.get_mut(&id) {
            entry.status = CampaignStatus::Paused;
            return true;
        }
        false
    }

    pub fn list_campaigns(&self) -> Vec<CampaignState> {
        self.campaigns.iter().map(|e| e.value().clone()).collect()
    }

    pub fn list_failures(&self) -> Vec<FailureReport> {
        // Failures are forwarded to the bridge; query the bridge for history
        self.bridge.recent_failures()
    }

    pub async fn stats(&self) -> OrchestratorStats {
        self.stats.read().await.clone()
    }
}
