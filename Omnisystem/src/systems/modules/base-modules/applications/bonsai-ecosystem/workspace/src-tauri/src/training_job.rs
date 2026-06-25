//! Training Job Manager — spawns, monitors, and streams training processes.
//!
//! Each training run is a child process (weekly_train.ps1 or a phase-specific
//! Python command). Stdout/stderr are captured line-by-line and forwarded to the
//! frontend via Tauri events. Job state is persisted to SQLite.
//!
//! Events emitted:
//!   `training-log`            { job_id, line, level }
//!   `training-phase-change`   { job_id, phase }
//!   `training-progress`       { job_id, phase, progress, message }
//!   `training-completed`      { job_id, adapter_path }
//!   `training-error`          { job_id, error }

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Running,
    Completed,
    Stopped,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterInfo {
    pub name: String,
    pub path: String,
    pub size_mb: f64,
    pub created_at: String,
    pub is_deployed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingJobStatus {
    pub job_id: String,
    pub phases: Vec<String>,
    pub status: JobStatus,
    pub current_phase: Option<String>,
    pub progress: u8, // 0–100 overall
    pub elapsed_secs: u64,
    pub log_tail: Vec<String>, // last 100 lines
    pub adapter_path: Option<String>,
    pub error: Option<String>,
}

// Internal job handle (not serialised)
struct JobHandle {
    status: JobStatus,
    phases: Vec<String>,
    current_phase: Option<String>,
    progress: u8,
    started: Instant,
    logs: Vec<String>,
    adapter_path: Option<String>,
    error: Option<String>,
    // The child is taken/aborted when stop() is called
    child: Option<Child>,
}

// ── Manager ───────────────────────────────────────────────────────────────────

pub struct TrainingJobManager {
    jobs: Mutex<HashMap<String, JobHandle>>,
    active_id: Mutex<Option<String>>,
    workspace: PathBuf,
}

impl TrainingJobManager {
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            jobs: Mutex::new(HashMap::new()),
            active_id: Mutex::new(None),
            workspace,
        }
    }

    /// Spawn a training run. `phases` = None → full weekly pipeline.
    pub async fn start(
        self: &Arc<Self>,
        app: AppHandle,
        phases: Option<Vec<String>>,
    ) -> Result<String, String> {
        // Only one job at a time
        if let Some(id) = self.active_id.lock().await.as_deref() {
            let jobs = self.jobs.lock().await;
            if let Some(j) = jobs.get(id) {
                if j.status == JobStatus::Running {
                    return Err("A training job is already running".into());
                }
            }
        }

        let selected = phases.clone().unwrap_or_else(|| {
            vec![
                "safety", "survival", "tool_use", "code", "chat", "reason", "final", "convert",
            ]
            .into_iter()
            .map(String::from)
            .collect()
        });

        // Build the PowerShell command
        let script = self.workspace.join("scripts/weekly_train.ps1");
        let mut cmd = Command::new("powershell");
        cmd.arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(&script);

        // Add -Skip* flags for phases NOT in selected
        let all_phases = [
            "Safety", "Survival", "ToolUse", "Code", "Chat", "Reason", "Final", "Convert",
        ];
        let phase_map: HashMap<&str, &str> = [
            ("safety", "Safety"),
            ("survival", "Survival"),
            ("tool_use", "ToolUse"),
            ("code", "Code"),
            ("chat", "Chat"),
            ("reason", "Reason"),
            ("final", "Final"),
            ("convert", "Convert"),
        ]
        .into_iter()
        .collect();

        for p in &all_phases {
            let key = phase_map.iter().find(|(_, v)| **v == *p).map(|(k, _)| *k);
            if let Some(k) = key {
                if !selected.contains(&k.to_string()) {
                    cmd.arg(format!("-Skip{p}"));
                }
            }
        }

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(&self.workspace);

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn training: {e}"))?;

        let stdout = child.stdout.take().ok_or("Could not capture stdout")?;
        let stderr = child.stderr.take().ok_or("Could not capture stderr")?;

        let job_id = uuid::Uuid::new_v4().to_string();

        {
            let mut jobs = self.jobs.lock().await;
            jobs.insert(
                job_id.clone(),
                JobHandle {
                    status: JobStatus::Running,
                    phases: selected.clone(),
                    current_phase: selected.first().cloned(),
                    progress: 0,
                    started: Instant::now(),
                    logs: Vec::new(),
                    adapter_path: None,
                    error: None,
                    child: Some(child),
                },
            );
        }
        *self.active_id.lock().await = Some(job_id.clone());

        info!("[trainer] job {job_id} started — phases: {selected:?}");

        // Stream stdout
        let mgr = Arc::clone(self);
        let app2 = app.clone();
        let id2 = job_id.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                mgr.handle_log_line(&app2, &id2, &line, "stdout").await;
            }
        });

        // Stream stderr
        let mgr = Arc::clone(self);
        let app3 = app.clone();
        let id3 = job_id.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                mgr.handle_log_line(&app3, &id3, &line, "stderr").await;
            }
        });

        // Wait for completion
        let mgr = Arc::clone(self);
        let app4 = app.clone();
        let id4 = job_id.clone();
        tokio::spawn(async move {
            // Poll until child is done (we already took stdout/stderr)
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let mut jobs = mgr.jobs.lock().await;
                if let Some(job) = jobs.get_mut(&id4) {
                    if let Some(child) = job.child.as_mut() {
                        match child.try_wait() {
                            Ok(Some(status)) => {
                                if status.success() {
                                    job.status = JobStatus::Completed;
                                    job.progress = 100;
                                    let adapter = dirs::home_dir()
                                        .unwrap_or_default()
                                        .join(".bonsai/models/bonsai-latest.gguf")
                                        .to_string_lossy()
                                        .to_string();
                                    job.adapter_path = Some(adapter.clone());
                                    let phases_done = job.phases.clone();
                                    // Persist to brain_metadata
                                    let mut meta = crate::brain_metadata::BrainMetadata::load();
                                    for p in &phases_done {
                                        meta.record_phase(p);
                                    }
                                    let _ = app4.emit(
                                        "training-completed",
                                        serde_json::json!({
                                            "job_id": id4,
                                            "adapter_path": adapter,
                                            "phases_done": phases_done,
                                        }),
                                    );
                                    info!("[trainer] job {id4} completed");
                                    let n = phases_done.len();
                                    let _ = app4
                                        .notification()
                                        .builder()
                                        .title("🎉 BonsAI Training Complete!")
                                        .body(format!(
                                            "{} lesson{} finished. Ready to deploy!",
                                            n,
                                            if n == 1 { "" } else { "s" }
                                        ))
                                        .show();
                                } else {
                                    let code = status.code().unwrap_or(-1);
                                    let err = format!("Process exited with code {code}");
                                    job.status = JobStatus::Error;
                                    job.error = Some(err.clone());
                                    let _ = app4.emit(
                                        "training-error",
                                        serde_json::json!({
                                            "job_id": id4,
                                            "error": err,
                                        }),
                                    );
                                    warn!("[trainer] job {id4} error: {code}");
                                }
                                break;
                            }
                            Ok(None) => { /* still running */ }
                            Err(e) => {
                                job.status = JobStatus::Error;
                                job.error = Some(e.to_string());
                                break;
                            }
                        }
                    }
                } else {
                    break;
                }
            }
        });

        Ok(job_id)
    }

    /// Stop the active job.
    pub async fn stop(&self) -> Result<(), String> {
        let id = self.active_id.lock().await.clone().ok_or("No active job")?;
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.get_mut(&id) {
            if let Some(mut child) = job.child.take() {
                let _ = child.kill().await;
            }
            job.status = JobStatus::Stopped;
            info!("[trainer] job {id} stopped");
        }
        Ok(())
    }

    /// Get current status.
    pub async fn status(&self) -> Option<TrainingJobStatus> {
        let id = self.active_id.lock().await.clone()?;
        let jobs = self.jobs.lock().await;
        let job = jobs.get(&id)?;
        Some(TrainingJobStatus {
            job_id: id,
            phases: job.phases.clone(),
            status: job.status.clone(),
            current_phase: job.current_phase.clone(),
            progress: job.progress,
            elapsed_secs: job.started.elapsed().as_secs(),
            log_tail: job.logs.iter().rev().take(100).rev().cloned().collect(),
            adapter_path: job.adapter_path.clone(),
            error: job.error.clone(),
        })
    }

    // Parse a log line for phase/progress markers and update state.
    async fn handle_log_line(&self, app: &AppHandle, job_id: &str, line: &str, _src: &str) {
        // Throttle: 200ms minimum between events
        let level = if line.contains("ERROR") || line.contains("error") {
            "error"
        } else if line.contains("warn") || line.contains("WARN") {
            "warn"
        } else {
            "info"
        };

        let _ = app.emit(
            "training-log",
            serde_json::json!({
                "job_id": job_id,
                "line": line,
                "level": level,
            }),
        );

        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.logs.push(line.to_string());
            if job.logs.len() > 2000 {
                job.logs.drain(0..500);
            }

            // Detect phase changes from weekly_train.ps1 output
            // "[phase] Phase 3: Tool-Use DPO"
            if let Some(rest) = line.strip_prefix("[phase] ") {
                job.current_phase = Some(rest.to_string());
                let _ = app.emit(
                    "training-phase-change",
                    serde_json::json!({
                        "job_id": job_id,
                        "phase": rest,
                    }),
                );
            }

            // Parse progress JSON: {"phase":"safety_dpo","progress":45,"message":"..."}
            if line.trim_start().starts_with('{') {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim()) {
                    if let (Some(p), Some(prog)) = (v["phase"].as_str(), v["progress"].as_u64()) {
                        job.progress = prog.min(99) as u8;
                        let msg = v["message"].as_str().unwrap_or("");
                        let _ = app.emit(
                            "training-progress",
                            serde_json::json!({
                                "job_id": job_id,
                                "phase": p,
                                "progress": prog,
                                "message": msg,
                            }),
                        );
                    }
                }
            }
        }
    }
}

// ── Adapter listing ───────────────────────────────────────────────────────────

pub fn list_adapters() -> Vec<AdapterInfo> {
    let adapters_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".bonsai/adapters");
    let deployed = dirs::home_dir()
        .unwrap_or_default()
        .join(".bonsai/models/bonsai-latest.gguf");

    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&adapters_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let meta = std::fs::metadata(&path);
            let size_mb = dir_size_mb(&path);
            let created_at = meta
                .ok()
                .and_then(|m| m.created().ok())
                .map(|t| {
                    let secs = t
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    format_unix_ts(secs)
                })
                .unwrap_or_else(|| "unknown".into());

            // Check if this adapter is the currently deployed model
            // (heuristic: compare dir name with deployed GGUF parent dir)
            let is_deployed = deployed.parent().map(|p| p == path).unwrap_or(false);

            result.push(AdapterInfo {
                name,
                path: path.to_string_lossy().to_string(),
                size_mb,
                created_at,
                is_deployed,
            });
        }
    }

    // Also list any pre-built GGUFs
    let models_dir = dirs::home_dir().unwrap_or_default().join(".bonsai/models");
    if let Ok(entries) = std::fs::read_dir(&models_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("gguf") {
                continue;
            }
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let meta = std::fs::metadata(&path).ok();
            let size_mb = meta
                .as_ref()
                .map(|m| m.len() as f64 / 1_048_576.0)
                .unwrap_or(0.0);
            let created_at = meta
                .and_then(|m| m.created().ok())
                .map(|t| {
                    format_unix_ts(
                        t.duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    )
                })
                .unwrap_or_else(|| "unknown".into());
            let is_deployed = path == deployed;
            result.push(AdapterInfo {
                name,
                path: path.to_string_lossy().to_string(),
                size_mb,
                created_at,
                is_deployed,
            });
        }
    }

    result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    result
}

fn dir_size_mb(path: &std::path::Path) -> f64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for e in entries.flatten() {
            if let Ok(m) = e.metadata() {
                if m.is_file() {
                    total += m.len();
                }
            }
        }
    }
    total as f64 / 1_048_576.0
}

fn format_unix_ts(secs: u64) -> String {
    // Simple ISO-ish format without chrono dependency issues
    let dt = chrono::DateTime::from_timestamp(secs as i64, 0).unwrap_or_else(chrono::Utc::now);
    dt.format("%Y-%m-%d %H:%M").to_string()
}
