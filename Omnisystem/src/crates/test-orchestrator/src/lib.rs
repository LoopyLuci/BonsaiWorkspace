//! test-orchestrator (UTOF): a deterministic polyglot test harness.
//!
//! Loads a `TestSpec` (TOML), schedules one job per (language, test case)
//! pair, runs each job as a real subprocess, compares actual vs. expected
//! output with JSON-aware fidelity scoring, and stores/aggregates results.

pub mod comparer;
pub mod core;
pub mod error;
pub mod orchestrator;
pub mod runner;
pub mod scheduler;
pub mod spec;
pub mod storage;
pub mod types;

pub use comparer::{compare_outputs, ComparisonResult};
pub use core::Core;
pub use orchestrator::{Orchestrator, UtofConfig};
pub use runner::run_test;
pub use scheduler::{Job, Scheduler};
pub use spec::{TestCase, TestSpec};
pub use storage::{SpecStats, StorageEntry, TestResult, TestStatus, TestStorage};
pub use types::State;
