/// Tool Registry - All MCP tools available to agents
/// Registers Bug Hunter, Linter, and Bonsai Ecosystem tools

use serde_json::{json, Value};
use std::collections::HashMap;
use crate::{bug_hunt_tools, lint_tools};

#[derive(Debug, Clone)]
pub struct McpToolRegistry {
    tools: HashMap<String, ToolDefinition>,
}

#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

impl McpToolRegistry {
    pub fn new() -> Self {
        let mut tools = HashMap::new();

        // ===================================================================
        // BUG HUNTER TOOLS
        // ===================================================================
        tools.insert("bonsai_scan_repo".to_string(), ToolDefinition {
            name: "bonsai_scan_repo".to_string(),
            description: "Scan repository for bugs, vulnerabilities, and code quality issues".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Repository path to scan"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["quick", "full", "ai"],
                        "description": "Scan mode: quick (fast), full (comprehensive), ai (with AI analysis)"
                    },
                    "ai_review": {
                        "type": "boolean",
                        "description": "Enable AI-powered analysis"
                    },
                    "output_format": {
                        "type": "string",
                        "enum": ["json", "markdown", "sarif"],
                        "description": "Output format"
                    }
                },
                "required": ["path"]
            }),
        });

        tools.insert("bonsai_list_findings".to_string(), ToolDefinition {
            name: "bonsai_list_findings".to_string(),
            description: "List findings from a scan, optionally filtered by severity".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "scan_id": {
                        "type": "string",
                        "description": "Scan ID from bonsai_scan_repo"
                    },
                    "severity": {
                        "type": "string",
                        "enum": ["critical", "high", "medium", "low", "info"],
                        "description": "Filter by severity level"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of findings to return"
                    }
                },
                "required": ["scan_id"]
            }),
        });

        tools.insert("bonsai_get_finding".to_string(), ToolDefinition {
            name: "bonsai_get_finding".to_string(),
            description: "Get detailed information about a specific finding".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "finding_id": {
                        "type": "string",
                        "description": "Finding ID"
                    }
                },
                "required": ["finding_id"]
            }),
        });

        tools.insert("bonsai_auto_fix".to_string(), ToolDefinition {
            name: "bonsai_auto_fix".to_string(),
            description: "Apply automatic fix for a finding".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "finding_id": {
                        "type": "string",
                        "description": "Finding ID"
                    },
                    "confirm": {
                        "type": "boolean",
                        "description": "Confirm the fix application"
                    }
                },
                "required": ["finding_id"]
            }),
        });

        tools.insert("bonsai_explain_diagnostic".to_string(), ToolDefinition {
            name: "bonsai_explain_diagnostic".to_string(),
            description: "Get AI explanation of a finding".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "finding_id": {
                        "type": "string",
                        "description": "Finding ID"
                    }
                },
                "required": ["finding_id"]
            }),
        });

        tools.insert("bonsai_prioritize_findings".to_string(), ToolDefinition {
            name: "bonsai_prioritize_findings".to_string(),
            description: "Prioritize findings by impact or effort".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "scan_id": {
                        "type": "string",
                        "description": "Scan ID"
                    },
                    "strategy": {
                        "type": "string",
                        "enum": ["impact", "effort", "security", "maintainability"],
                        "description": "Prioritization strategy"
                    }
                },
                "required": ["scan_id"]
            }),
        });

        tools.insert("bonsai_generate_report".to_string(), ToolDefinition {
            name: "bonsai_generate_report".to_string(),
            description: "Generate a comprehensive scan report".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "scan_id": {
                        "type": "string",
                        "description": "Scan ID"
                    },
                    "format": {
                        "type": "string",
                        "enum": ["json", "markdown", "html", "pdf"],
                        "description": "Report format"
                    }
                },
                "required": ["scan_id"]
            }),
        });

        // ===================================================================
        // LINTER TOOLS
        // ===================================================================
        tools.insert("bonsai_lint_file".to_string(), ToolDefinition {
            name: "bonsai_lint_file".to_string(),
            description: "Lint a single file".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path"
                    },
                    "languages": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Filter by languages"
                    },
                    "rules": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Filter by rules"
                    },
                    "fix": {
                        "type": "boolean",
                        "description": "Apply automatic fixes"
                    }
                },
                "required": ["path"]
            }),
        });

        tools.insert("bonsai_lint_repo".to_string(), ToolDefinition {
            name: "bonsai_lint_repo".to_string(),
            description: "Lint entire repository".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "quick": {
                        "type": "boolean",
                        "description": "Quick mode (faster but less thorough)"
                    }
                }
            }),
        });

        tools.insert("bonsai_generate_lint_rule".to_string(), ToolDefinition {
            name: "bonsai_generate_lint_rule".to_string(),
            description: "Generate a lint rule from description".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "description": {
                        "type": "string",
                        "description": "Rule description"
                    },
                    "language": {
                        "type": "string",
                        "description": "Target language"
                    }
                },
                "required": ["description", "language"]
            }),
        });

        tools.insert("bonsai_explain_diagnostic".to_string(), ToolDefinition {
            name: "bonsai_explain_diagnostic".to_string(),
            description: "Explain a lint rule or diagnostic".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "rule_id": {
                        "type": "string",
                        "description": "Rule ID"
                    }
                },
                "required": ["rule_id"]
            }),
        });

        tools.insert("bonsai_apply_fix".to_string(), ToolDefinition {
            name: "bonsai_apply_fix".to_string(),
            description: "Apply a fix from a diagnostic".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string" },
                    "line": { "type": "integer" },
                    "fix_id": { "type": "string" }
                },
                "required": ["file", "line", "fix_id"]
            }),
        });

        tools.insert("bonsai_dismiss_diagnostic".to_string(), ToolDefinition {
            name: "bonsai_dismiss_diagnostic".to_string(),
            description: "Dismiss a diagnostic".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string" },
                    "line": { "type": "integer" },
                    "rule_id": { "type": "string" }
                },
                "required": ["file", "line", "rule_id"]
            }),
        });

        tools.insert("bonsai_report_false_positive".to_string(), ToolDefinition {
            name: "bonsai_report_false_positive".to_string(),
            description: "Report a false positive".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "rule_id": { "type": "string" },
                    "file": { "type": "string" },
                    "line": { "type": "integer" }
                },
                "required": ["rule_id", "file", "line"]
            }),
        });

        Self { tools }
    }

    pub fn list_tools(&self) -> Vec<ToolDefinition> {
        self.tools.values().cloned().collect()
    }

    pub fn get_tool(&self, name: &str) -> Option<ToolDefinition> {
        self.tools.get(name).cloned()
    }

    pub async fn execute_tool(&self, name: &str, args: Value) -> Result<Value, String> {
        match name {
            // Bug Hunt tools
            "bonsai_scan_repo" => bug_hunt_tools::handle_scan_repo(args).await.map_err(|e| e.to_string()),
            "bonsai_list_findings" => bug_hunt_tools::handle_list_findings(args).await.map_err(|e| e.to_string()),
            "bonsai_get_finding" => bug_hunt_tools::handle_get_finding(args).await.map_err(|e| e.to_string()),
            "bonsai_auto_fix" => bug_hunt_tools::handle_auto_fix(args).await.map_err(|e| e.to_string()),
            "bonsai_explain_diagnostic" if args.get("finding_id").is_some() => {
                bug_hunt_tools::handle_explain_diagnostic(args).await.map_err(|e| e.to_string())
            },
            "bonsai_prioritize_findings" => bug_hunt_tools::handle_prioritize_findings(args).await.map_err(|e| e.to_string()),
            "bonsai_generate_report" => bug_hunt_tools::handle_generate_report(args).await.map_err(|e| e.to_string()),

            // Linter tools
            "bonsai_lint_file" => lint_tools::handle_lint_file(args).await.map_err(|e| e.to_string()),
            "bonsai_lint_repo" => lint_tools::handle_lint_repo(args).await.map_err(|e| e.to_string()),
            "bonsai_generate_lint_rule" => lint_tools::handle_generate_lint_rule(args).await.map_err(|e| e.to_string()),
            "bonsai_explain_diagnostic" if args.get("rule_id").is_some() => {
                lint_tools::handle_explain_diagnostic(args).await.map_err(|e| e.to_string())
            },
            "bonsai_apply_fix" => lint_tools::handle_apply_fix(args).await.map_err(|e| e.to_string()),
            "bonsai_dismiss_diagnostic" => lint_tools::handle_dismiss_diagnostic(args).await.map_err(|e| e.to_string()),
            "bonsai_report_false_positive" => lint_tools::handle_report_false_positive(args).await.map_err(|e| e.to_string()),

            _ => Err(format!("Unknown tool: {}", name)),
        }
    }
}

impl Default for McpToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
