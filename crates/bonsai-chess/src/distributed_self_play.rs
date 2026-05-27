//! Distributed multi-machine chess self-play using CRDT state convergence.
//!
//! Each node runs a `DistributedSelfPlayWorker` actor that plays games locally,
//! then syncs `GameRecord`s with peers via HTTP. CRDT primitives ensure
//! convergence without coordination.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use bonsai_crdt::{GCounter, LwwRegister, OrSet, VClock};
use bonsai_actors::{Actor, ActorContext, ActorRef, ActorSystem};

use crate::mcts::{MctsConfig, TrainingExample, self_play_game};
use crate::network::NetworkEvaluator;

// ── GameRecord ────────────────────────────────────────────────────────────────

/// A completed self-play game stored in the distributed set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GameRecord {
    pub id: Uuid,
    pub node_id: String,
    pub winner: Option<String>, // "white" | "black" | None (draw)
    pub move_count: u32,
    pub timestamp_ms: u64,
    /// Serialised training examples (policy + value targets per position).
    pub examples_json: String,
}

impl std::fmt::Display for GameRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl GameRecord {
    pub fn new(node_id: impl Into<String>, examples: &[TrainingExample], winner: Option<&str>) -> Self {
        let examples_json = serde_json::to_string(examples).unwrap_or_default();
        Self {
            id: Uuid::new_v4(),
            node_id: node_id.into(),
            winner: winner.map(|s| s.to_string()),
            move_count: examples.len() as u32,
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            examples_json,
        }
    }

    /// Deserialise training examples from the stored JSON.
    pub fn to_training_examples(&self) -> Vec<TrainingExample> {
        serde_json::from_str(&self.examples_json).unwrap_or_default()
    }

    /// Convert to DPO (direct preference optimisation) pairs for language model fine-tuning.
    pub fn to_dpo_pairs(&self) -> Vec<DpoPair> {
        let examples = self.to_training_examples();
        examples.iter().enumerate().filter_map(|(i, ex)| {
            // Best move = highest MCTS probability
            let best_move = ex.move_probs.iter()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(m, _)| m.clone())?;
            let value = ex.game_result.unwrap_or(0.0);
            let chosen = format!(
                "Position {}: best move {} (value estimate {:.2})",
                i, best_move, value
            );
            let rejected = format!(
                "Position {}: random move (value estimate {:.2})",
                i, -value
            );
            Some(DpoPair { chosen, rejected })
        }).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpoPair {
    pub chosen: String,
    pub rejected: String,
}

// ── Distributed state ─────────────────────────────────────────────────────────

/// CRDT-based distributed state shared across nodes.
#[derive(Clone, Serialize, Deserialize)]
pub struct DistributedSelfPlayState {
    /// All game records observed across the cluster (add-wins OR-Set).
    pub games: OrSet<GameRecord>,
    /// Games played per node.
    pub games_played: GCounter,
    /// Cluster config: current MCTS simulations count.
    pub mcts_sims: LwwRegister<u32>,
    /// Causal clock for this state snapshot.
    pub clock: VClock,
}

impl DistributedSelfPlayState {
    pub fn new(initial_sims: u32) -> Self {
        Self {
            games: OrSet::new(),
            games_played: GCounter::new(),
            mcts_sims: LwwRegister::new(initial_sims),
            clock: VClock::new(),
        }
    }

    pub fn merge(&mut self, other: &Self) {
        self.games.merge(&other.games);
        self.games_played.merge(&other.games_played);
        self.mcts_sims.merge(&other.mcts_sims);
        self.clock.merge(&other.clock);
    }

    pub fn total_games(&self) -> u64 {
        self.games_played.value()
    }
}

// ── Actor messages ────────────────────────────────────────────────────────────

pub enum SelfPlayMsg {
    /// Play one game locally and store the record.
    PlayGame,
    /// Merge incoming state from a peer.
    MergePeer(DistributedSelfPlayState),
    /// Retrieve current state (request-reply).
    GetState(tokio::sync::oneshot::Sender<DistributedSelfPlayState>),
    /// Retrieve recent training examples.
    GetExamples(tokio::sync::oneshot::Sender<Vec<TrainingExample>>),
}

// ── DistributedSelfPlayWorker actor ──────────────────────────────────────────

pub struct DistributedSelfPlayWorker {
    node_id: String,
    state: Arc<RwLock<DistributedSelfPlayState>>,
    recent_examples: Vec<TrainingExample>,
    mcts_config: MctsConfig,
}

impl DistributedSelfPlayWorker {
    pub fn new(node_id: impl Into<String>, mcts_sims: u32) -> Self {
        let node_id = node_id.into();
        Self {
            node_id: node_id.clone(),
            state: Arc::new(RwLock::new(DistributedSelfPlayState::new(mcts_sims))),
            recent_examples: Vec::new(),
            mcts_config: MctsConfig {
                num_simulations: mcts_sims,
                ..MctsConfig::default()
            },
        }
    }

    async fn play_one_game(&mut self) {
        let cfg = self.mcts_config.clone();
        let node_id = self.node_id.clone();
        let state_arc = self.state.clone();

        let result = tokio::task::spawn_blocking(move || {
            let evaluator = NetworkEvaluator::load_default();
            self_play_game(&evaluator, &cfg, &[])
        }).await;

        if let Ok(examples) = result {
            // Determine winner from last game_result
            let winner = examples.last()
                .and_then(|ex| ex.game_result)
                .map(|r| if r > 0.0 { "white" } else if r < 0.0 { "black" } else { "" })
                .filter(|s| !s.is_empty());

            let record = GameRecord::new(&node_id, &examples, winner);
            {
                let mut st = state_arc.write().await;
                st.games.add(record.clone());
                st.games_played.increment(&node_id);
                st.clock.tick(&node_id);
            }
            self.recent_examples.extend(examples);
            // Cap at 10_000 examples to bound memory
            if self.recent_examples.len() > 10_000 {
                self.recent_examples.drain(0..5_000);
            }
            tracing::info!(node=%node_id, "self-play game complete, winner={:?}", record.winner);
        }
    }
}

#[async_trait::async_trait]
impl Actor for DistributedSelfPlayWorker {
    type Msg = SelfPlayMsg;

    async fn on_start(&mut self, _ctx: &mut ActorContext) {
        tracing::info!(node=%self.node_id, "DistributedSelfPlayWorker started");
    }

    async fn receive(&mut self, msg: SelfPlayMsg, _ctx: &mut ActorContext) {
        match msg {
            SelfPlayMsg::PlayGame => {
                self.play_one_game().await;
            }
            SelfPlayMsg::MergePeer(peer_state) => {
                let mut st = self.state.write().await;
                st.merge(&peer_state);
            }
            SelfPlayMsg::GetState(reply) => {
                let st = self.state.read().await.clone();
                let _ = reply.send(st);
            }
            SelfPlayMsg::GetExamples(reply) => {
                let _ = reply.send(self.recent_examples.clone());
            }
        }
    }

    async fn on_stop(&mut self) {
        tracing::info!(node=%self.node_id, "DistributedSelfPlayWorker stopping");
    }
}

// ── PeerSyncManager ───────────────────────────────────────────────────────────

/// Manages state sync with discovered peers via simple HTTP polling.
/// Worker ref is stored as the raw channel to avoid clone trait issues.
pub struct PeerSyncManager {
    node_id: String,
    peers: Arc<RwLock<Vec<String>>>, // peer base URLs: "http://host:port"
    worker_tx: tokio::sync::mpsc::UnboundedSender<SelfPlayMsg>,
}

impl PeerSyncManager {
    pub fn new(
        node_id: impl Into<String>,
        worker_tx: tokio::sync::mpsc::UnboundedSender<SelfPlayMsg>,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            peers: Arc::new(RwLock::new(Vec::new())),
            worker_tx,
        }
    }

    pub async fn add_peer(&self, url: impl Into<String>) {
        self.peers.write().await.push(url.into());
    }

    /// Background sync loop: sync every `interval_secs` seconds.
    pub async fn run_sync_loop(self: Arc<Self>, interval_secs: u64) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        loop {
            interval.tick().await;
            // Request our state, then push to peers
            let (tx, rx) = tokio::sync::oneshot::channel();
            if self.worker_tx.send(SelfPlayMsg::GetState(tx)).is_err() { break; }
            let our_state = match rx.await { Ok(s) => s, Err(_) => break };

            let peers = self.peers.read().await.clone();
            for peer_url in &peers {
                // In a real deployment: POST our_state to peer, GET peer state
                // Skipping actual HTTP here — implement via axum handler at the app layer
                tracing::debug!(node=%self.node_id, peer=%peer_url, "sync tick (HTTP disabled in lib)");
                drop(&our_state); // suppress unused warning in test builds
                break;
            }
        }
    }
}

// ── DistributedSelfPlayEngine ─────────────────────────────────────────────────

/// Top-level entry point. Creates the actor and exposes the current state
/// for the REST handler at `/api/v2/selfplay/state`.
pub struct DistributedSelfPlayEngine {
    pub worker: ActorRef<SelfPlayMsg>,
    pub state_arc: Arc<RwLock<DistributedSelfPlayState>>,
}

impl DistributedSelfPlayEngine {
    pub fn start(system: &Arc<ActorSystem>, node_id: impl Into<String> + Clone, mcts_sims: u32) -> Self {
        let worker_actor = DistributedSelfPlayWorker::new(node_id.clone(), mcts_sims);
        let state_arc = worker_actor.state.clone();
        let worker = system.spawn(worker_actor);

        Self { worker, state_arc }
    }

    /// Trigger a self-play game (fire-and-forget).
    pub fn play_game(&self) {
        let _ = self.worker.send(SelfPlayMsg::PlayGame);
    }

    /// Get current CRDT state (for REST export).
    pub async fn get_state(&self) -> DistributedSelfPlayState {
        let (tx, rx) = tokio::sync::oneshot::channel();
        if self.worker.send(SelfPlayMsg::GetState(tx)).is_ok() {
            rx.await.unwrap_or_else(|_| DistributedSelfPlayState::new(100))
        } else {
            self.state_arc.read().await.clone()
        }
    }

    /// Merge incoming peer state (called from REST POST handler).
    pub fn merge_peer(&self, state: DistributedSelfPlayState) {
        let _ = self.worker.send(SelfPlayMsg::MergePeer(state));
    }

    /// Collect recent training examples from this node.
    pub async fn get_examples(&self) -> Vec<TrainingExample> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        if self.worker.send(SelfPlayMsg::GetExamples(tx)).is_ok() {
            rx.await.unwrap_or_default()
        } else {
            vec![]
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_record_display_is_uuid() {
        let record = GameRecord::new("node-1", &[], Some("white"));
        let s = record.to_string();
        assert_eq!(s.len(), 36); // UUID string length
    }

    #[test]
    fn state_merge_is_idempotent() {
        let mut a = DistributedSelfPlayState::new(100);
        let b = DistributedSelfPlayState::new(100);
        a.merge(&b);
        a.merge(&b);
        assert_eq!(a.total_games(), 0);
    }

    #[test]
    fn game_record_dpo_pairs_empty_examples() {
        let record = GameRecord::new("node-x", &[], None);
        let pairs = record.to_dpo_pairs();
        assert!(pairs.is_empty());
    }

    #[tokio::test]
    async fn engine_starts_and_get_state() {
        let system = ActorSystem::new();
        let engine = DistributedSelfPlayEngine::start(&system, "test-node", 1);
        let state = engine.get_state().await;
        assert_eq!(state.total_games(), 0);
    }
}
