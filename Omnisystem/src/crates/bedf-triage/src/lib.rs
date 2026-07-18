//! bedf-triage: crash deduplication and automated fix suggestion pipeline.
//!
//! Incoming crash reports are hashed into a stable signature (via
//! [`crash_dedup::CrashDeduplicator`]) so repeat crashes with identical
//! stack traces are collapsed into a single unique bucket, and new/unique
//! crashes are matched against a small library of known failure patterns
//! (via [`fix_generator::FixGenerator`]) to suggest a fix. [`triage::TriageEngine`]
//! ties both stages together into a single pipeline.

pub mod config;
pub mod crash_dedup;
pub mod error;
pub mod fix_generator;
pub mod triage;

pub use config::{Config, TriageConfig};
pub use crash_dedup::{CrashDeduplicator, CrashSignature};
pub use error::{Error, Result};
pub use fix_generator::{FixGenerator, GeneratedFix};
pub use triage::{CrashReport, TriageEngine, TriageResult};
