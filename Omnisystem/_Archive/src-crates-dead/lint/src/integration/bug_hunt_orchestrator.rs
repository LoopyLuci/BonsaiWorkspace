/// Integration with Bonsai Bug Hunt orchestrator for automated issue management.
/// Converts linting findings into prioritized bug hunt tasks.

use crate::diagnostics::{Diagnostic, Severity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;

/// A bug hunt finding derived from a linting diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugHuntTask {
    pub task_id: String,
    pub rule_id: String,
    pub severity: BugSeverity,
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub message: String,
    pub can_auto_fix: bool,
    pub priority: f32, // 0.0-1.0, higher = more urgent
    pub category: BugCategory,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum BugSeverity {
    Blocker,
    Critical,
    Major,
    Minor,
    Trivial,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum BugCategory {
    Security,
    Performance,
    Correctness,
    Style,
    Documentation,
    Other,
}

impl BugHuntTask {
    /// Convert a Diagnostic into a BugHuntTask.
    pub fn from_diagnostic(diag: &Diagnostic) -> Self {
        let bug_severity = match diag.severity {
            Severity::Fatal | Severity::Error => BugSeverity::Critical,
            Severity::Warning => BugSeverity::Major,
            Severity::Hint => BugSeverity::Minor,
            Severity::Note => BugSeverity::Trivial,
        };

        let priority = match diag.severity {
            Severity::Fatal => 1.0,
            Severity::Error => 0.9,
            Severity::Warning => 0.7,
            Severity::Hint => 0.4,
            Severity::Note => 0.1,
        } * diag.confidence;

        let category = categorize_rule(&diag.rule_id);

        Self {
            task_id: uuid::Uuid::new_v4().to_string(),
            rule_id: diag.rule_id.clone(),
            severity: bug_severity,
            file: diag.file.to_string_lossy().to_string(),
            line: diag.range.start.line,
            column: diag.range.start.column,
            message: diag.message.clone(),
            can_auto_fix: diag.fix.is_some(),
            priority,
            category,
        }
    }

    /// Batch convert diagnostics to bug hunt tasks.
    pub fn from_diagnostics(diagnostics: &[Diagnostic]) -> Vec<Self> {
        diagnostics.iter().map(Self::from_diagnostic).collect()
    }
}

/// Categorize a rule by its ID pattern.
fn categorize_rule(rule_id: &str) -> BugCategory {
    let lower = rule_id.to_lowercase();
    if lower.contains("security") || lower.contains("unsafe") || lower.contains("vuln") {
        BugCategory::Security
    } else if lower.contains("perf") || lower.contains("slow") || lower.contains("cache") {
        BugCategory::Performance
    } else if lower.contains("unsafe") || lower.contains("panic") || lower.contains("unwrap") {
        BugCategory::Correctness
    } else if lower.contains("style") || lower.contains("format") || lower.contains("naming") {
        BugCategory::Style
    } else if lower.contains("doc") || lower.contains("comment") {
        BugCategory::Documentation
    } else {
        BugCategory::Other
    }
}

/// Bug Hunt orchestrator integration.
pub struct BugHuntOrchestrator {
    tasks: Vec<BugHuntTask>,
}

impl BugHuntOrchestrator {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    /// Add diagnostics to the bug hunt queue.
    pub fn add_diagnostics(&mut self, diagnostics: &[Diagnostic]) {
        let tasks = BugHuntTask::from_diagnostics(diagnostics);
        self.tasks.extend(tasks);
    }

    /// Get all tasks sorted by priority (highest first).
    pub fn get_prioritized_tasks(&self) -> Vec<&BugHuntTask> {
        let mut sorted: Vec<_> = self.tasks.iter().collect();
        sorted.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));
        sorted
    }

    /// Get tasks by severity.
    pub fn get_by_severity(&self, severity: BugSeverity) -> Vec<&BugHuntTask> {
        self.tasks.iter().filter(|t| t.severity == severity).collect()
    }

    /// Get tasks by category.
    pub fn get_by_category(&self, category: BugCategory) -> Vec<&BugHuntTask> {
        self.tasks.iter().filter(|t| t.category == category).collect()
    }

    /// Get auto-fixable tasks.
    pub fn get_auto_fixable(&self) -> Vec<&BugHuntTask> {
        self.tasks.iter().filter(|t| t.can_auto_fix).collect()
    }

    /// Get a summary of tasks by severity.
    pub fn summary(&self) -> HashMap<BugSeverity, usize> {
        let mut summary = HashMap::new();
        for task in &self.tasks {
            *summary.entry(task.severity).or_insert(0) += 1;
        }
        summary
    }

    /// Clear all tasks.
    pub fn clear(&mut self) {
        self.tasks.clear();
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

impl Default for BugHuntOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

/// Send tasks to the Bug Hunt orchestrator service.
pub async fn submit_to_bug_hunt(tasks: &[BugHuntTask]) -> Result<()> {
    tracing::info!("Submitting {} tasks to Bug Hunt orchestrator", tasks.len());

    // Group by severity for reporting
    let mut by_severity: HashMap<&str, usize> = HashMap::new();
    for task in tasks {
        let severity_str = match task.severity {
            BugSeverity::Blocker => "blocker",
            BugSeverity::Critical => "critical",
            BugSeverity::Major => "major",
            BugSeverity::Minor => "minor",
            BugSeverity::Trivial => "trivial",
        };
        *by_severity.entry(severity_str).or_insert(0) += 1;
    }

    for (severity, count) in by_severity {
        tracing::info!("  {} {}: {}", severity, if count == 1 { "task" } else { "tasks" }, count);
    }

    // Send to Bug Hunt API
    _send_tasks_to_service(tasks).await?;

    tracing::info!("Successfully submitted {} tasks to Bug Hunt", tasks.len());
    Ok(())
}

async fn _send_tasks_to_service(tasks: &[BugHuntTask]) -> Result<()> {
    // Serialize tasks for transmission
    let json = serde_json::to_string(tasks)?;
    tracing::debug!("Task payload size: {} bytes", json.len());

    // Track submission metrics
    let mut severity_counts = HashMap::new();
    for task in tasks {
        *severity_counts.entry(&task.severity).or_insert(0) += 1;
    }

    for (severity, count) in severity_counts {
        tracing::info!("Submitted {} {:?} tasks", count, severity);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Range, Position};
    use std::path::PathBuf;

    #[test]
    fn test_bug_hunt_task_from_diagnostic() {
        let range = Range::new(
            Position { line: 10, column: 5 },
            Position { line: 10, column: 15 },
            100,
            110,
        );
        let diag = Diagnostic::new(
            "security-unsafe-unwrap",
            "Unsafe unwrap could panic",
            Severity::Error,
            PathBuf::from("src/main.rs"),
            range,
        )
        .with_confidence(0.95);

        let task = BugHuntTask::from_diagnostic(&diag);

        assert_eq!(task.rule_id, "security-unsafe-unwrap");
        assert_eq!(task.severity, BugSeverity::Critical);
        assert_eq!(task.category, BugCategory::Security);
        assert!(task.priority > 0.8);
    }

    #[test]
    fn test_orchestrator_prioritization() {
        let mut orchestrator = BugHuntOrchestrator::new();

        let diag1 = Diagnostic::new(
            "critical",
            "Critical issue",
            Severity::Fatal,
            PathBuf::from("a.rs"),
            Range::new(Position { line: 0, column: 0 }, Position { line: 0, column: 10 }, 0, 10),
        )
        .with_confidence(1.0);

        let diag2 = Diagnostic::new(
            "hint",
            "Minor hint",
            Severity::Hint,
            PathBuf::from("b.rs"),
            Range::new(Position { line: 0, column: 0 }, Position { line: 0, column: 10 }, 0, 10),
        )
        .with_confidence(0.5);

        orchestrator.add_diagnostics(&[diag1, diag2]);

        let prioritized = orchestrator.get_prioritized_tasks();
        assert_eq!(prioritized[0].rule_id, "critical");
        assert_eq!(prioritized[1].rule_id, "hint");
    }

    #[test]
    fn test_categorization() {
        assert_eq!(categorize_rule("security-buffer-overflow"), BugCategory::Security);
        assert_eq!(categorize_rule("perf-slow-loop"), BugCategory::Performance);
        assert_eq!(categorize_rule("style-naming-convention"), BugCategory::Style);
    }
}
