//! BonsAI Multi-Agent Coordinator
//!
//! Splits a task across N async workers, each calling a local model endpoint.
//! Workers pull items from a shared work-stealing channel; results are collected
//! and returned to the caller as a `CoordinatorResult`.
//!
//! Usage from a Tauri command:
//!   ```rust
//!   let result = Coordinator::new(cfg)
//!       .run(CoordinatorTask { description: "...", items: vec!["a", "b"] })
//!       .await?;
//!   ```

use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{info, warn};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    /// Model server URL (e.g. "http://127.0.0.1:8082")
    pub model_url: String,
    /// Max parallel workers
    pub max_workers: usize,
    /// Per-worker request timeout in seconds
    pub timeout_secs: u64,
    /// Max tokens per worker response
    pub max_tokens: u32,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            model_url: "http://127.0.0.1:8082".into(),
            max_workers: 4,
            timeout_secs: 60,
            max_tokens: 512,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorTask {
    /// High-level description of what workers should do with each item
    pub description: String,
    /// Items to distribute (file paths, text chunks, questions, etc.)
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResult {
    pub item: String,
    pub output: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorResult {
    pub task_id: String,
    pub total_items: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub results: Vec<WorkerResult>,
    pub elapsed_ms: u64,
}

// ── Coordinator ───────────────────────────────────────────────────────────────

pub struct Coordinator {
    cfg: CoordinatorConfig,
}

impl Coordinator {
    pub fn new(cfg: CoordinatorConfig) -> Self {
        Self { cfg }
    }

    pub fn with_defaults() -> Self {
        Self::new(CoordinatorConfig::default())
    }

    /// Run the task: distribute items across workers, collect results.
    pub async fn run(&self, task: CoordinatorTask) -> CoordinatorResult {
        let task_id = uuid::Uuid::new_v4().to_string();
        let start = std::time::Instant::now();

        let num_workers = self.cfg.max_workers.min(task.items.len()).max(1);
        info!(
            "[coordinator] task={task_id} items={} workers={num_workers}",
            task.items.len()
        );

        // Shared work queue
        let queue: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(task.items.clone()));
        let description = Arc::new(task.description.clone());
        let cfg = Arc::new(self.cfg.clone());

        // Spawn workers
        let mut handles = Vec::with_capacity(num_workers);
        for worker_id in 0..num_workers {
            let queue = Arc::clone(&queue);
            let description = Arc::clone(&description);
            let cfg = Arc::clone(&cfg);

            handles.push(tokio::spawn(async move {
                worker_loop(worker_id, queue, description, cfg).await
            }));
        }

        let worker_results: Vec<Vec<WorkerResult>> = join_all(handles)
            .await
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();

        let results: Vec<WorkerResult> = worker_results.into_iter().flatten().collect();
        let succeeded = results.iter().filter(|r| r.error.is_none()).count();
        let failed = results.len() - succeeded;

        info!(
            "[coordinator] task={task_id} done: {succeeded} ok, {failed} failed, {}ms",
            start.elapsed().as_millis()
        );

        CoordinatorResult {
            task_id,
            total_items: task.items.len(),
            succeeded,
            failed,
            results,
            elapsed_ms: start.elapsed().as_millis() as u64,
        }
    }
}

// ── Worker ────────────────────────────────────────────────────────────────────

async fn worker_loop(
    worker_id: usize,
    queue: Arc<Mutex<Vec<String>>>,
    description: Arc<String>,
    cfg: Arc<CoordinatorConfig>,
) -> Vec<WorkerResult> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(cfg.timeout_secs))
        .build()
        .unwrap_or_default();

    let mut results = Vec::new();

    loop {
        let item = {
            let mut q = queue.lock().await;
            if q.is_empty() {
                break;
            }
            q.remove(0)
        };

        let result = call_model(&client, &cfg.model_url, &description, &item, cfg.max_tokens).await;
        match result {
            Ok(output) => {
                results.push(WorkerResult {
                    item,
                    output,
                    error: None,
                });
            }
            Err(e) => {
                warn!("[coordinator] worker={worker_id} item={item:?} err={e}");
                results.push(WorkerResult {
                    item,
                    output: String::new(),
                    error: Some(e),
                });
            }
        }
    }

    results
}

async fn call_model(
    client: &reqwest::Client,
    model_url: &str,
    description: &str,
    item: &str,
    max_tokens: u32,
) -> Result<String, String> {
    let prompt = format!("{description}\n\nItem:\n{item}");
    let payload = serde_json::json!({
        "messages": [
            { "role": "user", "content": prompt }
        ],
        "max_tokens": max_tokens,
        "temperature": 0.1,
    });

    let resp = client
        .post(format!("{model_url}/v1/chat/completions"))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("model returned HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("parse failed: {e}"))?;

    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    Ok(content)
}

// ── Convenience builder ───────────────────────────────────────────────────────

/// Analyse a list of files for issues using the DreamAgent (port 8082) or any model.
pub async fn analyse_files(
    file_paths: Vec<String>,
    task_prompt: &str,
    model_url: Option<&str>,
) -> CoordinatorResult {
    let cfg = CoordinatorConfig {
        model_url: model_url.unwrap_or("http://127.0.0.1:8082").into(),
        max_workers: num_cpus(),
        ..Default::default()
    };
    Coordinator::new(cfg)
        .run(CoordinatorTask {
            description: task_prompt.to_string(),
            items: file_paths,
        })
        .await
}

fn num_cpus() -> usize {
    // Use available parallelism; cap at 8 to avoid overloading a small model server
    std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(4)
}
