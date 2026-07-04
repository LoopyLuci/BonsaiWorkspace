/// MCP tool definitions for linting.
/// These tools are registered with the Bonsai MCP server and exposed to AI agents.

use serde::{Deserialize, Serialize};
use serde_json::json;

/// Tool: lint_file - Lint a single file
pub fn tool_lint_file() -> serde_json::Value {
    json!({
        "name": "bonsai_lint_file",
        "description": "Lint a single file and return diagnostics",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to lint (relative to workspace root)"
                },
                "confidence_threshold": {
                    "type": "number",
                    "description": "Minimum confidence score for accepting diagnostics (0.0-1.0)",
                    "default": 0.7
                }
            },
            "required": ["path"]
        }
    })
}

/// Tool: lint_repo - Lint the entire repository
pub fn tool_lint_repo() -> serde_json::Value {
    json!({
        "name": "bonsai_lint_repo",
        "description": "Lint the entire workspace repository",
        "inputSchema": {
            "type": "object",
            "properties": {
                "exclude_patterns": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Glob patterns to exclude from linting"
                },
                "confidence_threshold": {
                    "type": "number",
                    "description": "Minimum confidence score (0.0-1.0)",
                    "default": 0.7
                },
                "ai_filtering": {
                    "type": "boolean",
                    "description": "Enable AI-powered false positive filtering",
                    "default": true
                }
            }
        }
    })
}

/// Tool: generate_lint_rule - Generate a linting rule from a description
pub fn tool_generate_lint_rule() -> serde_json::Value {
    json!({
        "name": "bonsai_generate_lint_rule",
        "description": "Generate a linting rule from a natural language description",
        "inputSchema": {
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "Describe the linting rule you want to create"
                },
                "language": {
                    "type": "string",
                    "description": "Programming language(s) the rule applies to",
                    "default": "rust"
                },
                "severity": {
                    "type": "string",
                    "enum": ["note", "hint", "warning", "error", "fatal"],
                    "description": "Severity level of the rule",
                    "default": "warning"
                },
                "example_good": {
                    "type": "string",
                    "description": "Example of code that passes the rule"
                },
                "example_bad": {
                    "type": "string",
                    "description": "Example of code that violates the rule"
                }
            },
            "required": ["description"]
        }
    })
}

/// Tool: explain_diagnostic - Explain why a diagnostic was generated
pub fn tool_explain_diagnostic() -> serde_json::Value {
    json!({
        "name": "bonsai_explain_diagnostic",
        "description": "Get an AI-generated explanation for a linting diagnostic",
        "inputSchema": {
            "type": "object",
            "properties": {
                "rule_id": {
                    "type": "string",
                    "description": "ID of the linting rule"
                },
                "code_snippet": {
                    "type": "string",
                    "description": "The code snippet that triggered the diagnostic"
                },
                "language": {
                    "type": "string",
                    "description": "Programming language"
                }
            },
            "required": ["rule_id", "code_snippet"]
        }
    })
}

/// Tool result for lint operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintResult {
    pub success: bool,
    pub diagnostics: Vec<DiagnosticInfo>,
    pub summary: LintSummaryInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticInfo {
    pub rule_id: String,
    pub message: String,
    pub severity: String,
    pub file: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintSummaryInfo {
    pub total_diagnostics: usize,
    pub by_severity: std::collections::HashMap<String, usize>,
    pub duration_ms: u128,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definitions_are_valid_json() {
        let tools = vec![
            tool_lint_file(),
            tool_lint_repo(),
            tool_generate_lint_rule(),
            tool_explain_diagnostic(),
        ];

        for tool in tools {
            assert!(tool.get("name").is_some());
            assert!(tool.get("description").is_some());
            assert!(tool.get("inputSchema").is_some());
        }
    }
}
