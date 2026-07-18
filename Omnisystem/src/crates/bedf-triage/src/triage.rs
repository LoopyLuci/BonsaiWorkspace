//! Triage engine: ties crash deduplication and fix generation together into
//! a single pipeline for processing incoming crash reports.

use crate::config::TriageConfig;
use crate::crash_dedup::{CrashDeduplicator, CrashSignature};
use crate::error::{Error, Result};
use crate::fix_generator::{FixGenerator, GeneratedFix};
use serde::{Deserialize, Serialize};

/// A raw crash report submitted to the triage engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashReport {
    pub id: String,
    pub stack_trace: String,
}

impl CrashReport {
    pub fn new(id: impl Into<String>, stack_trace: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            stack_trace: stack_trace.into(),
        }
    }
}

/// The outcome of triaging a single crash report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageResult {
    pub report_id: String,
    pub signature: CrashSignature,
    pub is_duplicate: bool,
    pub suggested_fix: Option<GeneratedFix>,
}

/// Coordinates deduplication and fix generation for a stream of crash
/// reports, respecting the configured limits.
pub struct TriageEngine {
    config: TriageConfig,
    dedup: CrashDeduplicator,
    fixer: FixGenerator,
    fixes_generated: usize,
}

impl TriageEngine {
    pub fn new(config: TriageConfig) -> Self {
        Self {
            config,
            dedup: CrashDeduplicator::new(),
            fixer: FixGenerator::new(),
            fixes_generated: 0,
        }
    }

    /// Process a single crash report: compute its signature, determine
    /// whether it's a duplicate of a previously seen crash, and (for new,
    /// unique crashes) attempt to generate a fix suggestion.
    pub fn triage(&mut self, report: &CrashReport) -> Result<TriageResult> {
        if report.stack_trace.trim().is_empty() {
            return Err(Error::EmptyStackTrace);
        }

        if !self.config.enabled {
            let signature = self.dedup.compute_signature(&report.stack_trace);
            return Ok(TriageResult {
                report_id: report.id.clone(),
                signature,
                is_duplicate: false,
                suggested_fix: None,
            });
        }

        let signature = self.dedup.compute_signature(&report.stack_trace);
        let is_duplicate = self.dedup.is_duplicate(&signature);

        if !is_duplicate {
            self.dedup.record_crash(signature.clone());
        }

        let suggested_fix = if !is_duplicate
            && self.config.enable_ai_fixes
            && self.fixes_generated < self.config.max_fix_suggestions
        {
            let fix = self.fixer.generate_fix(&report.stack_trace);
            if fix.is_some() {
                self.fixes_generated += 1;
            }
            fix
        } else {
            None
        };

        Ok(TriageResult {
            report_id: report.id.clone(),
            signature,
            is_duplicate,
            suggested_fix,
        })
    }

    /// Process a batch of crash reports in order, skipping (rather than
    /// failing the whole batch on) any individual report that can't be
    /// triaged.
    pub fn triage_batch(&mut self, reports: &[CrashReport]) -> Vec<TriageResult> {
        reports
            .iter()
            .filter_map(|r| self.triage(r).ok())
            .collect()
    }

    pub fn unique_crash_count(&self) -> usize {
        self.dedup.total_unique_crashes()
    }

    pub fn fixes_generated(&self) -> usize {
        self.fixes_generated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triage_new_crash_generates_fix() {
        let mut engine = TriageEngine::new(TriageConfig::default());
        let report = CrashReport::new("r1", "thread panicked at 'index out of bounds'");
        let result = engine.triage(&report).unwrap();
        assert!(!result.is_duplicate);
        assert!(result.suggested_fix.is_some());
        assert_eq!(engine.unique_crash_count(), 1);
    }

    #[test]
    fn triage_duplicate_crash_is_flagged_and_no_fix_regenerated() {
        let mut engine = TriageEngine::new(TriageConfig::default());
        let report = CrashReport::new("r1", "null pointer dereference");
        let first = engine.triage(&report).unwrap();
        assert!(!first.is_duplicate);

        let dup_report = CrashReport::new("r2", "null pointer dereference");
        let second = engine.triage(&dup_report).unwrap();
        assert!(second.is_duplicate);
        assert!(second.suggested_fix.is_none());
        assert_eq!(engine.unique_crash_count(), 1);
    }

    #[test]
    fn triage_empty_stack_trace_errors() {
        let mut engine = TriageEngine::new(TriageConfig::default());
        let report = CrashReport::new("r1", "   ");
        assert!(matches!(engine.triage(&report), Err(Error::EmptyStackTrace)));
    }

    #[test]
    fn triage_respects_max_fix_suggestions() {
        let mut config = TriageConfig::default();
        config.max_fix_suggestions = 1;
        let mut engine = TriageEngine::new(config);

        let r1 = CrashReport::new("r1", "index out of bounds");
        let r2 = CrashReport::new("r2", "null pointer dereference");

        let res1 = engine.triage(&r1).unwrap();
        let res2 = engine.triage(&r2).unwrap();

        assert!(res1.suggested_fix.is_some());
        assert!(res2.suggested_fix.is_none(), "should stop generating fixes past the configured max");
        assert_eq!(engine.fixes_generated(), 1);
    }

    #[test]
    fn triage_disabled_engine_skips_fix_generation() {
        let mut config = TriageConfig::default();
        config.enabled = false;
        let mut engine = TriageEngine::new(config);

        let report = CrashReport::new("r1", "index out of bounds");
        let result = engine.triage(&report).unwrap();
        assert!(result.suggested_fix.is_none());
        assert_eq!(engine.unique_crash_count(), 0);
    }

    #[test]
    fn triage_batch_processes_all_reports() {
        let mut engine = TriageEngine::new(TriageConfig::default());
        let reports = vec![
            CrashReport::new("r1", "index out of bounds"),
            CrashReport::new("r2", "index out of bounds"),
            CrashReport::new("r3", "deadlock detected"),
        ];
        let results = engine.triage_batch(&reports);
        assert_eq!(results.len(), 3);
        assert!(!results[0].is_duplicate);
        assert!(results[1].is_duplicate);
        assert!(!results[2].is_duplicate);
    }
}
