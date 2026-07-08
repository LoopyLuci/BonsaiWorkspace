//! Survival System — the background monitoring loop.
//!
//! One of the Survival System's tools (see `super` for the others: `kb`,
//! `bug_db`, `bug_hunter`, `sns_bridge`, `crash_ingest`) — same shape as the
//! existing sidecar-health watchdog (`lib.rs`'s `tauri::async_runtime::spawn`
//! timer loop): sleep, wake, do the work, repeat. Scanning is read-only and
//! safe by default (`survival_enabled`, default true); submitting
//! discovered bugs to the self-upgrade agent additionally requires
//! `self_upgrade_enabled` (default false, unchanged from Part C) —
//! detection never implies fixing.
//!
//! Each cycle: round-robins to the next `targets::MonitorTarget`
//! (`bug_hunter::scan_target`), ingests any new crash reports
//! (`crash_ingest`) and any new fuzzing/sandbox findings (`sns_bridge`),
//! upserts everything into the Bug Database, and — if Self-Build is also
//! enabled — submits the highest-signal open bugs through the same
//! self-upgrade pipeline built in Part C.
//!
//! `SurvivalDaemon` is the single Tauri-managed state for all of this — the
//! background loop and the manual `scan_now` command both operate on the
//! same `Arc<SurvivalDaemon>`, so the round-robin position and the crash/
//! SNS high-water marks are shared, not duplicated per call site.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::agent::{AgentContext, AgentMessage};
use crate::agent_host::AgentHost;
use crate::model_orchestrator::ModelOrchestrator;

use super::bug_db::BugDb;
use super::bug_hunter;
use super::crash_ingest::CrashIngestCursor;
use super::sns_bridge::SnsIngestCursor;
use super::targets::TargetScheduler;

const SCAN_INTERVAL: Duration = Duration::from_secs(20 * 60);
/// Never submit more than this many bugs to the self-upgrade agent per
/// cycle — each submission is a real model call plus a sandboxed
/// build+test, not a cheap operation.
const MAX_AUTO_SUBMIT_PER_CYCLE: i64 = 3;

pub struct SurvivalDaemon {
    pub app_handle: AppHandle,
    pub bug_db: Arc<BugDb>,
    pub agent_host: Arc<AgentHost>,
    pub orchestrator: Arc<ModelOrchestrator>,
    pub f3: Arc<failure_finder::F3Orchestrator>,
    pub sns: Arc<sns::SandboxSupervisor>,
    pub repo_root: PathBuf,
    scanning: AtomicBool,
    scheduler: Mutex<TargetScheduler>,
    crash_cursor: Mutex<CrashIngestCursor>,
    sns_cursor: Mutex<SnsIngestCursor>,
}

impl SurvivalDaemon {
    pub fn new(
        app_handle: AppHandle,
        bug_db: Arc<BugDb>,
        agent_host: Arc<AgentHost>,
        orchestrator: Arc<ModelOrchestrator>,
        f3: Arc<failure_finder::F3Orchestrator>,
        sns: Arc<sns::SandboxSupervisor>,
        repo_root: PathBuf,
    ) -> Self {
        Self {
            app_handle,
            bug_db,
            agent_host,
            orchestrator,
            f3,
            sns,
            repo_root,
            scanning: AtomicBool::new(false),
            scheduler: Mutex::new(TargetScheduler::new(super::targets::default_registry())),
            crash_cursor: Mutex::new(CrashIngestCursor::new()),
            sns_cursor: Mutex::new(SnsIngestCursor::new()),
        }
    }
}

pub fn spawn(daemon: Arc<SurvivalDaemon>) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(SCAN_INTERVAL).await;
            run_cycle(&daemon).await;
        }
    });
}

/// Runs one scan+ingest(+maybe-submit) cycle. Shared by the background
/// timer loop and the manual `scan_now` command so "Scan Now" in the UI
/// does exactly what the daemon does, just on demand.
pub async fn run_cycle(daemon: &SurvivalDaemon) -> usize {
    if !crate::features::FeatureFlags::is_enabled("survival") {
        return 0;
    }
    if daemon.scanning.swap(true, Ordering::SeqCst) {
        // A scan is already in flight (manual trigger overlapping the
        // timer, or a prior cycle still running a slow `cargo test`) —
        // skip rather than run two `cargo`/`npm` processes concurrently
        // against the same tree.
        return 0;
    }

    let mut discovered = Vec::new();
    {
        let mut scheduler = daemon.scheduler.lock().await;
        if let Some(target) = scheduler.next() {
            discovered.extend(bug_hunter::scan_target(&daemon.repo_root, target));
        }
    }
    {
        let mut crash_cursor = daemon.crash_cursor.lock().await;
        discovered.extend(crash_cursor.poll_new());
    }
    {
        let mut sns_cursor = daemon.sns_cursor.lock().await;
        discovered.extend(sns_cursor.poll_new(&daemon.f3, &daemon.sns));
    }

    let mut touched = 0usize;
    for bug in discovered {
        if let Ok(record) = daemon.bug_db.upsert(bug).await {
            touched += 1;
            let is_new = record.occurrence_count == 1;
            let _ = daemon.app_handle.emit(if is_new { "bug-discovered" } else { "bug-updated" }, &record);
        }
    }

    if crate::features::FeatureFlags::is_enabled("self_upgrade") {
        if let Ok(candidates) = daemon.bug_db.fixable_candidates(MAX_AUTO_SUBMIT_PER_CYCLE).await {
            for bug in candidates {
                submit_for_fix(daemon, &bug).await;
            }
        }
    }

    daemon.scanning.store(false, Ordering::SeqCst);
    touched
}

async fn submit_for_fix(daemon: &SurvivalDaemon, bug: &super::bug_db::BugRecord) {
    let goal = format!(
        "Fix this bug discovered by the Survival System:\n\nSource: {}\nSeverity: {}\nFile: {}\nOccurrences: {}\n\n{}",
        bug.source,
        bug.severity,
        bug.file_path.as_deref().unwrap_or("(unknown)"),
        bug.occurrence_count,
        bug.message,
    );

    let ctx = AgentContext { model_url: daemon.orchestrator.active_slot_url().await };
    let msg = AgentMessage { content: goal, role: None, metadata: None };

    match daemon.agent_host.handle("self-upgrader", ctx, msg).await {
        Ok(output) => {
            let proposal_id = output
                .actions
                .first()
                .and_then(|a| a.payload.get("id"))
                .and_then(|v| v.as_str());
            if let Some(id) = proposal_id {
                let _ = daemon.bug_db.link_proposal(bug.id, id).await;
            } else {
                // No files were proposed at all — still an attempt, so it
                // still counts against the retry cap.
                let _ = daemon.bug_db.increment_fix_attempts(bug.id).await;
            }
        }
        Err(_) => {
            let _ = daemon.bug_db.increment_fix_attempts(bug.id).await;
        }
    }
    let _ = daemon.app_handle.emit("bug-updated", bug.id);
}
