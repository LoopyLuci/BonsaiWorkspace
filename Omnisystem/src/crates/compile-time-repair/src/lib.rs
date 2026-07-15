//! compile-time-repair: heuristic regex-based Rust source analysis and
//! auto-repair. Detects common issues (unused variables/imports, missing
//! returns, null-pointer / buffer-overflow heuristics, doc-comment issues)
//! and can apply file-level repairs, logging each repair to a persistent
//! history database.

pub mod analyzer;
pub mod database;
pub mod patterns;
pub mod repair_engine;

pub use analyzer::{CompileError, CompileTimeAnalyzer, ErrorType};
pub use database::{RepairDatabase, RepairRecord, RepairStatistics};
pub use patterns::{PatternDatabase, RepairPattern, TestCase};
pub use repair_engine::{Repair, RepairEngine};
