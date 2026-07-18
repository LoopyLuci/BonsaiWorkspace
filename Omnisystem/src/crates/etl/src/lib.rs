//! etl (EternalTrainingLoop): a feedback-driven confidence system for
//! lint rules.
//!
//! User feedback on diagnostics (fix applied, false positive reported,
//! dismissed, manually fixed) is collected ([`feedback::FeedbackCollector`])
//! into storage ([`storage::ETLStorage`], in-memory, or
//! [`storage_sqlx::SqlxStorage`] for a real database), aggregated into
//! per-rule confidence metrics ([`confidence::RuleConfidenceCalculator`]),
//! used to recommend severity actions (promote/demote/disable) applied
//! via [`adjuster::RuleConfidenceAdjuster`], and low-confidence rules get
//! refinement proposals from [`refiner::RuleRefiner`]. Everything is
//! observable via [`events::UniverseEventEmitter`] /
//! [`universe_bridge`]. [`orchestrator::EternalTrainingLoop`] ties one
//! full cycle together.

pub mod adjuster;
pub mod confidence;
pub mod events;
pub mod feedback;
pub mod lint_integration;
pub mod orchestrator;
pub mod refiner;
pub mod storage;
pub mod storage_sqlx;
pub mod universe_bridge;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use adjuster::RuleConfidenceAdjuster;
pub use confidence::{RuleConfidenceCalculator, RuleConfidenceMetrics};
pub use events::{FeedbackEvent, FeedbackEventType, UniverseEventEmitter};
pub use feedback::FeedbackCollector;
pub use orchestrator::{CycleResult, EternalTrainingLoop};
pub use refiner::{RuleMutationProposal, RuleRefiner};
pub use storage::ETLStorage;
pub use storage_sqlx::SqlxStorage;

/// A confidence update to apply to a single rule, produced by one ETL
/// cycle. Referenced throughout the archived source (adjuster, events,
/// universe_bridge, and the pre-written test suite) but never actually
/// defined anywhere in the archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfidenceUpdate {
    pub rule_id: String,
    pub old_confidence: f32,
    pub new_confidence: f32,
    pub action: String,
    pub true_positives: u32,
    pub false_positives: u32,
    pub dismissed_count: u32,
    pub timestamp: DateTime<Utc>,
}
