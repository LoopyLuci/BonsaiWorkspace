//! Swarm Orchestrator — central coordinator for a named swarm.
//!
//! Manages the full lifecycle: creation → planning → dispatch → monitoring →
//! result synthesis → cleanup. One `SwarmOrchestrator` instance exists per
//! running swarm; the `SwarmRegistry` holds all active instances.

use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    dag::{TaskDag, TaskNode, TaskStatus},
    hierarchy::{AgentHierarchy, HierarchyNode, HierarchyStats, NodeStatus},
    ledger::{LedgerEventKind, SwarmLedger},
    registry::CapabilityRegistry,
    role::{Capability, SwarmRole},
};

// ── Public types ───────────────────────────────────────────────────────────────

/// Request to create a new swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmSpec {
    pub name: String,
    pub goal: String,
    /// Suggested maximum number of agent workers.
    pub max_workers: u32,
    /// List of tool names the swarm is allowed to use. Empty = all safe tools.
    pub allowed_tools: Vec<String>,
    /// Timeout in seconds for the entire swarm. None = 1 hour.
    pub timeout_secs: Option<u64>,
    /// Optional project workspace path (injected into agent context).
    pub workspace_path: Option<String>,
}

/// High-level status of a swarm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmStatus {
    Initialising,
    Planning,
    Running,
    Paused,
    Completing,
    Completed,
    Failed { reason: String },
    Cancelled,
}

impl SwarmStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            SwarmStatus::Completed | SwarmStatus::Failed { .. } | SwarmStatus::Cancelled
        )
    }
}

/// Snapshot of a swarm's state for the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmSnapshot {
    pub id: Uuid,
    pub name: String,
    pub goal: String,
    pub status: SwarmStatus,
    pub progress: f64,
    pub eta_minutes: f64,
    pub agent_stats: HierarchyStats,
    pub task_total: usize,
    pub task_completed: usize,
    pub task_failed: usize,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result_summary: Option<String>,
    pub ledger_entries: usize,
}

/// Message type for the internal swarm command channel.
#[derive(Debug)]
pub enum SwarmCommand {
    /// Mark a task as completed with a result string.
    TaskCompleted { task_id: Uuid, agent_id: Uuid, result: String },
    /// Mark a task as failed.
    TaskFailed { task_id: Uuid, agent_id: Uuid, reason: String },
    /// Heartbeat from an agent node.
    Heartbeat { node_id: Uuid, cpu_load: f64, ram_mb: f64, progress: f64 },
    /// Pause all activity.
    Pause,
    /// Resume after pause.
    Resume,
    /// Immediately cancel the swarm.
    Cancel,
}

// ── SwarmOrchestrator ──────────────────────────────────────────────────────────

/// Orchestrator for a single named swarm.
pub struct SwarmOrchestrator {
    pub id: Uuid,
    pub spec: SwarmSpec,
    pub status: Arc<RwLock<SwarmStatus>>,
    pub dag: Arc<RwLock<TaskDag>>,
    pub hierarchy: AgentHierarchy,
    pub ledger: SwarmLedger,
    pub registry: CapabilityRegistry,
    pub cmd_tx: mpsc::UnboundedSender<SwarmCommand>,
    pub created_at: DateTime<Utc>,
    pub started_at: Arc<RwLock<Option<DateTime<Utc>>>>,
    pub completed_at: Arc<RwLock<Option<DateTime<Utc>>>>,
    pub result_summary: Arc<RwLock<Option<String>>>,
}

impl SwarmOrchestrator {
    /// Create and start a new swarm. Returns the orchestrator and its background task handle.
    pub fn spawn(
        spec: SwarmSpec,
        registry: CapabilityRegistry,
    ) -> (Arc<Self>, tokio::task::JoinHandle<()>) {
        let id = Uuid::new_v4();
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

        let orch = Arc::new(Self {
            id,
            spec: spec.clone(),
            status: Arc::new(RwLock::new(SwarmStatus::Initialising)),
            dag: Arc::new(RwLock::new(TaskDag::new())),
            hierarchy: AgentHierarchy::new(id),
            ledger: SwarmLedger::new(id),
            registry,
            cmd_tx,
            created_at: Utc::now(),
            started_at: Arc::new(RwLock::new(None)),
            completed_at: Arc::new(RwLock::new(None)),
            result_summary: Arc::new(RwLock::new(None)),
        });

        let orch_clone = orch.clone();
        let handle = tokio::spawn(async move {
            orch_clone.run_loop(cmd_rx).await;
        });

        (orch, handle)
    }

    async fn run_loop(&self, mut cmd_rx: mpsc::UnboundedReceiver<SwarmCommand>) {
        self.ledger.append(LedgerEventKind::SwarmCreated, None).await;
        *self.status.write().await = SwarmStatus::Planning;

        // Seed the plan from the goal (stub: in production calls a planning LLM).
        self.seed_plan().await;
        self.ledger
            .append(
                LedgerEventKind::PlanCreated {
                    task_count: self.dag.read().await.nodes.len(),
                },
                None,
            )
            .await;

        *self.status.write().await = SwarmStatus::Running;
        *self.started_at.write().await = Some(Utc::now());

        // Spawn a ProjectManager node.
        let pm = HierarchyNode::new(
            self.id,
            None,
            SwarmRole::ProjectManager,
            format!("PM – {}", self.spec.name),
            self.spec.goal.clone(),
        );
        let pm_id = self.hierarchy.insert(pm).await;
        self.ledger
            .append(LedgerEventKind::AgentSpawned { role: "ProjectManager".into() }, None)
            .await;
        self.hierarchy.set_status(pm_id, NodeStatus::Working).await;

        info!(swarm_id = %self.id, "[swarm] running");

        // Main event loop.
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                SwarmCommand::TaskCompleted { task_id, agent_id, result } => {
                    self.handle_task_completed(task_id, agent_id, result).await;
                }
                SwarmCommand::TaskFailed { task_id, agent_id, reason } => {
                    self.handle_task_failed(task_id, agent_id, reason).await;
                }
                SwarmCommand::Heartbeat { node_id, cpu_load, ram_mb, progress } => {
                    self.hierarchy.heartbeat(node_id, cpu_load, ram_mb, progress).await;
                }
                SwarmCommand::Pause => {
                    *self.status.write().await = SwarmStatus::Paused;
                    warn!(swarm_id = %self.id, "[swarm] paused");
                }
                SwarmCommand::Resume => {
                    *self.status.write().await = SwarmStatus::Running;
                    info!(swarm_id = %self.id, "[swarm] resumed");
                }
                SwarmCommand::Cancel => {
                    *self.status.write().await = SwarmStatus::Cancelled;
                    self.ledger
                        .append(LedgerEventKind::SwarmFailed, None)
                        .await;
                    info!(swarm_id = %self.id, "[swarm] cancelled");
                    return;
                }
            }

            // Check if all tasks are done.
            if self.dag.read().await.progress() >= 1.0 {
                *self.status.write().await = SwarmStatus::Completed;
                *self.completed_at.write().await = Some(Utc::now());
                *self.result_summary.write().await =
                    Some(format!("Swarm '{}' completed all tasks.", self.spec.name));
                self.ledger.append(LedgerEventKind::SwarmCompleted, None).await;
                info!(swarm_id = %self.id, "[swarm] all tasks completed");
                return;
            }
        }
    }

    async fn handle_task_completed(&self, task_id: Uuid, agent_id: Uuid, result: String) {
        let mut dag = self.dag.write().await;
        if let Some(task) = dag.nodes.get_mut(&task_id) {
            task.status = TaskStatus::Completed;
            task.result = Some(result);
            task.completed_at = Some(Utc::now());
        }
        drop(dag);
        self.ledger
            .append(LedgerEventKind::TaskCompleted { agent_id }, Some(task_id))
            .await;
        self.hierarchy.record_task_result(agent_id, true).await;
    }

    async fn handle_task_failed(&self, task_id: Uuid, agent_id: Uuid, reason: String) {
        let mut dag = self.dag.write().await;
        if let Some(task) = dag.nodes.get_mut(&task_id) {
            task.status = TaskStatus::Failed { reason: reason.clone() };
            task.completed_at = Some(Utc::now());
        }
        drop(dag);
        self.ledger
            .append(
                LedgerEventKind::TaskFailed { agent_id, reason },
                Some(task_id),
            )
            .await;
        self.hierarchy.record_task_result(agent_id, false).await;
    }

    /// Seed the DAG with a stub plan.
    /// In production this calls the local planning LLM with the goal.
    async fn seed_plan(&self) {
        let swarm_id = self.id;
        let mut dag = self.dag.write().await;

        let t1 = TaskNode::new(
            swarm_id,
            format!("Analyse goal: {}", self.spec.goal),
            vec![],
            vec![Capability::knowledge("planning")],
            2.0,
        );
        let t1_id = t1.id;

        let t2 = TaskNode::new(
            swarm_id,
            "Execute primary subtask".to_string(),
            vec![t1_id],
            vec![Capability::tool("execute_code")],
            5.0,
        );
        let t2_id = t2.id;

        let t3 = TaskNode::new(
            swarm_id,
            "Synthesise and report results".to_string(),
            vec![t2_id],
            vec![Capability::knowledge("synthesis")],
            1.0,
        );

        dag.insert(t1);
        dag.insert(t2);
        dag.insert(t3);
    }

    /// Take a snapshot of the swarm's current state.
    pub async fn snapshot(&self) -> SwarmSnapshot {
        let dag = self.dag.read().await;
        let status = self.status.read().await.clone();
        let task_total = dag.nodes.len();
        let task_completed = dag.nodes.values().filter(|n| n.status == TaskStatus::Completed).count();
        let task_failed = dag
            .nodes
            .values()
            .filter(|n| matches!(n.status, TaskStatus::Failed { .. }))
            .count();
        let progress = dag.progress();
        let eta_minutes = dag.eta_minutes();
        drop(dag);

        let agent_stats = self.hierarchy.aggregate_stats().await;

        SwarmSnapshot {
            id: self.id,
            name: self.spec.name.clone(),
            goal: self.spec.goal.clone(),
            status,
            progress,
            eta_minutes,
            agent_stats,
            task_total,
            task_completed,
            task_failed,
            created_at: self.created_at,
            started_at: *self.started_at.read().await,
            completed_at: *self.completed_at.read().await,
            result_summary: self.result_summary.read().await.clone(),
            ledger_entries: self.ledger.len().await,
        }
    }
}

// ── SwarmRegistry ──────────────────────────────────────────────────────────────

/// Global registry of all active swarm orchestrators.
#[derive(Clone)]
pub struct SwarmRegistry {
    swarms: Arc<RwLock<HashMap<Uuid, Arc<SwarmOrchestrator>>>>,
    registry: CapabilityRegistry,
}

impl SwarmRegistry {
    pub fn new() -> Self {
        Self {
            swarms: Arc::new(RwLock::new(HashMap::new())),
            registry: CapabilityRegistry::new(),
        }
    }

    /// Spawn a new swarm from a spec. Returns the swarm id.
    pub async fn create_swarm(&self, spec: SwarmSpec) -> Uuid {
        let (orch, _handle) = SwarmOrchestrator::spawn(spec, self.registry.clone());
        let id = orch.id;
        self.swarms.write().await.insert(id, orch);
        id
    }

    pub async fn get(&self, id: Uuid) -> Option<Arc<SwarmOrchestrator>> {
        self.swarms.read().await.get(&id).cloned()
    }

    pub async fn send_command(&self, id: Uuid, cmd: SwarmCommand) -> bool {
        if let Some(orch) = self.get(id).await {
            orch.cmd_tx.send(cmd).is_ok()
        } else {
            false
        }
    }

    pub async fn list_snapshots(&self) -> Vec<SwarmSnapshot> {
        let ids: Vec<Uuid> = self.swarms.read().await.keys().cloned().collect();
        let mut snaps = Vec::new();
        for id in ids {
            if let Some(orch) = self.get(id).await {
                snaps.push(orch.snapshot().await);
            }
        }
        snaps
    }

    pub async fn remove_completed(&self) {
        let mut map = self.swarms.write().await;
        map.retain(|_, v| {
            // Can't await inside retain; use try_read as best effort
            v.status.try_read().map(|s| !s.is_terminal()).unwrap_or(true)
        });
    }

    /// The global capability registry (shared across all swarms).
    pub fn capability_registry(&self) -> &CapabilityRegistry {
        &self.registry
    }

    /// Spawn a swarm from a `SwarmTemplate`, passing arbitrary JSON parameters
    /// as the swarm goal description. Returns the new swarm's UUID.
    pub async fn spawn_from_template(
        &self,
        template: &crate::templates::SwarmTemplate,
        params: serde_json::Value,
    ) -> Uuid {
        let goal = format!(
            "Template: {}. Params: {}",
            template.name,
            serde_json::to_string(&params).unwrap_or_default()
        );
        let spec = SwarmSpec {
            name: format!("{}-{}", template.name, uuid::Uuid::new_v4()),
            goal,
            max_workers: template.max_agents as u32,
            allowed_tools: template.members.iter()
                .flat_map(|m| m.required_tools.clone())
                .collect(),
            timeout_secs: Some(template.timeout_secs),
            workspace_path: None,
        };
        self.create_swarm(spec).await
    }
}

impl Default for SwarmRegistry {
    fn default() -> Self {
        Self::new()
    }
}
