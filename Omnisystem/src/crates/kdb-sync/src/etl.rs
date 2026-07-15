//! Phase A ETL output types.
//!
//! "Phase A" is the OmniLint static-analysis pass that watches how
//! developers actually respond to lint findings (fixed, dismissed,
//! ignored) across a project and turns that into per-rule confidence
//! metrics. [`metrics::RuleMetric::from_etl_metrics`](crate::metrics::RuleMetric::from_etl_metrics)
//! turns a [`RuleConfidenceMetrics`] into the normalized [`crate::metrics::RuleMetric`]
//! shape that gets uploaded to KDB and aggregated across projects.

use serde::{Deserialize, Serialize};

/// Raw per-rule confidence signal produced by Phase A for a single project.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct RuleConfidenceMetrics {
    /// Findings the developer acted on (fixed the flagged issue).
    pub true_positives: u32,
    /// Findings the developer dismissed/ignored as not actually a problem.
    pub false_positives: u32,
    /// Findings dismissed without a clear signal either way.
    pub dismissed_count: u32,
    /// Findings where an auto-fix was applied.
    pub applied_fixes: u32,
    /// Fraction of applied fixes that stuck (weren't reverted/re-flagged).
    pub fix_success_rate: f32,
}

impl RuleConfidenceMetrics {
    pub fn new(
        true_positives: u32,
        false_positives: u32,
        dismissed_count: u32,
        applied_fixes: u32,
        fix_success_rate: f32,
    ) -> Self {
        Self {
            true_positives,
            false_positives,
            dismissed_count,
            applied_fixes,
            fix_success_rate,
        }
    }

    /// Total number of findings this metric was derived from.
    pub fn total_findings(&self) -> u32 {
        self.true_positives + self.false_positives + self.dismissed_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_findings_sums_all_categories() {
        let m = RuleConfidenceMetrics::new(10, 2, 3, 8, 0.9);
        assert_eq!(m.total_findings(), 15);
    }

    #[test]
    fn default_is_all_zero() {
        let m = RuleConfidenceMetrics::default();
        assert_eq!(m.total_findings(), 0);
    }
}
