//! bonsai-failure-finder — Forced Failure Finder (F³)
//!
//! Proactively discovers errors in every Bonsai component by running
//! sandboxed, resource-limited fuzzing campaigns. Every discovered failure
//! is automatically added to the Survival Knowledge Base.
//!
//! Architecture:
//!   F3Orchestrator
//!     → CampaignSpec (targets, strategies, resources)
//!     → FuzzWorker(s) (sandboxed, parallel)
//!       → Mutator (generates adversarial inputs)
//!       → ExecutionResult (Success | Crash | Hang | Assertion | Violation)
//!     → SurvivalBridge (dedup + persist to KB)

pub mod campaign;
pub mod orchestrator;
pub mod survival_bridge;
pub mod worker;

pub use campaign::{CampaignSpec, CampaignState, CampaignStatus, FuzzStrategy, TargetKind, ResourceBudget};
pub use orchestrator::{F3Orchestrator, OrchestratorStats};
pub use survival_bridge::SurvivalBridge;
pub use worker::{FailureReport, FuzzWorker};

/// Default path for the Survival KB database.
pub const DEFAULT_KB_PATH: &str = "~/.bonsai/survival_kb.db";
