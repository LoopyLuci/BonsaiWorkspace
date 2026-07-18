//! EternalWorkshop: BonsAI's background memory consolidation daemon.
//!
//! Periodically (nightly, or after a period of user idleness) reads pending
//! "memory node" activity records from a shared SQLite database, asks a
//! local DreamAgent sidecar LLM to consolidate them into concise insights
//! (falling back to a deterministic dedup heuristic if the sidecar is
//! unavailable), appends the result to the workspace's `BONSAI.md`, and
//! notifies the main app over HTTP.

pub mod config;
pub mod dream_executor;
pub mod memory_nodes;
pub mod scheduler;

pub use config::Config;
pub use memory_nodes::{MemoryNode, MemoryNodeStore};
pub use scheduler::Scheduler;
