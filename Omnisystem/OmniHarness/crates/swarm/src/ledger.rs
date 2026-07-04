//! Swarm Ledger — append-only, hash-chained audit trail.
//!
//! Every significant swarm action is recorded: task assignments, tool invocations,
//! decisions, results, and errors. The chain enables traceability, compliance auditing,
//! and serves as training data for the Eternal Training Loop.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// The kind of event recorded in the ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerEventKind {
    SwarmCreated,
    SwarmCompleted,
    SwarmFailed,
    TaskAssigned { agent_id: Uuid },
    TaskStarted { agent_id: Uuid },
    TaskCompleted { agent_id: Uuid },
    TaskFailed { agent_id: Uuid, reason: String },
    TaskRetried { agent_id: Uuid, attempt: u32 },
    AgentSpawned { role: String },
    AgentStopped { reason: String },
    ToolInvoked { tool: String, agent_id: Uuid },
    ToolResult { tool: String, agent_id: Uuid, success: bool },
    PlanCreated { task_count: usize },
    PlanVerified,
    PlanRejected { reason: String },
    AssistantSuggestion { agent_id: Uuid, suggestion: String },
    CapabilityNegotiation { requester: Uuid, provider: Uuid, accepted: bool },
    AgentMigrated { from_device: String, to_device: String },
    UserApproval { approved: bool },
    Custom { tag: String, payload: serde_json::Value },
}

/// A single entry in the swarm ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    /// Sequential index within the swarm's ledger.
    pub seq: u64,
    pub swarm_id: Uuid,
    pub task_id: Option<Uuid>,
    pub event: LedgerEventKind,
    pub timestamp: DateTime<Utc>,
    /// BLAKE3 hash of `prev_hash || seq || swarm_id || event_json`.
    pub hash: String,
    pub prev_hash: String,
}

/// In-memory append-only ledger for a single swarm.
/// In production this would persist to SQLite / CAS.
#[derive(Clone)]
pub struct SwarmLedger {
    swarm_id: Uuid,
    entries: Arc<RwLock<Vec<LedgerEntry>>>,
}

impl SwarmLedger {
    pub fn new(swarm_id: Uuid) -> Self {
        Self {
            swarm_id,
            entries: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Append an event. Returns the new entry's sequence number.
    pub async fn append(&self, event: LedgerEventKind, task_id: Option<Uuid>) -> u64 {
        let mut entries = self.entries.write().await;
        let seq = entries.len() as u64;
        let prev_hash = entries
            .last()
            .map(|e| e.hash.clone())
            .unwrap_or_else(|| "genesis".to_string());

        let event_json = serde_json::to_string(&event).unwrap_or_default();
        let hash_input = format!("{prev_hash}|{seq}|{}|{event_json}", self.swarm_id);
        let hash = blake3::hash(hash_input.as_bytes()).to_hex().to_string();

        entries.push(LedgerEntry {
            seq,
            swarm_id: self.swarm_id,
            task_id,
            event,
            timestamp: Utc::now(),
            hash,
            prev_hash,
        });
        seq
    }

    /// Recent N entries (for dashboard display).
    pub async fn tail(&self, n: usize) -> Vec<LedgerEntry> {
        let entries = self.entries.read().await;
        let len = entries.len();
        let start = len.saturating_sub(n);
        entries[start..].to_vec()
    }

    /// All entries (for full audit).
    pub async fn all(&self) -> Vec<LedgerEntry> {
        self.entries.read().await.clone()
    }

    pub async fn len(&self) -> usize {
        self.entries.read().await.len()
    }
}
