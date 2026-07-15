//! Bonsai Watchdog
//!
//! A SQLite-backed self-healing knowledge base ([`kb`]) plus a deterministic
//! + AI-assisted repair engine ([`repair`]) used by the `watchdog` launch
//! supervisor binary (see `src/main.rs`) to detect crashes, look up known
//! fixes, and keep growing its fix library from successful repairs.

pub mod kb;
pub mod repair;

pub use kb::{FixEntry, KnowledgeBase, TrainingEntry, SEEDED_FIXES};
pub use repair::{ai_diagnose_and_fix, attempt_launch_repair, attempt_repair, run_script};

#[cfg(test)]
mod tests;
