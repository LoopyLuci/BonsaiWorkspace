//! Task DAG — directed acyclic graph of subtasks with dependency tracking.

use std::collections::{HashMap, HashSet, VecDeque};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::role::Capability;

/// Status of a single task node in the DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Queued,
    Running { assigned_to: Uuid },
    Completed,
    Failed { reason: String },
    Cancelled,
}

impl TaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Completed | TaskStatus::Failed { .. } | TaskStatus::Cancelled)
    }
}

/// A single unit of work within a swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    pub id: Uuid,
    pub swarm_id: Uuid,
    pub description: String,
    /// Human-readable context injected into the agent's prompt.
    pub context: String,
    pub status: TaskStatus,
    /// IDs of tasks that must complete before this one can start.
    pub depends_on: Vec<Uuid>,
    /// Required capabilities for assignment.
    pub required_capabilities: Vec<Capability>,
    /// Estimated time in minutes.
    pub estimated_minutes: f64,
    /// Actual result after completion (serialised JSON or text).
    pub result: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl TaskNode {
    pub fn new(
        swarm_id: Uuid,
        description: impl Into<String>,
        depends_on: Vec<Uuid>,
        required_capabilities: Vec<Capability>,
        estimated_minutes: f64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            swarm_id,
            description: description.into(),
            context: String::new(),
            status: TaskStatus::Pending,
            depends_on,
            required_capabilities,
            estimated_minutes,
            result: None,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }
}

/// Validation errors returned by `TaskDag::validate`.
#[derive(Debug, thiserror::Error)]
pub enum DagError {
    #[error("cycle detected involving task {0}")]
    CycleDetected(Uuid),
    #[error("task {0} references unknown dependency {1}")]
    UnknownDependency(Uuid, Uuid),
}

/// A directed acyclic graph of tasks for a single swarm.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TaskDag {
    pub nodes: HashMap<Uuid, TaskNode>,
}

impl TaskDag {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a task and return its id.
    pub fn insert(&mut self, node: TaskNode) -> Uuid {
        let id = node.id;
        self.nodes.insert(id, node);
        id
    }

    /// All tasks whose dependencies are fully satisfied.
    pub fn ready_tasks(&self) -> Vec<&TaskNode> {
        self.nodes
            .values()
            .filter(|n| {
                n.status == TaskStatus::Pending
                    && n.depends_on.iter().all(|dep_id| {
                        self.nodes
                            .get(dep_id)
                            .map(|d| d.status == TaskStatus::Completed)
                            .unwrap_or(false)
                    })
            })
            .collect()
    }

    /// Topological sort (Kahn's algorithm). Returns `Err` if a cycle exists.
    pub fn validate(&self) -> Result<Vec<Uuid>, DagError> {
        for node in self.nodes.values() {
            for dep in &node.depends_on {
                if !self.nodes.contains_key(dep) {
                    return Err(DagError::UnknownDependency(node.id, *dep));
                }
            }
        }

        let mut in_degree: HashMap<Uuid, usize> = self.nodes.keys().map(|k| (*k, 0)).collect();
        for node in self.nodes.values() {
            for dep in &node.depends_on {
                *in_degree.entry(*dep).or_insert(0) += 0; // ensure dep exists
                // node depends on dep, so dep->node edge; node's in_degree increases
            }
        }
        // recalculate: in_degree[n] = number of tasks that n depends on (incoming edges)
        let mut in_deg: HashMap<Uuid, usize> = self.nodes.keys().map(|k| (*k, 0)).collect();
        for node in self.nodes.values() {
            for _dep in &node.depends_on {
                *in_deg.entry(node.id).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<Uuid> = in_deg
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut visited: HashSet<Uuid> = HashSet::new();
        while let Some(id) = queue.pop_front() {
            visited.insert(id);
            // find nodes that depend on this one
            for node in self.nodes.values() {
                if node.depends_on.contains(&id) {
                    let deg = in_deg.entry(node.id).or_insert(1);
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 && !visited.contains(&node.id) {
                        queue.push_back(node.id);
                    }
                }
            }
        }

        if visited.len() != self.nodes.len() {
            let cycle_node = self
                .nodes
                .keys()
                .find(|id| !visited.contains(id))
                .copied()
                .unwrap();
            return Err(DagError::CycleDetected(cycle_node));
        }

        Ok(visited.into_iter().collect())
    }

    /// Overall completion ratio 0.0–1.0.
    pub fn progress(&self) -> f64 {
        if self.nodes.is_empty() {
            return 1.0;
        }
        let done = self.nodes.values().filter(|n| n.status == TaskStatus::Completed).count();
        done as f64 / self.nodes.len() as f64
    }

    /// Estimated remaining minutes (sum of incomplete tasks).
    pub fn eta_minutes(&self) -> f64 {
        self.nodes
            .values()
            .filter(|n| !n.status.is_terminal())
            .map(|n| n.estimated_minutes)
            .sum()
    }
}
