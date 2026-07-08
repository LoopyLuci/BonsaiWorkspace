//! Automatic Bug Report Filing
//!
//! Detects issues from test divergences and automatically files bug reports.
//! Integrates with external issue tracking systems (GitHub, GitLab, Jira, etc.).

use polyglot_pong_common::*;
use tracing::{info, warn};
use uuid::Uuid;

/// Bug reporter for filing automatic reports.
pub struct BugReporter {
    pub tracker_url: String,
    pub filed_bugs: Vec<FiledBug>,
}

/// A filed bug with tracking information.
#[derive(Debug, Clone)]
pub struct FiledBug {
    pub bug_id: String,
    pub report: BugReport,
    pub external_id: Option<String>,
    pub status: BugStatus,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub enum BugStatus {
    Filed,
    Acknowledged,
    Assigned,
    Fixed,
    Closed,
}

impl BugReporter {
    /// Create a new bug reporter.
    pub fn new(tracker_url: &str) -> Self {
        Self {
            tracker_url: tracker_url.into(),
            filed_bugs: Vec::new(),
        }
    }

    /// File a bug report.
    pub async fn file_report(&mut self, report: &BugReport) -> anyhow::Result<FiledBug> {
        info!(
            "Filing bug: {} → {} ({})",
            report.source_lang, report.target_lang, report.failure_type
        );

        // Construct bug report title and body
        let title = self.format_title(report);
        let body = self.format_body(report);

        // In production: POST to tracker API
        // For now: simulate filing
        let external_id = Some(format!("POLYGLOT-{}", Uuid::new_v4().simple()));

        let filed = FiledBug {
            bug_id: report.bug_id.clone(),
            report: report.clone(),
            external_id,
            status: BugStatus::Filed,
            timestamp: chrono::Utc::now(),
        };

        self.filed_bugs.push(filed.clone());
        Ok(filed)
    }

    /// Format bug title.
    fn format_title(&self, report: &BugReport) -> String {
        format!(
            "[POLYGLOT-PONG] {} in {} → {} (seed: {})",
            report.failure_type, report.source_lang, report.target_lang, report.universe_hash
        )
    }

    /// Format bug report body.
    fn format_body(&self, report: &BugReport) -> String {
        format!(
            "## Reproduction\n\n```\n{}\n```\n\n## Details\n\n- Source Language: {}\n- Target Language: {}\n- Failure Type: {}\n- Compiler Version: {}\n- Universe Hash: {}\n- Timestamp: {}",
            report.minimized_source,
            report.source_lang,
            report.target_lang,
            report.failure_type,
            report.compiler_version,
            report.universe_hash,
            chrono::Utc::now()
        )
    }

    /// Get all filed bugs.
    pub fn all_bugs(&self) -> &[FiledBug] {
        &self.filed_bugs
    }

    /// Get bugs by status.
    pub fn bugs_by_status(&self, status: BugStatus) -> Vec<&FiledBug> {
        self.filed_bugs
            .iter()
            .filter(|b| std::mem::discriminant(&b.status) == std::mem::discriminant(&status))
            .collect()
    }

    /// Generate a bug report summary.
    pub fn summary(&self) -> BugSummary {
        let total = self.filed_bugs.len();
        let by_type = self.count_by_failure_type();
        let by_language = self.count_by_language_pair();

        BugSummary {
            total_bugs: total,
            by_failure_type: by_type,
            by_language_pair: by_language,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Count bugs by failure type.
    fn count_by_failure_type(&self) -> std::collections::HashMap<String, usize> {
        let mut counts = std::collections::HashMap::new();

        for bug in &self.filed_bugs {
            *counts
                .entry(format!("{:?}", bug.report.failure_type))
                .or_insert(0) += 1;
        }

        counts
    }

    /// Count bugs by language pair.
    fn count_by_language_pair(&self) -> std::collections::HashMap<(Language, Language), usize> {
        let mut counts = std::collections::HashMap::new();

        for bug in &self.filed_bugs {
            *counts
                .entry((
                    bug.report.source_lang.clone(),
                    bug.report.target_lang.clone(),
                ))
                .or_insert(0) += 1;
        }

        counts
    }
}

/// Bug report summary.
#[derive(Debug, Clone)]
pub struct BugSummary {
    pub total_bugs: usize,
    pub by_failure_type: std::collections::HashMap<String, usize>,
    pub by_language_pair: std::collections::HashMap<(Language, Language), usize>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl BugSummary {
    /// Get top N problematic language pairs.
    pub fn top_problematic_pairs(&self, n: usize) -> Vec<((Language, Language), usize)> {
        let mut pairs: Vec<_> = self.by_language_pair.iter().map(|(k, v)| (k.clone(), *v)).collect();
        pairs.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        pairs.truncate(n);
        pairs
    }

    /// Export summary as text.
    pub fn to_text(&self) -> String {
        let mut text = format!("Bug Summary\n============\n");
        text.push_str(&format!("Total Bugs: {}\n", self.total_bugs));
        text.push_str(&format!("Timestamp: {}\n\n", self.timestamp));

        text.push_str("By Failure Type:\n");
        for (typ, count) in &self.by_failure_type {
            text.push_str(&format!("  {}: {}\n", typ, count));
        }

        text.push_str("\nTop Problematic Language Pairs:\n");
        for ((src, tgt), count) in self.top_problematic_pairs(10) {
            text.push_str(&format!("  {} → {}: {}\n", src, tgt, count));
        }

        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bug_reporter_creation() {
        let reporter = BugReporter::new("http://localhost:3000/api/bugs");
        assert_eq!(reporter.filed_bugs.len(), 0);
    }

    #[test]
    fn test_format_title() {
        let report = BugReport::new(
            "Rust".into(),
            "Python".into(),
            TestId(uuid::Uuid::new_v4()),
            FailureType::CompilationError,
            "code".into(),
            "rustc 1.7".into(),
        );

        let reporter = BugReporter::new("http://example.com");
        let title = reporter.format_title(&report);
        assert!(title.contains("Rust"));
        assert!(title.contains("Python"));
    }

    #[tokio::test]
    async fn test_file_report() {
        let mut reporter = BugReporter::new("http://localhost:3000/api/bugs");
        let report = BugReport::new(
            "Rust".into(),
            "Python".into(),
            TestId(uuid::Uuid::new_v4()),
            FailureType::BehaviouralDivergence,
            "test code".into(),
            "rustc 1.7".into(),
        );

        let filed = reporter.file_report(&report).await.unwrap();
        assert_eq!(filed.report.source_lang, "Rust");
        assert_eq!(reporter.filed_bugs.len(), 1);
    }

    #[test]
    fn test_bug_summary() {
        let reporter = BugReporter {
            tracker_url: "http://example.com".into(),
            filed_bugs: vec![],
        };

        let summary = reporter.summary();
        assert_eq!(summary.total_bugs, 0);
    }
}
