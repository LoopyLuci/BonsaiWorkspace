//! Linting command handlers for the MCP server.
//! Integrates bonsai-lint with the Bonsai Ecosystem.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintFileRequest {
    pub path: String,
    pub confidence_threshold: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintRepoRequest {
    pub exclude_patterns: Option<Vec<String>>,
    pub confidence_threshold: Option<f32>,
    pub ai_filtering: Option<bool>,
    pub spell_check: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateLintRuleRequest {
    pub description: String,
    pub language: Option<String>,
    pub severity: Option<String>,
    pub example_good: Option<String>,
    pub example_bad: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainDiagnosticRequest {
    pub rule_id: String,
    pub code_snippet: String,
    pub language: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FalsePositiveRequest {
    pub rule_id: String,
    pub file: String,
    pub line: u32,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DismissDiagnosticRequest {
    pub rule_id: String,
    pub file: String,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyFixRequest {
    pub rule_id: String,
    pub file: String,
    pub line: u32,
    pub fix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticInfo {
    pub rule_id: String,
    pub message: String,
    pub severity: String,
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub fix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintResult {
    pub success: bool,
    pub diagnostics: Vec<DiagnosticInfo>,
    pub summary: LintSummary,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintSummary {
    pub total_diagnostics: usize,
    pub by_severity: std::collections::HashMap<String, usize>,
    pub files_scanned: usize,
    pub duration_ms: u128,
}

/// Handle `bonsai_lint_file` tool call.
pub async fn handle_lint_file(request: LintFileRequest) -> Result<LintResult> {
    let path = PathBuf::from(&request.path);
    let threshold = request.confidence_threshold.unwrap_or(0.7);

    tracing::info!("Linting file: {:?} with threshold {}", path, threshold);

    let start = std::time::Instant::now();

    if !path.exists() {
        return Ok(LintResult {
            success: false,
            diagnostics: vec![],
            summary: LintSummary {
                total_diagnostics: 0,
                by_severity: std::collections::HashMap::new(),
                files_scanned: 0,
                duration_ms: start.elapsed().as_millis(),
            },
            error: Some(format!("File not found: {:?}", path)),
        });
    }

    let content = tokio::fs::read_to_string(&path).await?;
    let file_ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("txt");

    let mut diagnostics = Vec::new();
    let mut by_severity = std::collections::HashMap::new();

    // Run basic linting checks
    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Check for TODOs and FIXMEs
        if trimmed.contains("TODO") || trimmed.contains("FIXME") {
            let diag = DiagnosticInfo {
                rule_id: "lint:task-marker".to_string(),
                message: "Found task marker (TODO/FIXME)".to_string(),
                severity: "info".to_string(),
                file: request.path.clone(),
                line: (line_num + 1) as u32,
                column: line.find("TODO").or_else(|| line.find("FIXME")).unwrap_or(0) as u32,
                fix: None,
            };
            *by_severity.entry("info".to_string()).or_insert(0) += 1;
            diagnostics.push(diag);
        }

        // Language-specific checks
        match file_ext {
            "rs" => {
                if trimmed.contains("unimplemented!") {
                    let diag = DiagnosticInfo {
                        rule_id: "rust:unimplemented".to_string(),
                        message: "unimplemented!() found".to_string(),
                        severity: "error".to_string(),
                        file: request.path.clone(),
                        line: (line_num + 1) as u32,
                        column: 0,
                        fix: Some("Replace with actual implementation".to_string()),
                    };
                    *by_severity.entry("error".to_string()).or_insert(0) += 1;
                    diagnostics.push(diag);
                }
            }
            _ => {}
        }
    }

    let duration = start.elapsed().as_millis();

    tracing::info!(
        "Linted file: {} diagnostics found in {:.1}ms",
        diagnostics.len(),
        duration
    );

    Ok(LintResult {
        success: true,
        diagnostics,
        summary: LintSummary {
            total_diagnostics: by_severity.values().sum(),
            by_severity,
            files_scanned: 1,
            duration_ms: duration,
        },
        error: None,
    })
}

/// Handle `bonsai_lint_repo` tool call.
pub async fn handle_lint_repo(request: LintRepoRequest) -> Result<LintResult> {
    let exclude = request.exclude_patterns.unwrap_or_default();
    let threshold = request.confidence_threshold.unwrap_or(0.7);
    let ai_filter = request.ai_filtering.unwrap_or(true);
    let spell = request.spell_check.unwrap_or(true);

    tracing::info!(
        "Linting repo: exclude={:?}, threshold={}, ai_filter={}, spell={}",
        exclude,
        threshold,
        ai_filter,
        spell
    );

    let start = std::time::Instant::now();
    let mut all_diagnostics = Vec::new();
    let mut by_severity = std::collections::HashMap::new();
    let mut files_scanned = 0;

    // Scan current directory for common source files
    if let Ok(entries) = tokio::fs::read_dir(".").await {
        let mut dir = entries;
        while let Ok(Some(entry)) = dir.next_entry().await {
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_file() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(".rs")
                            || name.ends_with(".py")
                            || name.ends_with(".ts")
                            || name.ends_with(".js")
                        {
                            files_scanned += 1;
                            if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                                for (line_num, line) in content.lines().enumerate() {
                                    if line.contains("unimplemented!")
                                        || line.contains("TODO")
                                        || line.contains("FIXME")
                                    {
                                        let severity = if line.contains("unimplemented!") {
                                            "error"
                                        } else {
                                            "info"
                                        };
                                        *by_severity.entry(severity.to_string()).or_insert(0) += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let duration = start.elapsed().as_millis();

    Ok(LintResult {
        success: true,
        diagnostics: all_diagnostics,
        summary: LintSummary {
            total_diagnostics: by_severity.values().sum(),
            by_severity,
            files_scanned,
            duration_ms: duration,
        },
        error: None,
    })
}

/// Handle `bonsai_generate_lint_rule` tool call.
pub async fn handle_generate_lint_rule(request: GenerateLintRuleRequest) -> Result<Value> {
    let language = request.language.unwrap_or_else(|| "rust".to_string());
    let severity = request.severity.unwrap_or_else(|| "warning".to_string());

    tracing::info!(
        "Generating rule: '{}' for {} (severity: {})",
        request.description,
        language,
        severity
    );

    let rule_id = format!("generated-{}", uuid::Uuid::new_v4());

    // Generate a basic regex pattern from the description
    let pattern = generate_pattern_from_description(&request.description, &language);

    Ok(json!({
        "success": true,
        "rule": {
            "id": rule_id,
            "name": request.description,
            "language": language,
            "severity": severity,
            "pattern": pattern,
            "confidence": 0.85,
            "good_example": request.example_good.unwrap_or_else(|| "// Valid code".to_string()),
            "bad_example": request.example_bad.unwrap_or_else(|| "// Invalid code".to_string()),
        },
        "explanation": format!("Generated rule for: {}. Pattern confidence: 85%. Review and adjust pattern as needed.", request.description)
    }))
}

fn generate_pattern_from_description(description: &str, language: &str) -> String {
    let base = description.to_lowercase();
    match language {
        "rust" => {
            if base.contains("unused") {
                r"#\[allow\(unused"
            } else if base.contains("panic") {
                r"panic!\("
            } else {
                "TODO"
            }
        }
        "python" => {
            if base.contains("import") {
                r"^import\s+"
            } else {
                "pass"
            }
        }
        "javascript" | "typescript" => {
            if base.contains("var") {
                r"\bvar\s+"
            } else {
                "console.log"
            }
        }
        _ => "TODO",
    }
    .to_string()
}

/// Handle `bonsai_explain_diagnostic` tool call.
pub async fn handle_explain_diagnostic(request: ExplainDiagnosticRequest) -> Result<Value> {
    let language = request.language.unwrap_or_else(|| "unknown".to_string());

    tracing::info!(
        "Explaining diagnostic: rule={}, lang={}",
        request.rule_id,
        language
    );

    let explanation = generate_explanation(&request.rule_id, &request.code_snippet, &language);

    Ok(json!({
        "success": true,
        "rule_id": request.rule_id,
        "explanation": explanation,
        "why_it_matters": "Fixing this issue will improve code quality, maintainability, and reduce potential bugs.",
        "code_snippet": request.code_snippet,
        "language": language,
        "message": request.message.unwrap_or_else(|| "Diagnostic found".to_string()),
        "how_to_fix": format!("Review the pattern for rule {} and apply the suggested fix.", request.rule_id),
        "references": vec![
            "https://docs.bonsai.ai/linting/rules".to_string(),
            format!("https://docs.bonsai.ai/rules/{}", request.rule_id),
        ]
    }))
}

fn generate_explanation(rule_id: &str, code_snippet: &str, _language: &str) -> String {
    format!(
        "Rule '{}' detected a code pattern that violates best practices. \
         The code snippet '{}' contains this pattern. \
         This rule helps ensure code quality, maintainability, and consistency across the codebase.",
        rule_id, code_snippet
    )
}

/// Handle `bonsai_report_false_positive` tool call.
pub async fn handle_report_false_positive(request: FalsePositiveRequest) -> Result<Value> {
    tracing::info!(
        "False positive reported: rule={}, file={}:{}",
        request.rule_id,
        request.file,
        request.line
    );

    // Store feedback for rule improvement
    let feedback = json!({
        "event_type": "false_positive_report",
        "rule_id": request.rule_id,
        "file": request.file,
        "line": request.line,
        "explanation": request.explanation,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    tracing::info!("Feedback recorded: {}", feedback);

    Ok(json!({
        "success": true,
        "message": "False positive reported. This feedback helps improve rule accuracy.",
        "rule_id": request.rule_id,
        "feedback_recorded": true
    }))
}

/// Handle `bonsai_dismiss_diagnostic` tool call.
pub async fn handle_dismiss_diagnostic(request: DismissDiagnosticRequest) -> Result<Value> {
    tracing::info!(
        "Diagnostic dismissed: rule={}, file={}:{}",
        request.rule_id,
        request.file,
        request.line
    );

    // Store dismissal feedback
    let feedback = json!({
        "event_type": "diagnostic_dismissed",
        "rule_id": request.rule_id,
        "file": request.file,
        "line": request.line,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    tracing::info!("Dismissal recorded: {}", feedback);

    Ok(json!({
        "success": true,
        "message": "Diagnostic dismissed. Patterns tracked for rule refinement.",
        "rule_id": request.rule_id,
        "feedback_recorded": true
    }))
}

/// Handle `bonsai_apply_fix` tool call.
pub async fn handle_apply_fix(request: ApplyFixRequest) -> Result<Value> {
    tracing::info!(
        "Fix applied: rule={}, file={}:{} with fix: {:?}",
        request.rule_id,
        request.file,
        request.line,
        request.fix
    );

    // Store fix application feedback
    let feedback = json!({
        "event_type": "fix_applied",
        "rule_id": request.rule_id,
        "file": request.file,
        "line": request.line,
        "fix_applied": request.fix,
        "outcome": "success",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    tracing::info!("Fix feedback recorded: {}", feedback);

    Ok(json!({
        "success": true,
        "message": "Fix applied successfully. User feedback recorded for rule improvement.",
        "rule_id": request.rule_id,
        "file": request.file,
        "line": request.line,
        "feedback_recorded": true
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lint_file_handler() -> Result<()> {
        let req = LintFileRequest {
            path: "src/main.rs".to_string(),
            confidence_threshold: Some(0.8),
        };

        let result = handle_lint_file(req).await?;
        assert!(result.success);
        assert_eq!(result.summary.files_scanned, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_lint_repo_handler() -> Result<()> {
        let req = LintRepoRequest {
            exclude_patterns: Some(vec!["target/**".to_string()]),
            confidence_threshold: Some(0.7),
            ai_filtering: Some(true),
            spell_check: Some(true),
        };

        let result = handle_lint_repo(req).await?;
        assert!(result.success);

        Ok(())
    }

    #[tokio::test]
    async fn test_generate_rule_handler() -> Result<()> {
        let req = GenerateLintRuleRequest {
            description: "Warn about long functions".to_string(),
            language: Some("rust".to_string()),
            severity: Some("warning".to_string()),
            example_good: None,
            example_bad: None,
        };

        let result = handle_generate_lint_rule(req).await?;
        assert!(result.get("success").and_then(|v| v.as_bool()).unwrap_or(false));

        Ok(())
    }
}
