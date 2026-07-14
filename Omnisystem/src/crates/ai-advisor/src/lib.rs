//! ai-advisor: a graceful-degradation AI advisory framework.
//!
//! Every consumer implements [`SovereignService`] with a deterministic
//! core (always correct, never fails) and optional heuristic/AI layers.
//! The [`Arbiter`] executes through the ladder AI -> heuristic ->
//! deterministic core -> safe stub, validating AI advice for latency,
//! confidence, and run-to-run consistency before trusting it.
//!
//! On top of that ladder sit two independent higher-level services:
//! multi-advisor request routing/aggregation ([`advisor_service`]) and
//! conflict detection/arbitration across multiple advisor opinions
//! ([`arbiter_orchestrator`]) - see those modules for their APIs (both
//! define their own `Arbiter`-adjacent types, so they're exposed via
//! their module paths rather than glob-exported at the crate root to
//! avoid name collisions with [`arbiter::Arbiter`]).

pub mod advisor_service;
pub mod advisory;
pub mod advisory_engine;
pub mod arbiter;
pub mod arbiter_orchestrator;
pub mod error;
pub mod metrics;
pub mod metrics_service;
pub mod service;

pub use advisory::{AdvisoryDomain, AdvisoryHealth, AdvisoryOutput, ConsistencyWindow, DisabledAdvisory};
pub use arbiter::{Arbiter, ArbiterConfig, ExecutionDecision};
pub use error::{Error, Result};
pub use metrics::{ArbiterMetrics, ArbiterState};
pub use service::{DeterministicCore, ExecutionResult, ExecutionTier, HeuristicLayer, SovereignService};
