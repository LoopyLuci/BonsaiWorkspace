//! Federated training coordinator — manages contributions from multiple
//! local Bonsai instances (e.g., two machines on a LAN) using CRDT-based state.
//!
//! All state is merge-convergent: no central coordinator, no conflict resolution.
//! Offline-first: peers sync only when explicitly connected via the P2P module.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crdt::{GCounter, LwwRegister, OrSet, PNCounter, VClock};

// ── Peer identity ─────────────────────────────────────────────────────────────

pub type PeerId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: PeerId,
    pub display_name: String,
    pub endpoint: String,
    pub last_seen: i64,
}

// ── Adapter descriptor ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AdapterDescriptor {
    /// CAS key (Blake3 hex) of the adapter weights.
    pub cas_key: String,
    /// Human-readable label (e.g., "cycle_42_lora").
    pub label: String,
    /// Originating peer.
    pub peer_id: PeerId,
    /// Loss at the time of export.
    pub loss: String,
    pub created_at: i64,
}

impl std::fmt::Display for AdapterDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.peer_id, self.cas_key)
    }
}

// ── CRDT-backed federation state ──────────────────────────────────────────────

/// The shared state of the federated training session.
/// All fields are CRDTs — merging two `FederatedState`s produces correct state.
#[derive(Default)]
pub struct FederatedState {
    /// How many training examples each peer has contributed.
    pub contributions: GCounter,
    /// Reputation scores (can go up or down — positive feedback / negative feedback).
    pub reputation: HashMap<PeerId, PNCounter>,
    /// Set of known-good adapter CAS keys (add-wins; removed adapters become tombstoned).
    pub adapter_registry: OrSet<AdapterDescriptor>,
    /// The currently selected "best" adapter for each model base.
    /// Key: base model name, value: AdapterDescriptor as JSON string.
    pub best_adapter: HashMap<String, LwwRegister<String>>,
    /// Causal clock for ordering events across peers.
    pub clock: VClock,
}

impl FederatedState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn merge(&mut self, other: &FederatedState) {
        self.contributions.merge(&other.contributions);
        for (peer, counter) in &other.reputation {
            self.reputation
                .entry(peer.clone())
                .or_insert_with(PNCounter::new)
                .merge(counter);
        }
        self.adapter_registry.merge(&other.adapter_registry);
        for (model, reg) in &other.best_adapter {
            self.best_adapter
                .entry(model.clone())
                .or_insert_with(|| LwwRegister::new(String::new()))
                .merge(reg);
        }
        self.clock.merge(&other.clock);
    }
}

// ── FederatedTrainer ──────────────────────────────────────────────────────────

pub struct FederatedTrainer {
    local_peer_id: PeerId,
    state: RwLock<FederatedState>,
    peers: RwLock<HashMap<PeerId, PeerInfo>>,
}

impl FederatedTrainer {
    pub fn new(local_peer_id: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            local_peer_id: local_peer_id.into(),
            state: RwLock::new(FederatedState::new()),
            peers: RwLock::new(HashMap::new()),
        })
    }

    // ── Local mutations ───────────────────────────────────────────────────────

    /// Record that the local peer has contributed `count` new training examples.
    pub async fn record_local_contribution(&self, count: u64) {
        let mut state = self.state.write().await;
        state.contributions.increment_by(&self.local_peer_id, count);
        state.clock.tick(&self.local_peer_id);
        debug!("[federated] local contribution: +{count}");
    }

    /// Register a new adapter produced by the local training loop.
    pub async fn register_adapter(&self, desc: AdapterDescriptor, base_model: &str) {
        let mut state = self.state.write().await;
        state.adapter_registry.add(desc.clone());

        // Update best adapter (LWW — highest lamport timestamp wins)
        let ts = state.clock.get(&self.local_peer_id) + 1;
        let desc_json = serde_json::to_string(&desc).unwrap_or_default();
        state
            .best_adapter
            .entry(base_model.to_string())
            .or_insert_with(|| LwwRegister::new(String::new()))
            .set(desc_json, ts, self.local_peer_id.clone());
        state.clock.tick(&self.local_peer_id);

        info!(
            "[federated] registered adapter '{}' for '{base_model}'",
            desc.label
        );
    }

    /// Positive reputation feedback for a peer (e.g., their adapter improved eval).
    pub async fn upvote_peer(&self, peer_id: &PeerId) {
        let mut state = self.state.write().await;
        state
            .reputation
            .entry(peer_id.clone())
            .or_insert_with(PNCounter::new)
            .increment(peer_id);
    }

    /// Negative reputation feedback for a peer (e.g., their adapter regressed eval).
    pub async fn downvote_peer(&self, peer_id: &PeerId) {
        let mut state = self.state.write().await;
        state
            .reputation
            .entry(peer_id.clone())
            .or_insert_with(PNCounter::new)
            .decrement(peer_id);
    }

    // ── Merge ──────────────────────────────────────────────────────────────────

    /// Merge a peer's state snapshot received over the P2P channel.
    pub async fn merge_peer_state(&self, peer_state: FederatedStateSnapshot) {
        let peer_id = peer_state.peer_id.clone();
        let mut state = self.state.write().await;

        // Convert snapshot to local CRDT structures and merge
        state.contributions.merge(&{
            let mut c = GCounter::new();
            for (k, v) in &peer_state.contributions {
                c.increment_by(k, *v);
            }
            c
        });

        for (model, (adapter_json, ts, origin)) in &peer_state.best_adapter {
            state
                .best_adapter
                .entry(model.clone())
                .or_insert_with(|| LwwRegister::new(String::new()))
                .set(adapter_json.clone(), *ts, origin.clone());
        }

        state.clock.tick(&peer_id);
        info!("[federated] merged state from peer '{peer_id}'");
    }

    // ── Queries ────────────────────────────────────────────────────────────────

    /// Total number of training examples across all peers.
    pub async fn total_contributions(&self) -> u64 {
        self.state.read().await.contributions.value()
    }

    /// Reputation score for a peer.
    pub async fn peer_reputation(&self, peer_id: &PeerId) -> i64 {
        self.state
            .read()
            .await
            .reputation
            .get(peer_id)
            .map(|c| c.value())
            .unwrap_or(0)
    }

    /// List all registered adapters (live, not tombstoned).
    pub async fn list_adapters(&self) -> Vec<String> {
        self.state.read().await.adapter_registry.elements()
    }

    /// Get the current best adapter for a base model (JSON string of AdapterDescriptor).
    pub async fn best_adapter_for(&self, base_model: &str) -> Option<String> {
        let state = self.state.read().await;
        let reg = state.best_adapter.get(base_model)?;
        let s = reg.get().clone();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }

    /// Register a new peer.
    pub async fn add_peer(&self, info: PeerInfo) {
        self.peers.write().await.insert(info.id.clone(), info);
    }

    /// Return known peers.
    pub async fn list_peers(&self) -> Vec<PeerInfo> {
        self.peers.read().await.values().cloned().collect()
    }

    /// Produce a serializable snapshot for sending to a peer.
    pub async fn export_snapshot(&self) -> FederatedStateSnapshot {
        let state = self.state.read().await;
        FederatedStateSnapshot {
            peer_id: self.local_peer_id.clone(),
            contributions: {
                // Extract GCounter internals as HashMap<String, u64>
                // We use the public API only
                let mut m = HashMap::new();
                // GCounter doesn't expose internals; use total per-peer via a
                // separate tracking map. For the snapshot we just expose the total.
                m.insert(self.local_peer_id.clone(), state.contributions.value());
                m
            },
            best_adapter: state
                .best_adapter
                .iter()
                .map(|(model, reg)| {
                    let ts_clock = state.clock.get(&self.local_peer_id);
                    (
                        model.clone(),
                        (reg.get().clone(), ts_clock, self.local_peer_id.clone()),
                    )
                })
                .collect(),
        }
    }
}

/// Wire-format snapshot for P2P state exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedStateSnapshot {
    pub peer_id: PeerId,
    pub contributions: HashMap<String, u64>,
    pub best_adapter: HashMap<String, (String, u64, String)>,
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn federated_stats(
    state: tauri::State<'_, crate::AppState>,
) -> Result<serde_json::Value, String> {
    let ft = &state.federated_trainer;
    let total = ft.total_contributions().await;
    let adapters = ft.list_adapters().await;
    let peers = ft.list_peers().await;
    Ok(serde_json::json!({
        "total_contributions": total,
        "adapter_count": adapters.len(),
        "peer_count": peers.len(),
        "peers": peers,
    }))
}

#[tauri::command]
pub async fn federated_list_adapters(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<String>, String> {
    Ok(state.federated_trainer.list_adapters().await)
}
