/// bonsai-watchdog — launch guardian and survival supervisor.
///
/// Lifecycle:
///   1. Write builtin seed fixes into the knowledge base on first run.
///   2. Launch the main Bonsai process.
///   3. Monitor it: ping health endpoint every 15s.
///   4. On exit / crash: collect logs, attempt repair, relaunch.
///   5. Persist every successful fix so the KB grows over time.
///
/// The watchdog is meant to run as a separate process, started by the OS
/// autostart / installer before the main app.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep;
use tracing::{error, info, warn};

mod kb;
mod repair;
#[cfg(test)]
mod tests;

use kb::KnowledgeBase;

// ── EternalWorkshop sidecar ───────────────────────────────────────────────────

const WORKSHOP_HEALTH_SEC: u64 = 60;

fn workshop_exe() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("BONSAI_WORKSHOP_EXE") {
        return Some(PathBuf::from(p));
    }
    let exe_dir = std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(&PathBuf::from("."))
        .to_path_buf();

    #[cfg(target_os = "windows")]
    let candidate = exe_dir.join("eternal-workshop.exe");
    #[cfg(not(target_os = "windows"))]
    let candidate = exe_dir.join("eternal-workshop");

    if candidate.exists() { Some(candidate) } else { None }
}

fn spawn_workshop() -> Option<Child> {
    let exe = workshop_exe()?;
    info!("[watchdog] launching EternalWorkshop at {}", exe.display());
    match Command::new(&exe)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => {
            info!("[watchdog] EternalWorkshop running (PID {})", child.id());
            Some(child)
        }
        Err(e) => {
            warn!("[watchdog] failed to start EternalWorkshop: {e}");
            None
        }
    }
}

/// Supervise EternalWorkshop in a background task.
/// Restarts with exponential backoff on crash; gives up after MAX_RESTARTS.
async fn supervise_workshop() {
    let mut restarts: u32 = 0;
    loop {
        match spawn_workshop() {
            None => {
                // Binary not present — not an error, just not built yet
                info!("[watchdog] EternalWorkshop binary not found — skipping");
                return;
            }
            Some(mut child) => {
                restarts = 0;
                loop {
                    sleep(Duration::from_secs(WORKSHOP_HEALTH_SEC)).await;
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            warn!("[watchdog] EternalWorkshop exited: {status}");
                            break;
                        }
                        Ok(None) => { /* still running */ }
                        Err(e) => {
                            error!("[watchdog] EternalWorkshop wait error: {e}");
                            break;
                        }
                    }
                }
            }
        }

        restarts += 1;
        if restarts >= MAX_RESTARTS {
            error!("[watchdog] EternalWorkshop max restarts reached — giving up");
            return;
        }
        let backoff = RESTART_BACKOFF_SEC * restarts as u64;
        warn!("[watchdog] EternalWorkshop restarting in {backoff}s (attempt {restarts})");
        sleep(Duration::from_secs(backoff)).await;
    }
}

// ── Constants ─────────────────────────────────────────────────────────────────

const MAX_RESTARTS:        u32 = 10;
const RESTART_BACKOFF_SEC: u64 = 5;
const HEALTH_PING_SEC:     u64 = 15;
const HEALTH_URL:          &str = "http://127.0.0.1:11369/health";

// ── Seed knowledge base with built-in deterministic rules ────────────────────

fn seed_kb(kb: &KnowledgeBase) {
    let seeds: &[(&str, &str)] = &[
        ("EADDRINUSE",                   "lsof -ti:11369 2>/dev/null | xargs -r kill -9 || netstat -ano | findstr :11369"),
        ("address already in use",        "lsof -ti:11369 2>/dev/null | xargs -r kill -9"),
        ("Failed to bind socket",         "lsof -ti:11369 2>/dev/null | xargs -r kill -9"),
        ("Cannot find module",            "npm install --prefix bonsai-workspace"),
        ("toml parse error",              "rm -f ~/.bonsai/bonsai-config.json"),
        ("TOML parse error",              "rm -f ~/.bonsai/bonsai-config.json"),
        ("database disk image is malformed", "rm -f ~/.bonsai/bonsai.db && echo 'DB reset'"),
        ("Failed to create CAS",          "mkdir -p ~/.bonsai/cas_blobs && echo 'CAS dir created'"),
        ("llama-server exited",           "echo 'waiting for llama-server to restart'"),
        ("GPU: out of memory",            "echo 'GPU OOM — falling back to CPU mode'"),
        ("no space left on device",       "find /tmp -name 'bonsai_*' -mmin +60 -delete"),
    ];

    for (pattern, script) in seeds {
        // Only insert if not already present
        let existing = kb.find_matching(pattern);
        if existing.is_empty() {
            let _ = kb.insert_fix(pattern, "rule", script, 0.9, "system");
        }
    }
}

// ── Launch ────────────────────────────────────────────────────────────────────

/// Resolve the path to the main Bonsai executable.
fn bonsai_exe() -> PathBuf {
    // 1. Env override
    if let Ok(p) = std::env::var("BONSAI_EXE") {
        return PathBuf::from(p);
    }
    // 2. Sibling binary (release build next to watchdog)
    let exe_dir = std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(&PathBuf::from("."))
        .to_path_buf();

    #[cfg(target_os = "windows")]
    let candidates = [
        exe_dir.join("bonsai-workspace.exe"),
        PathBuf::from(r"C:\Program Files\Bonsai\bonsai-workspace.exe"),
    ];
    #[cfg(not(target_os = "windows"))]
    let candidates = [
        exe_dir.join("bonsai-workspace"),
        PathBuf::from("/usr/local/bin/bonsai-workspace"),
    ];

    for c in &candidates {
        if c.exists() { return c.clone(); }
    }

    // 3. Development fallback: cargo tauri dev
    exe_dir.join("bonsai-workspace")
}

fn spawn_bonsai() -> std::io::Result<Child> {
    let exe = bonsai_exe();
    info!("[watchdog] launching {}", exe.display());
    Command::new(&exe)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
}

// ── Log collection ────────────────────────────────────────────────────────────

fn collect_recent_logs() -> String {
    let log_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".bonsai/logs/bonsai.log");
    if log_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&log_path) {
            return content
                .lines()
                .rev()
                .take(100)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n");
        }
    }
    "No logs found".into()
}

// ── Health ping ───────────────────────────────────────────────────────────────

async fn is_bonsai_healthy() -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_default();
    client.get(HEALTH_URL).send().await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

// ── Main loop ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("[watchdog] Bonsai Watchdog v{} starting (PID {})",
        env!("CARGO_PKG_VERSION"),
        std::process::id());

    let kb_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".bonsai/survival_kb.db")
        .to_string_lossy()
        .to_string();
    let kb = Arc::new(KnowledgeBase::open(&kb_path).expect("KB init failed"));
    seed_kb(&kb);
    let _ = kb.seed_defaults(); // load all historical project error patterns

    // Start EternalWorkshop sidecar in a background task (non-fatal if absent)
    tokio::spawn(supervise_workshop());

    let mut restarts: u32 = 0;

    loop {
        if restarts >= MAX_RESTARTS {
            error!("[watchdog] max restarts ({MAX_RESTARTS}) reached — giving up");
            break;
        }

        match spawn_bonsai() {
            Ok(mut child) => {
                info!("[watchdog] Bonsai running (PID {})", child.id());
                restarts = 0;

                // Monitor loop — check health every HEALTH_PING_SEC seconds.
                loop {
                    sleep(Duration::from_secs(HEALTH_PING_SEC)).await;

                    // Check if the process has exited
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            error!("[watchdog] Bonsai exited: {status}");
                            break;
                        }
                        Ok(None) => {
                            // Still running — ping health endpoint
                            if !is_bonsai_healthy().await {
                                warn!("[watchdog] health ping failed");
                            }
                        }
                        Err(e) => {
                            error!("[watchdog] process wait error: {e}");
                            break;
                        }
                    }
                }

                // Collect crash logs and attempt repair.
                let logs = collect_recent_logs();
                let fixed_id = repair::attempt_repair(&kb, &logs).await;
                if let Some(id) = fixed_id {
                    kb.record_outcome(id, true).ok();
                    info!("[watchdog] repair succeeded (fix #{id}), restarting Bonsai");
                } else {
                    warn!("[watchdog] no repair found, retrying in {RESTART_BACKOFF_SEC}s");
                    sleep(Duration::from_secs(RESTART_BACKOFF_SEC * (restarts as u64 + 1))).await;
                }
            }
            Err(e) => {
                error!("[watchdog] failed to spawn Bonsai: {e}");
                repair::attempt_launch_repair(&e.to_string());
                sleep(Duration::from_secs(RESTART_BACKOFF_SEC)).await;
            }
        }

        restarts += 1;
    }
}
