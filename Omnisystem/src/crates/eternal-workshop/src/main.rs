#![allow(clippy::ptr_arg, clippy::unnecessary_map_or)]

//! EternalWorkshop — BonsAI's background memory consolidation daemon.
//!
//! Runs alongside the main Bonsai Workspace app (started by the Watchdog).
//! Performs nightly memory consolidation using the DreamAgent model, then
//! rewrites BONSAI.md with the day's learnings.
//!
//! The daemon communicates with the main app via:
//!   - SQLite (shared `memory_nodes.db` in the app data directory)
//!   - HTTP POST to the app's local API (port 11369) to emit Tauri events
//!
//! Usage:
//!   eternal-workshop [--db-path <path>] [--api-port <port>] [--dream-agent-port <port>]

mod config;
mod dream_executor;
mod memory_nodes;
mod scheduler;

use log::{error, info};
use tokio::signal;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cfg = config::Config::from_env_or_defaults();
    info!(
        "[eternal-workshop] starting — db={} api=:{} dream=:{}",
        cfg.db_path.display(),
        cfg.api_port,
        cfg.dream_agent_port
    );

    let store = match memory_nodes::MemoryNodeStore::open(&cfg.db_path).await {
        Ok(s) => s,
        Err(e) => {
            error!("[eternal-workshop] failed to open memory store: {e}");
            std::process::exit(1);
        }
    };

    let sched = scheduler::Scheduler::new(store, cfg.clone());

    tokio::select! {
        _ = sched.run() => {
            info!("[eternal-workshop] scheduler exited");
        }
        _ = signal::ctrl_c() => {
            info!("[eternal-workshop] received Ctrl-C, shutting down");
        }
    }
}
