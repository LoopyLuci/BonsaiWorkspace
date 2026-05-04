use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::model_orchestrator::{InferRequest, InferStats, InferenceOverrides, ModelOrchestrator};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    UserChat,
    BotMessage,
    SwarmWorker,
    BackgroundTask,
    SystemTask,
}

impl TaskType {
    fn priority(self) -> u8 {
        match self {
            Self::UserChat => 1,
            Self::BotMessage => 2,
            Self::SwarmWorker => 3,
            Self::BackgroundTask => 4,
            Self::SystemTask => 5,
        }
    }

    fn is_critical(self) -> bool {
        matches!(self, Self::UserChat | Self::BotMessage)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskSource {
    Buddy,
    Bot,
    Workspace,
    Scheduler,
}

impl TaskSource {
    fn as_orchestrator_source(self) -> &'static str {
        match self {
            Self::Buddy => "assistant",
            Self::Bot => "bot",
            Self::Workspace => "workspace",
            Self::Scheduler => "workspace",
        }
    }

    const ORDER: [TaskSource; 4] = [
        TaskSource::Buddy,
        TaskSource::Bot,
        TaskSource::Workspace,
        TaskSource::Scheduler,
    ];
}

#[derive(Debug, Clone)]
pub struct QueueConfig {
    pub max_parallel_inference: usize,
    pub min_free_ram_mb: u64,
    pub max_cpu_pct: f32,
    pub starvation_boost_secs: u64,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_parallel_inference: 1,
            min_free_ram_mb: 2048,
            max_cpu_pct: 90.0,
            starvation_boost_secs: 30,
        }
    }
}

#[derive(Debug)]
pub struct InferenceTask {
    pub task_type: TaskType,
    pub source: TaskSource,
    pub model_id: Option<String>,
    pub messages: Vec<serde_json::Value>,
    pub max_tokens: u32,
    pub overrides: Option<InferenceOverrides>,
    pub stream_tx: Option<mpsc::UnboundedSender<String>>,
    pub cancel_flag: Option<Arc<std::sync::atomic::AtomicBool>>,
    pub estimated_tokens: u32,
    pub estimated_ram_mb: u64,
}

struct QueuedTask {
    id: String,
    task_type: TaskType,
    source: TaskSource,
    model_id: Option<String>,
    messages: Vec<serde_json::Value>,
    max_tokens: u32,
    overrides: Option<InferenceOverrides>,
    stream_tx: Option<mpsc::UnboundedSender<String>>,
    cancel_flag: Option<Arc<std::sync::atomic::AtomicBool>>,
    estimated_tokens: u32,
    estimated_ram_mb: u64,
    created_at: Instant,
    result_tx: oneshot::Sender<Result<(String, InferStats), String>>,
}

#[derive(Clone, Serialize)]
pub struct ActiveTaskInfo {
    pub id: String,
    pub task_type: TaskType,
    pub source: TaskSource,
    pub running_for_ms: u64,
}

#[derive(Clone, Serialize, Default)]
pub struct SourceQueueStats {
    pub pending: usize,
    pub active: usize,
    pub served_last_60s: usize,
    pub avg_latency_ms_last_60s: u64,
    pub starved: bool,
}

#[derive(Clone, Serialize, Default)]
pub struct TaskQueueStatus {
    pub pending_total: usize,
    pub active_total: usize,
    pub max_parallel_inference: usize,
    pub free_ram_mb: u64,
    pub cpu_pct: f32,
    pub sources: HashMap<String, SourceQueueStats>,
    pub active_tasks: Vec<ActiveTaskInfo>,
}

struct CompletedSample {
    source: TaskSource,
    finished_at: Instant,
    latency_ms: u64,
}

struct ActiveTask {
    id: String,
    task_type: TaskType,
    source: TaskSource,
    started_at: Instant,
}

struct QueueStats {
    rr_cursor: usize,
    served_total: HashMap<TaskSource, u64>,
    last_served: HashMap<TaskSource, Instant>,
    recent: VecDeque<CompletedSample>,
}

impl QueueStats {
    fn new() -> Self {
        Self {
            rr_cursor: 0,
            served_total: HashMap::new(),
            last_served: HashMap::new(),
            recent: VecDeque::new(),
        }
    }

    fn prune_recent(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(60);
        while self
            .recent
            .front()
            .map(|r| r.finished_at < cutoff)
            .unwrap_or(false)
        {
            self.recent.pop_front();
        }
    }
}

enum GateDecision {
    Accept,
    Defer,
    Reject(String),
}

#[derive(Clone)]
pub struct TaskQueue {
    orchestrator: Arc<ModelOrchestrator>,
    pending: Arc<Mutex<VecDeque<QueuedTask>>>,
    active: Arc<Mutex<HashMap<String, ActiveTask>>>,
    stats: Arc<Mutex<QueueStats>>,
    config: QueueConfig,
}

impl TaskQueue {
    pub fn new(orchestrator: Arc<ModelOrchestrator>, config: QueueConfig) -> Self {
        let queue = Self {
            orchestrator,
            pending: Arc::new(Mutex::new(VecDeque::new())),
            active: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(QueueStats::new())),
            config,
        };
        queue.start_dispatch_loop();
        queue
    }

    fn start_dispatch_loop(&self) {
        let queue = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                queue.dispatch_once().await;
                tokio::time::sleep(Duration::from_millis(75)).await;
            }
        });
    }

    pub async fn submit(&self, task: InferenceTask) -> Result<(String, InferStats), String> {
        let id = uuid::Uuid::new_v4().to_string();
        let (result_tx, result_rx) = oneshot::channel();

        let queued = QueuedTask {
            id,
            task_type: task.task_type,
            source: task.source,
            model_id: task.model_id,
            messages: task.messages,
            max_tokens: task.max_tokens,
            overrides: task.overrides,
            stream_tx: task.stream_tx,
            cancel_flag: task.cancel_flag,
            estimated_tokens: task.estimated_tokens,
            estimated_ram_mb: task.estimated_ram_mb,
            created_at: Instant::now(),
            result_tx,
        };

        self.pending.lock().await.push_back(queued);

        result_rx
            .await
            .map_err(|_| "task queue request cancelled".to_string())?
    }

    async fn dispatch_once(&self) {
        let active_len = self.active.lock().await.len();
        if active_len >= self.config.max_parallel_inference {
            return;
        }

        let Some(task) = self.next_task().await else {
            return;
        };

        match self.can_accept(task.task_type, task.estimated_ram_mb).await {
            GateDecision::Accept => {
                self.dispatch_task(task).await;
            }
            GateDecision::Defer => {
                self.pending.lock().await.push_back(task);
            }
            GateDecision::Reject(err) => {
                let _ = task.result_tx.send(Err(err));
            }
        }
    }

    async fn next_task(&self) -> Option<QueuedTask> {
        let mut pending = self.pending.lock().await;
        if pending.is_empty() {
            return None;
        }

        let top_priority = pending
            .iter()
            .map(|t| t.task_type.priority())
            .min()
            .unwrap_or(5);

        let starvation_window = Duration::from_secs(self.config.starvation_boost_secs);
        let now = Instant::now();

        let mut stats = self.stats.lock().await;
        let mut starved_source: Option<TaskSource> = None;
        let mut oldest_gap = Duration::ZERO;

        for src in TaskSource::ORDER {
            let has_pending = pending.iter().any(|t| t.source == src);
            if !has_pending {
                continue;
            }
            let gap = match stats.last_served.get(&src) {
                Some(ts) => now.saturating_duration_since(*ts),
                None => starvation_window + Duration::from_secs(1),
            };
            if gap >= starvation_window && gap >= oldest_gap {
                oldest_gap = gap;
                starved_source = Some(src);
            }
        }

        if let Some(src) = starved_source {
            if let Some(idx) = pending.iter().position(|t| t.source == src) {
                return pending.remove(idx);
            }
        }

        for offset in 0..TaskSource::ORDER.len() {
            let idx = (stats.rr_cursor + offset) % TaskSource::ORDER.len();
            let src = TaskSource::ORDER[idx];
            if let Some(task_idx) = pending
                .iter()
                .position(|t| t.task_type.priority() == top_priority && t.source == src)
            {
                stats.rr_cursor = (idx + 1) % TaskSource::ORDER.len();
                return pending.remove(task_idx);
            }
        }

        if let Some(idx) = pending.iter().position(|t| t.task_type.priority() == top_priority) {
            return pending.remove(idx);
        }

        pending.pop_front()
    }

    async fn can_accept(&self, task_type: TaskType, estimated_ram_mb: u64) -> GateDecision {
        let (free_ram_mb, cpu_pct) = system_snapshot();

        if free_ram_mb < self.config.min_free_ram_mb || cpu_pct > self.config.max_cpu_pct {
            if !task_type.is_critical() {
                return GateDecision::Defer;
            }
        }

        if estimated_ram_mb > 0 && free_ram_mb < estimated_ram_mb {
            // If no slot is currently active and memory is insufficient to load the model,
            // fail fast with a user-facing message instead of queueing forever.
            if self.orchestrator.active_slot_url().await.is_none() {
                return GateDecision::Reject(format!(
                    "insufficient memory to load requested model: need ~{} MB free, have {} MB",
                    estimated_ram_mb, free_ram_mb
                ));
            }
        }

        GateDecision::Accept
    }

    async fn dispatch_task(&self, task: QueuedTask) {
        let task_id = task.id.clone();
        {
            let mut active = self.active.lock().await;
            active.insert(
                task_id.clone(),
                ActiveTask {
                    id: task.id.clone(),
                    task_type: task.task_type,
                    source: task.source,
                    started_at: Instant::now(),
                },
            );
        }

        let queue = self.clone();
        tauri::async_runtime::spawn(async move {
            let (resp_tx, resp_rx) = oneshot::channel();

            let infer_res = queue.orchestrator.infer(InferRequest {
                model_id: task.model_id,
                messages: task.messages,
                max_tokens: task.max_tokens,
                overrides: task.overrides,
                stream_tx: task.stream_tx,
                cancel_flag: task.cancel_flag,
                resp_tx,
                source: task.source.as_orchestrator_source(),
            });

            let result = match infer_res {
                Ok(()) => match resp_rx.await {
                    Ok(r) => r,
                    Err(_) => Err("queued inference request cancelled".to_string()),
                },
                Err(e) => Err(e),
            };

            let latency_ms = task.created_at.elapsed().as_millis() as u64;
            queue.finish_task(&task_id, task.source, latency_ms).await;
            let _ = task.result_tx.send(result);
        });
    }

    async fn finish_task(&self, task_id: &str, source: TaskSource, latency_ms: u64) {
        self.active.lock().await.remove(task_id);

        let mut stats = self.stats.lock().await;
        *stats.served_total.entry(source).or_insert(0) += 1;
        stats.last_served.insert(source, Instant::now());
        stats.recent.push_back(CompletedSample {
            source,
            finished_at: Instant::now(),
            latency_ms,
        });
        stats.prune_recent();
    }

    pub async fn status(&self) -> TaskQueueStatus {
        let (free_ram_mb, cpu_pct) = system_snapshot();
        let pending = self.pending.lock().await;
        let active = self.active.lock().await;
        let mut stats = self.stats.lock().await;
        stats.prune_recent();

        let now = Instant::now();
        let starvation_window = Duration::from_secs(self.config.starvation_boost_secs);

        let mut sources: HashMap<String, SourceQueueStats> = HashMap::new();
        for src in TaskSource::ORDER {
            let key = format!("{:?}", src).to_lowercase();
            let pending_count = pending.iter().filter(|t| t.source == src).count();
            let active_count = active.values().filter(|t| t.source == src).count();
            let recent: Vec<&CompletedSample> = stats.recent.iter().filter(|r| r.source == src).collect();
            let served_last_60s = recent.len();
            let avg_latency_ms_last_60s = if recent.is_empty() {
                0
            } else {
                let total: u64 = recent.iter().map(|r| r.latency_ms).sum();
                total / (recent.len() as u64)
            };
            let starved = if pending_count == 0 {
                false
            } else {
                match stats.last_served.get(&src) {
                    Some(ts) => now.saturating_duration_since(*ts) >= starvation_window,
                    None => true,
                }
            };

            sources.insert(
                key,
                SourceQueueStats {
                    pending: pending_count,
                    active: active_count,
                    served_last_60s,
                    avg_latency_ms_last_60s,
                    starved,
                },
            );
        }

        let active_tasks = active
            .values()
            .map(|t| ActiveTaskInfo {
                id: t.id.clone(),
                task_type: t.task_type,
                source: t.source,
                running_for_ms: t.started_at.elapsed().as_millis() as u64,
            })
            .collect::<Vec<_>>();

        TaskQueueStatus {
            pending_total: pending.len(),
            active_total: active.len(),
            max_parallel_inference: self.config.max_parallel_inference,
            free_ram_mb,
            cpu_pct,
            sources,
            active_tasks,
        }
    }
}

fn system_snapshot() -> (u64, f32) {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.refresh_cpu_usage();

    let free_ram_mb = sys.available_memory() / (1024 * 1024);
    let cpu_pct = sys
        .cpus()
        .iter()
        .map(|c| c.cpu_usage())
        .fold(0.0f32, f32::max);

    (free_ram_mb, cpu_pct)
}
