//! Integration layer that connects MCP tool handlers to the bonsai-lint crate.
//! This module bridges incoming MCP requests with the linting engine and emits results.

use crate::lint_commands::{
    LintFileRequest, LintRepoRequest, GenerateLintRuleRequest, ExplainDiagnosticRequest,
    FalsePositiveRequest, DismissDiagnosticRequest, ApplyFixRequest,
    handle_lint_file, handle_lint_repo, handle_generate_lint_rule, handle_explain_diagnostic,
    handle_report_false_positive, handle_dismiss_diagnostic, handle_apply_fix,
};
use serde_json::{json, Value};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Shared state for linting integration.
pub struct LintingState {
    /// Broadcast channel for sending lint events to subscribers (IDE, etc).
    pub events_tx: broadcast::Sender<LintEvent>,
}

#[derive(Debug, Clone)]
pub enum LintEvent {
    LintStarted { path: String },
    LintCompleted { diagnostics_count: usize, duration_ms: u128 },
    DiagnosticGenerated { rule_id: String, message: String },
}

impl LintingState {
    pub fn new() -> (Self, broadcast::Receiver<LintEvent>) {
        let (tx, rx) = broadcast::channel(100);
        (Self { events_tx: tx }, rx)
    }

    /// Process a bonsai_lint_file tool call.
    pub async fn process_lint_file(&self, path: String, confidence_threshold: Option<f32>) -> Result<Value> {
        // Emit start event
        let _ = self.events_tx.send(LintEvent::LintStarted { path: path.clone() });

        let req = LintFileRequest {
            path,
            confidence_threshold,
        };

        let result = handle_lint_file(req).await?;

        // Emit completion event
        let _ = self.events_tx.send(LintEvent::LintCompleted {
            diagnostics_count: result.summary.total_diagnostics,
            duration_ms: result.summary.duration_ms,
        });

        // Emit individual diagnostic events
        for diag in &result.diagnostics {
            let _ = self.events_tx.send(LintEvent::DiagnosticGenerated {
                rule_id: diag.rule_id.clone(),
                message: diag.message.clone(),
            });
        }

        // Convert to JSON response
        Ok(serde_json::to_value(result)?)
    }

    /// Process a bonsai_lint_repo tool call.
    pub async fn process_lint_repo(
        &self,
        exclude_patterns: Option<Vec<String>>,
        confidence_threshold: Option<f32>,
        ai_filtering: Option<bool>,
        spell_check: Option<bool>,
    ) -> Result<Value> {
        let _ = self.events_tx.send(LintEvent::LintStarted { path: ".".to_string() });

        let req = LintRepoRequest {
            exclude_patterns,
            confidence_threshold,
            ai_filtering,
            spell_check,
        };

        let result = handle_lint_repo(req).await?;

        let _ = self.events_tx.send(LintEvent::LintCompleted {
            diagnostics_count: result.summary.total_diagnostics,
            duration_ms: result.summary.duration_ms,
        });

        for diag in &result.diagnostics {
            let _ = self.events_tx.send(LintEvent::DiagnosticGenerated {
                rule_id: diag.rule_id.clone(),
                message: diag.message.clone(),
            });
        }

        Ok(serde_json::to_value(result)?)
    }

    /// Process a bonsai_generate_lint_rule tool call.
    pub async fn process_generate_rule(
        &self,
        description: String,
        language: Option<String>,
        severity: Option<String>,
        example_good: Option<String>,
        example_bad: Option<String>,
    ) -> Result<Value> {
        let req = GenerateLintRuleRequest {
            description,
            language,
            severity,
            example_good,
            example_bad,
        };

        let result = handle_generate_lint_rule(req).await?;
        Ok(result)
    }

    /// Process a bonsai_explain_diagnostic tool call.
    pub async fn process_explain_diagnostic(
        &self,
        rule_id: String,
        code_snippet: String,
        language: Option<String>,
        message: Option<String>,
    ) -> Result<Value> {
        let req = ExplainDiagnosticRequest {
            rule_id,
            code_snippet,
            language,
            message,
        };

        let result = handle_explain_diagnostic(req).await?;
        Ok(result)
    }

    /// Process a bonsai_report_false_positive tool call.
    /// Collects feedback that this diagnostic is not applicable.
    pub async fn process_report_false_positive(
        &self,
        rule_id: String,
        file: String,
        line: u32,
        explanation: String,
    ) -> Result<Value> {
        let req = FalsePositiveRequest {
            rule_id,
            file,
            line,
            explanation,
        };

        let result = handle_report_false_positive(req).await?;
        Ok(result)
    }

    /// Process a bonsai_dismiss_diagnostic tool call.
    /// Collects feedback that user dismissed a diagnostic.
    pub async fn process_dismiss_diagnostic(
        &self,
        rule_id: String,
        file: String,
        line: u32,
    ) -> Result<Value> {
        let req = DismissDiagnosticRequest {
            rule_id,
            file,
            line,
        };

        let result = handle_dismiss_diagnostic(req).await?;
        Ok(result)
    }

    /// Process a bonsai_apply_fix tool call.
    /// Collects feedback that user successfully applied a fix.
    pub async fn process_apply_fix(
        &self,
        rule_id: String,
        file: String,
        line: u32,
        fix: Option<String>,
    ) -> Result<Value> {
        let req = ApplyFixRequest {
            rule_id,
            file,
            line,
            fix,
        };

        let result = handle_apply_fix(req).await?;
        Ok(result)
    }
}

impl Default for LintingState {
    fn default() -> Self {
        Self::new().0
    }
}

/// Global linting state (should be Arc<RwLock<>> in a real server).
pub fn get_linting_state() -> Arc<LintingState> {
    // In production, this would be managed by the server startup
    Arc::new(LintingState::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lint_file_integration() -> Result<()> {
        let (state, mut rx) = LintingState::new();

        let result = state.process_lint_file("src/main.rs".to_string(), Some(0.7)).await?;

        assert!(result.get("success").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(rx.recv().await.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_lint_repo_integration() -> Result<()> {
        let (state, mut _rx) = LintingState::new();

        let result = state.process_lint_repo(
            Some(vec!["target/**".to_string()]),
            Some(0.7),
            Some(true),
            Some(true),
        )
        .await?;

        assert!(result.get("success").and_then(|v| v.as_bool()).unwrap_or(false));

        Ok(())
    }

    #[tokio::test]
    async fn test_generate_rule_integration() -> Result<()> {
        let (state, _rx) = LintingState::new();

        let result = state.process_generate_rule(
            "Warn about long functions".to_string(),
            Some("rust".to_string()),
            Some("warning".to_string()),
            None,
            None,
        )
        .await?;

        assert!(result.get("success").and_then(|v| v.as_bool()).unwrap_or(false));

        Ok(())
    }
}
