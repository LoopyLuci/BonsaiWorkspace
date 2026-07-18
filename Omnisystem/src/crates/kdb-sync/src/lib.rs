//! KDB-Sync: collects per-project OmniLint rule-confidence metrics
//! (produced by the Phase A ETL pass, see [`etl`]), and aggregates them
//! across projects/domains/languages into consensus statistics --
//! confidence mean/std, false-positive rate, domain-specific variants, and
//! a recommended severity -- that feed the shared knowledge database (KDB)
//! used to tune lint rule behavior globally.

pub mod aggregator;
pub mod etl;
pub mod metrics;

pub use aggregator::{AggregatedMetrics, ProjectMetrics, RuleMetricsAggregator, RuleVariant};
pub use etl::RuleConfidenceMetrics;
pub use metrics::{MetricsCollector, MetricsSummary, RuleMetric};
