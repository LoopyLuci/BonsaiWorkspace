//! Agent role hierarchy and capability definitions.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Position in the agent hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmRole {
    /// Root overseer for a project. Decomposes goals into milestones, delegates to Managers.
    /// Does not execute tasks directly.
    ProjectManager,
    /// Domain owner (e.g. "Security Audit"). Receives milestones, breaks into sprints,
    /// assigns to SubManagers or Agents.
    Manager,
    /// Optional mid-level coordinator for complex sub-domains under a Manager.
    SubManager,
    /// Worker that executes individual tasks using tools and a local model.
    Agent,
    /// Ephemeral helper spawned by an Agent for a single fine-grained action.
    /// Cannot spawn further agents. Inherits parent permissions.
    SubAgent,
    /// Persistent personal AI advisor for any node in the hierarchy.
    /// Learns preferences, handles routine comms, cannot execute tools directly.
    Assistant,
}

impl SwarmRole {
    /// Maximum depth this role can delegate to.
    pub fn max_spawn_depth(&self) -> u8 {
        match self {
            SwarmRole::ProjectManager => 5,
            SwarmRole::Manager => 4,
            SwarmRole::SubManager => 3,
            SwarmRole::Agent => 1, // can spawn SubAgents only
            SwarmRole::SubAgent => 0,
            SwarmRole::Assistant => 0,
        }
    }

    pub fn can_spawn(&self) -> bool {
        self.max_spawn_depth() > 0
    }
}

impl std::fmt::Display for SwarmRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwarmRole::ProjectManager => write!(f, "ProjectManager"),
            SwarmRole::Manager => write!(f, "Manager"),
            SwarmRole::SubManager => write!(f, "SubManager"),
            SwarmRole::Agent => write!(f, "Agent"),
            SwarmRole::SubAgent => write!(f, "SubAgent"),
            SwarmRole::Assistant => write!(f, "Assistant"),
        }
    }
}

/// A specific capability an agent can advertise to the capability registry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Capability {
    /// Kind: "tool", "knowledge", "effect", "model"
    pub kind: String,
    /// Specific name: e.g. "execute_code", "rust_borrow_checker", "NetworkIO"
    pub name: String,
}

impl Capability {
    pub fn tool(name: impl Into<String>) -> Self {
        Self { kind: "tool".into(), name: name.into() }
    }
    pub fn knowledge(domain: impl Into<String>) -> Self {
        Self { kind: "knowledge".into(), name: domain.into() }
    }
    pub fn effect(name: impl Into<String>) -> Self {
        Self { kind: "effect".into(), name: name.into() }
    }
    pub fn model(name: impl Into<String>) -> Self {
        Self { kind: "model".into(), name: name.into() }
    }
}

/// An agent's advertised profile in the capability registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub agent_id: Uuid,
    pub role: SwarmRole,
    pub display_name: String,
    pub capabilities: Vec<Capability>,
    /// Credits per minute (0.0 for local agents).
    pub cost_per_minute: f64,
    /// Historical success rate 0.0–1.0.
    pub reliability: f64,
    /// Current load 0.0–1.0.
    pub current_load: f64,
    /// Whether this agent is on a remote device.
    pub is_remote: bool,
}
