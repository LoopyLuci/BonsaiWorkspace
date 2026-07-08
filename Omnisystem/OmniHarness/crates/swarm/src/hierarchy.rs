//! Hierarchical agent tree — the org-chart of a swarm.
//!
//! Each node in the tree corresponds to an agent at a specific role level.
//! The tree is the authoritative source for parent/child relationships,
//! used by the Swarm Commander UI for rendering and by the orchestrator
//! for delegation routing.

use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::role::SwarmRole;

/// Runtime status of a hierarchy node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Initialising,
    Idle,
    Working,
    Paused,
    Migrating,
    Error { reason: String },
    Stopped,
}

/// A single node in the agent hierarchy tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchyNode {
    pub id: Uuid,
    pub swarm_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub role: SwarmRole,
    pub display_name: String,
    /// Domain/specialty description shown in the inspector.
    pub domain: String,
    pub status: NodeStatus,
    /// Current task description (if any).
    pub current_task: Option<String>,
    /// Progress 0.0–1.0 of current task.
    pub progress: f64,
    /// CPU utilisation 0.0–1.0.
    pub cpu_load: f64,
    /// RAM usage in MB.
    pub ram_mb: f64,
    /// Credits consumed so far.
    pub credits_used: f64,
    /// Total tasks completed by this node.
    pub tasks_completed: u64,
    /// Total tasks failed.
    pub tasks_failed: u64,
    /// Whether running on a remote device.
    pub is_remote: bool,
    pub device_label: Option<String>,
    pub spawned_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
}

impl HierarchyNode {
    pub fn new(
        swarm_id: Uuid,
        parent_id: Option<Uuid>,
        role: SwarmRole,
        display_name: impl Into<String>,
        domain: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            swarm_id,
            parent_id,
            role,
            display_name: display_name.into(),
            domain: domain.into(),
            status: NodeStatus::Initialising,
            current_task: None,
            progress: 0.0,
            cpu_load: 0.0,
            ram_mb: 0.0,
            credits_used: 0.0,
            tasks_completed: 0,
            tasks_failed: 0,
            is_remote: false,
            device_label: None,
            spawned_at: now,
            last_heartbeat: now,
        }
    }

    /// Success rate 0.0–1.0.
    pub fn success_rate(&self) -> f64 {
        let total = self.tasks_completed + self.tasks_failed;
        if total == 0 { 1.0 } else { self.tasks_completed as f64 / total as f64 }
    }
}

/// Thread-safe agent hierarchy tree for a single swarm.
#[derive(Clone)]
pub struct AgentHierarchy {
    pub swarm_id: Uuid,
    nodes: Arc<RwLock<HashMap<Uuid, HierarchyNode>>>,
}

impl AgentHierarchy {
    pub fn new(swarm_id: Uuid) -> Self {
        Self {
            swarm_id,
            nodes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Insert a new node (returns its id).
    pub async fn insert(&self, node: HierarchyNode) -> Uuid {
        let id = node.id;
        self.nodes.write().await.insert(id, node);
        id
    }

    /// Update status for a node.
    pub async fn set_status(&self, id: Uuid, status: NodeStatus) {
        if let Some(n) = self.nodes.write().await.get_mut(&id) {
            n.status = status;
            n.last_heartbeat = Utc::now();
        }
    }

    /// Update heartbeat + metrics for a node.
    pub async fn heartbeat(&self, id: Uuid, cpu_load: f64, ram_mb: f64, progress: f64) {
        if let Some(n) = self.nodes.write().await.get_mut(&id) {
            n.cpu_load = cpu_load;
            n.ram_mb = ram_mb;
            n.progress = progress;
            n.last_heartbeat = Utc::now();
        }
    }

    /// Mark a task complete on a node.
    pub async fn record_task_result(&self, id: Uuid, success: bool) {
        if let Some(n) = self.nodes.write().await.get_mut(&id) {
            if success {
                n.tasks_completed += 1;
            } else {
                n.tasks_failed += 1;
            }
            n.progress = 0.0;
            n.current_task = None;
            n.status = NodeStatus::Idle;
        }
    }

    /// Assign a task description to a node.
    pub async fn assign_task(&self, id: Uuid, task_description: impl Into<String>) {
        if let Some(n) = self.nodes.write().await.get_mut(&id) {
            n.current_task = Some(task_description.into());
            n.status = NodeStatus::Working;
            n.progress = 0.0;
        }
    }

    /// Remove a node (agent stopped).
    pub async fn remove(&self, id: Uuid) {
        self.nodes.write().await.remove(&id);
    }

    /// Direct children of a node.
    pub async fn children_of(&self, parent_id: Uuid) -> Vec<HierarchyNode> {
        self.nodes
            .read()
            .await
            .values()
            .filter(|n| n.parent_id == Some(parent_id))
            .cloned()
            .collect()
    }

    /// Root node (ProjectManager with no parent).
    pub async fn root(&self) -> Option<HierarchyNode> {
        self.nodes
            .read()
            .await
            .values()
            .find(|n| n.role == SwarmRole::ProjectManager && n.parent_id.is_none())
            .cloned()
    }

    /// Flat snapshot of all nodes (for dashboard).
    pub async fn snapshot(&self) -> Vec<HierarchyNode> {
        self.nodes.read().await.values().cloned().collect()
    }

    /// Total node count.
    pub async fn count(&self) -> usize {
        self.nodes.read().await.len()
    }

    /// Aggregate stats for the Commander dashboard.
    pub async fn aggregate_stats(&self) -> HierarchyStats {
        let nodes = self.nodes.read().await;
        let total = nodes.len();
        let working = nodes.values().filter(|n| n.status == NodeStatus::Working).count();
        let idle = nodes.values().filter(|n| n.status == NodeStatus::Idle).count();
        let error = nodes.values().filter(|n| matches!(n.status, NodeStatus::Error { .. })).count();
        let total_credits: f64 = nodes.values().map(|n| n.credits_used).sum();
        let avg_load = if total > 0 {
            nodes.values().map(|n| n.cpu_load).sum::<f64>() / total as f64
        } else {
            0.0
        };
        HierarchyStats { total, working, idle, error, total_credits, avg_load }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchyStats {
    pub total: usize,
    pub working: usize,
    pub idle: usize,
    pub error: usize,
    pub total_credits: f64,
    pub avg_load: f64,
}
