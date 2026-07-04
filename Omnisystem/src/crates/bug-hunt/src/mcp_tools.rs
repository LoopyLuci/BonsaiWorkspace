/// MCP (Model Context Protocol) server integration for Bug Hunt.
/// Exposes bug hunt functionality as callable MCP tools for AI agents.

use serde_json::{json, Value};
use std::path::PathBuf;

/// Schema for the scan_repo MCP tool.
pub fn scan_repo_schema() -> Value {
    json!({
        "name": "bonsai_scan_repo",
        "description": "Scan a repository for bugs, vulnerabilities, and issues using Bonsai Bug Hunt",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the repository (local path or remote git URL)"
                },
                "mode": {
                    "type": "string",
                    "enum": ["quick", "full"],
                    "description": "Scan mode: quick (static only) or full (with AI review)"
                },
                "ai_review": {
                    "type": "boolean",
                    "description": "Enable AI code review using BonsAI V2"
                },
                "output_format": {
                    "type": "string",
                    "enum": ["json", "sarif", "html", "markdown"],
                    "description": "Output format for the report"
                }
            },
            "required": ["path"]
        }
    })
}

/// Schema for the list_findings MCP tool.
pub fn list_findings_schema() -> Value {
    json!({
        "name": "bonsai_list_findings",
        "description": "List findings from the last scan",
        "inputSchema": {
            "type": "object",
            "properties": {
                "severity": {
                    "type": "string",
                    "enum": ["critical", "high", "medium", "low", "info"],
                    "description": "Filter by severity level"
                },
                "file_pattern": {
                    "type": "string",
                    "description": "Filter by file pattern (glob)"
                }
            }
        }
    })
}

/// Schema for the fix_issue MCP tool.
pub fn fix_issue_schema() -> Value {
    json!({
        "name": "bonsai_fix_issue",
        "description": "Attempt to automatically fix an issue",
        "inputSchema": {
            "type": "object",
            "properties": {
                "issue_id": {
                    "type": "string",
                    "description": "UUID of the issue to fix"
                },
                "confirm": {
                    "type": "boolean",
                    "description": "Apply the fix without further confirmation"
                }
            },
            "required": ["issue_id"]
        }
    })
}

/// Get all MCP tool schemas.
pub fn all_schemas() -> Vec<Value> {
    vec![
        scan_repo_schema(),
        list_findings_schema(),
        fix_issue_schema(),
    ]
}
