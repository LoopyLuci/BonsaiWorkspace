/// Core types for bug hunt findings, severity, and status.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Severity level of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Severity {
    #[serde(rename = "info")]
    Info = 0,
    #[serde(rename = "low")]
    Low = 1,
    #[serde(rename = "medium")]
    Medium = 2,
    #[serde(rename = "high")]
    High = 3,
    #[serde(rename = "critical")]
    Critical = 4,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Low => "low",
            Severity::Medium => "medium",
            Severity::High => "high",
            Severity::Critical => "critical",
        }
    }
}

/// Status of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    Open,
    Acknowledged,
    Fixed,
    FalsePositive,
    Ignored,
}

/// Represents a single issue found during scanning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: Uuid,
    pub file_path: PathBuf,
    pub line_start: usize,
    pub line_end: usize,
    pub column_start: Option<usize>,
    pub column_end: Option<usize>,
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub suggestion: Option<String>,
    pub suggested_diff: Option<String>,
    pub confidence: f32, // 0.0..=1.0
    pub analyzer: String,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub status: FindingStatus,
    pub fixed_by_commit: Option<String>,
    pub tags: Vec<String>,
}

impl Finding {
    pub fn new(
        file_path: PathBuf,
        line_start: usize,
        line_end: usize,
        rule_id: String,
        severity: Severity,
        message: String,
        analyzer: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            file_path,
            line_start,
            line_end,
            column_start: None,
            column_end: None,
            rule_id,
            severity,
            message,
            suggestion: None,
            suggested_diff: None,
            confidence: 0.5,
            analyzer,
            first_seen: now,
            last_seen: now,
            status: FindingStatus::Open,
            fixed_by_commit: None,
            tags: Vec::new(),
        }
    }

    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn with_diff(mut self, diff: String) -> Self {
        self.suggested_diff = Some(diff);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_columns(mut self, start: usize, end: usize) -> Self {
        self.column_start = Some(start);
        self.column_end = Some(end);
        self
    }
}

/// Summary of a scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub scan_id: String,
    pub repository: String,
    pub timestamp: DateTime<Utc>,
    pub files_scanned: usize,
    pub issues_found: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

impl ScanSummary {
    pub fn from_findings(findings: &[Finding], repository: String) -> Self {
        let mut summary = ScanSummary {
            scan_id: format!("scan-{}", Uuid::new_v4()),
            repository,
            timestamp: Utc::now(),
            files_scanned: findings.iter().map(|f| &f.file_path).collect::<std::collections::HashSet<_>>().len(),
            issues_found: findings.len(),
            critical: 0,
            high: 0,
            medium: 0,
            low: 0,
            info: 0,
        };

        for finding in findings {
            match finding.severity {
                Severity::Critical => summary.critical += 1,
                Severity::High => summary.high += 1,
                Severity::Medium => summary.medium += 1,
                Severity::Low => summary.low += 1,
                Severity::Info => summary.info += 1,
            }
        }

        summary
    }
}

/// A scan report combining summary and findings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReport {
    pub summary: ScanSummary,
    pub issues: Vec<Finding>,
}
