use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventSource {
    User { peer_id: String },
    Agent { agent_id: String, swarm_id: String },
    System { component: String },
    Survival { trigger_event_id: String },
    Training { phase: String },
    Automation { rule_name: String },
    External { origin: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventCategory {
    FileChange,
    ConfigChange,
    ModelChange,
    AgentAction,
    SwarmEvent,
    CollaborationEvent,
    ComputeEvent,
    ExtensionEvent,
    SurvivalEvent,
    CreditTransaction,
    Checkpoint,
    Reversion,
}

impl EventCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventCategory::FileChange => "FileChange",
            EventCategory::ConfigChange => "ConfigChange",
            EventCategory::ModelChange => "ModelChange",
            EventCategory::AgentAction => "AgentAction",
            EventCategory::SwarmEvent => "SwarmEvent",
            EventCategory::CollaborationEvent => "CollaborationEvent",
            EventCategory::ComputeEvent => "ComputeEvent",
            EventCategory::ExtensionEvent => "ExtensionEvent",
            EventCategory::SurvivalEvent => "SurvivalEvent",
            EventCategory::CreditTransaction => "CreditTransaction",
            EventCategory::Checkpoint => "Checkpoint",
            EventCategory::Reversion => "Reversion",
        }
    }
}

impl std::fmt::Display for EventCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// The fundamental unit of time travel. Every mutation produces one or more events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniverseEvent {
    /// BLAKE3 hash of all other fields — serves as the unique event ID.
    pub event_id: String,
    /// Nanosecond-precision timestamp.
    pub timestamp_ns: u64,
    /// What triggered this change.
    pub source: EventSource,
    /// What kind of change occurred.
    pub category: EventCategory,
    /// Human-readable description.
    pub summary: String,
    /// What was changed (file path, config key, model name, agent ID, …).
    pub target: String,
    /// CAS hash of state BEFORE the change.
    pub before_hash: Option<String>,
    /// CAS hash of state AFTER the change.
    pub after_hash: Option<String>,
    /// CAS hash of the minimal binary delta.
    pub delta_hash: Option<String>,
    /// Arbitrary JSON context for rich querying.
    pub metadata: serde_json::Value,
    /// Causal chain — event IDs that logically preceded this one.
    pub parent_event_ids: Vec<String>,
    /// Ed25519 signature bytes (user-triggered events).
    pub signature: Option<Vec<u8>>,
    /// Device that recorded this event.
    pub device_id: String,
}

impl UniverseEvent {
    pub fn new(
        source: EventSource,
        category: EventCategory,
        summary: impl Into<String>,
        target: impl Into<String>,
        device_id: impl Into<String>,
    ) -> Self {
        let summary = summary.into();
        let target = target.into();
        let device_id = device_id.into();
        let timestamp_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        // Compute event_id as BLAKE3 of key fields
        let fingerprint = format!("{timestamp_ns}:{target}:{summary}");
        let event_id = blake3::hash(fingerprint.as_bytes()).to_hex().to_string();

        Self {
            event_id,
            timestamp_ns,
            source,
            category,
            summary,
            target,
            before_hash: None,
            after_hash: None,
            delta_hash: None,
            metadata: serde_json::Value::Null,
            parent_event_ids: Vec::new(),
            signature: None,
            device_id,
        }
    }

    pub fn with_hashes(mut self, before: Option<String>, after: Option<String>) -> Self {
        self.before_hash = before;
        self.after_hash = after;
        self
    }

    pub fn with_delta(mut self, delta_hash: String) -> Self {
        self.delta_hash = Some(delta_hash);
        self
    }

    pub fn with_metadata(mut self, meta: serde_json::Value) -> Self {
        self.metadata = meta;
        self
    }

    pub fn with_parents(mut self, parents: Vec<String>) -> Self {
        self.parent_event_ids = parents;
        self
    }
}

/// Hashes of each subsystem's state at snapshot time.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SubsystemHashes {
    pub workspace: String,
    pub configuration: String,
    pub model_registry: String,
    pub agent_registry: String,
    pub survival_kb: String,
    pub training_state: String,
    pub extension_registry: String,
    pub collaboration_state: String,
    pub compute_fabric_state: String,
    pub credit_ledger: String,
    pub issue_tracker: String,
    pub universe_events_tip: String,
}

/// Periodic full-state snapshot enabling fast reversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniverseSnapshot {
    pub snapshot_id: String,
    pub timestamp_ns: u64,
    pub label: Option<String>,
    pub trigger_event_id: String,
    pub state_hashes: SubsystemHashes,
    pub event_count_at_creation: u64,
}

impl UniverseSnapshot {
    pub fn new(
        label: Option<String>,
        trigger_event_id: impl Into<String>,
        state_hashes: SubsystemHashes,
        event_count: u64,
    ) -> Self {
        let trigger_event_id = trigger_event_id.into();
        let timestamp_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let fingerprint = format!("{timestamp_ns}:{trigger_event_id}");
        let snapshot_id = blake3::hash(fingerprint.as_bytes()).to_hex().to_string();
        Self {
            snapshot_id,
            timestamp_ns,
            label,
            trigger_event_id,
            state_hashes,
            event_count_at_creation: event_count,
        }
    }
}

/// Query filter for timeline queries.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimelineFilter {
    pub category: Option<EventCategory>,
    pub target_prefix: Option<String>,
    pub since_ns: Option<u64>,
    pub until_ns: Option<u64>,
    pub limit: Option<usize>,
}

/// Preview of what a revert operation will change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevertPreview {
    pub target_event_id: Option<String>,
    pub target_snapshot_id: Option<String>,
    pub affected_files: Vec<String>,
    pub affected_configs: Vec<String>,
    pub model_changes: Vec<String>,
    pub agent_changes: Vec<String>,
    pub event_count_to_undo: u64,
    pub estimated_duration_ms: u64,
}
