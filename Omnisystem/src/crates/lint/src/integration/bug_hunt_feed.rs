/// Integration with Bonsai Bug Hunt system.
/// Linting findings feed into the Bug Hunt orchestrator for prioritization and auto-fixing.

use crate::diagnostics::Diagnostic;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct BugHuntFinding {
    pub bug_id: String,
    pub rule_id: String,
    pub severity: String,
    pub file: String,
    pub line: u32,
    pub description: String,
    pub can_auto_fix: bool,
}

impl BugHuntFinding {
    pub fn from_diagnostic(diag: &Diagnostic) -> Self {
        let bug_id = uuid::Uuid::new_v4().to_string();
        Self {
            bug_id,
            rule_id: diag.rule_id.clone(),
            severity: diag.severity.to_string(),
            file: diag.file.to_string_lossy().to_string(),
            line: diag.range.start.line,
            description: diag.message.clone(),
            can_auto_fix: diag.fix.is_some(),
        }
    }
}

/// Feed linting diagnostics to the Bug Hunt system.
pub async fn feed_diagnostics_to_bug_hunt(diagnostics: &[Diagnostic]) -> Result<()> {
    let findings: Vec<BugHuntFinding> = diagnostics
        .iter()
        .map(BugHuntFinding::from_diagnostic)
        .collect();

    tracing::info!("Feeding {} findings to Bug Hunt", findings.len());

    // TODO: Send to bonsai-bug-hunt orchestrator
    // This would create bug hunt entries that can be auto-fixed

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Range, Position, Severity};
    use std::path::PathBuf;

    #[test]
    fn test_bug_hunt_finding_from_diagnostic() {
        let range = Range::new(
            Position { line: 5, column: 10 },
            Position { line: 5, column: 20 },
            100,
            110,
        );
        let diag = Diagnostic::new(
            "test-rule",
            "Test issue",
            Severity::Warning,
            PathBuf::from("main.rs"),
            range,
        );

        let finding = BugHuntFinding::from_diagnostic(&diag);
        assert_eq!(finding.rule_id, "test-rule");
        assert_eq!(finding.line, 5);
    }
}
