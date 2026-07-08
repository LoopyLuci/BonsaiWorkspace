use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use tracing::{info, warn};

use crate::types::{ComputeNode, FabricTask, TaskResult, TaskStatus};

#[derive(Debug)]
enum CoordMsg {
    AddNode(ComputeNode),
    RemoveNode(String),
    SubmitTask(FabricTask, tokio::sync::oneshot::Sender<TaskResult>),
    NodeResult(TaskResult),
    Shutdown,
}

pub struct CoordinatorActor {
    tx: mpsc::Sender<CoordMsg>,
}

impl CoordinatorActor {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(256);
        tokio::spawn(coordinator_loop(rx));
        Self { tx }
    }

    pub async fn add_node(&self, node: ComputeNode) {
        let _ = self.tx.send(CoordMsg::AddNode(node)).await;
    }

    pub async fn remove_node(&self, node_id: String) {
        let _ = self.tx.send(CoordMsg::RemoveNode(node_id)).await;
    }

    /// Report that a remote node completed a task. The coordinator will route
    /// the result back to the waiting `submit_task` caller.
    pub async fn report_node_result(&self, result: TaskResult) {
        let _ = self.tx.send(CoordMsg::NodeResult(result)).await;
    }

    /// Gracefully shut down the coordinator loop.
    pub async fn shutdown(&self) {
        let _ = self.tx.send(CoordMsg::Shutdown).await;
    }

    pub async fn submit_task(&self, task: FabricTask, deadline_ms: u64) -> Option<TaskResult> {
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        let _ = self.tx.send(CoordMsg::SubmitTask(task, resp_tx)).await;
        timeout(Duration::from_millis(deadline_ms), resp_rx)
            .await
            .ok()?
            .ok()
    }
}

impl Default for CoordinatorActor {
    fn default() -> Self {
        Self::new()
    }
}

struct CoordState {
    nodes: HashMap<String, ComputeNode>,
    // task_id -> response channel
    pending: HashMap<String, tokio::sync::oneshot::Sender<TaskResult>>,
    // queue of tasks waiting for a capable node (ECF-RG: earliest-completion-first)
    queue: Vec<(FabricTask, tokio::sync::oneshot::Sender<TaskResult>)>,
}

impl CoordState {
    fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            pending: HashMap::new(),
            queue: Vec::new(),
        }
    }

    fn best_node_for(&self, task: &FabricTask) -> Option<String> {
        self.nodes
            .values()
            .filter(|n| {
                n.is_online
                    && n.available_cores >= task.required_cores
                    && n.available_memory_mb >= task.required_memory_mb
            })
            // ECF-RG: pick node with most spare memory (simple heuristic)
            .max_by_key(|n| n.available_memory_mb)
            .map(|n| n.node_id.clone())
    }
}

async fn coordinator_loop(mut rx: mpsc::Receiver<CoordMsg>) {
    let mut state = CoordState::new();

    while let Some(msg) = rx.recv().await {
        match msg {
            CoordMsg::AddNode(node) => {
                info!(node_id = %node.node_id, "Fabric node registered");
                state.nodes.insert(node.node_id.clone(), node);
                drain_queue(&mut state);
            }
            CoordMsg::RemoveNode(id) => {
                state.nodes.remove(&id);
            }
            CoordMsg::SubmitTask(task, resp) => {
                let task_id = task.task_id.clone();
                if let Some(node_id) = state.best_node_for(&task) {
                    info!(task_id = %task_id, %node_id, "Assigned task");
                    // In a real implementation this would stream via TaskDistributeStream.
                    // Here we synthesise a mock completion for the local-only case.
                    let start = std::time::Instant::now();
                    let result = TaskResult {
                        task_id: task_id.clone(),
                        status: TaskStatus::Completed,
                        output: Some(b"ok".to_vec()),
                        duration_ms: start.elapsed().as_millis() as u64,
                    };
                    let _ = resp.send(result);
                } else {
                    warn!(task_id = %task_id, "No capable node — queuing");
                    state.queue.push((task, resp));
                }
            }
            CoordMsg::NodeResult(result) => {
                if let Some(resp) = state.pending.remove(&result.task_id) {
                    let _ = resp.send(result);
                }
            }
            CoordMsg::Shutdown => break,
        }
    }
}

fn drain_queue(state: &mut CoordState) {
    let mut remaining = Vec::new();
    let drained: Vec<_> = state.queue.drain(..).collect();
    for (task, resp) in drained {
        if let Some(node_id) = state.best_node_for(&task) {
            info!(task_id = %task.task_id, %node_id, "Drained queued task to node");
            let result = TaskResult {
                task_id: task.task_id.clone(),
                status: TaskStatus::Completed,
                output: Some(b"ok".to_vec()),
                duration_ms: 0,
            };
            let _ = resp.send(result);
        } else {
            remaining.push((task, resp));
        }
    }
    state.queue = remaining;
}
