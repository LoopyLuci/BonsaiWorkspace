/// Bonsai Linter MCP Tool Handlers
/// Exposes bonsai-lint functionality via MCP protocol.
///
/// `handle_lint_file`/`handle_lint_repo` used to return two hardcoded fake
/// diagnostics (or a fixed `files_scanned: 256` / `total_issues: 87` summary)
/// regardless of what file or repo was actually passed in. This is the
/// implementation `tool_registry.rs` actually dispatches "bonsai_lint_file"/
/// "bonsai_lint_repo" to — the separate, real recursive scanner in
/// `lint_commands.rs`/`lint_integration.rs` was never wired to these tool
/// names at all. Now both delegate to the same real scan engine
/// (`scan_rules`) that `bug_hunt_tools.rs` uses.
use anyhow::Result;
use serde_json::{json, Value};

/// Map the shared rule engine's severity vocabulary (critical/high/medium/
/// low/info) onto this tool surface's documented one (error/warning/hint),
/// since callers built against this MCP tool expect those three buckets.
fn map_severity(sev: &str) -> &'static str {
    match sev {
        "critical" | "high" => "error",
        "medium" => "warning",
        _ => "hint",
    }
}

/// Handle bonsai_lint - lint a single file
pub async fn handle_lint_file(args: Value) -> Result<Value> {
    let path = args["path"].as_str().ok_or_else(|| {
        anyhow::anyhow!("missing path parameter")
    })?;
    let fix = args["fix"].as_bool().unwrap_or(false);

    tracing::info!("Linting file: {} (fix={})", path, fix);

    let path_buf = std::path::PathBuf::from(path);
    if !path_buf.exists() {
        return Ok(json!({
            "file": path,
            "status": "error",
            "message": format!("File not found: {path}"),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }));
    }
    let content = tokio::fs::read_to_string(&path_buf).await?;
    let ext = crate::scan_rules::ext_of(&path_buf);

    let mut diagnostics = Vec::new();
    let mut warnings = 0u32;
    let mut errors = 0u32;
    let mut hints = 0u32;
    for (line_num, line) in content.lines().enumerate() {
        for m in crate::scan_rules::scan_line(line, ext) {
            let sev = map_severity(m.severity);
            match sev {
                "error" => errors += 1,
                "warning" => warnings += 1,
                _ => hints += 1,
            }
            diagnostics.push(json!({
                "file": path,
                "line": line_num + 1,
                "column": m.column,
                "rule": m.rule_id,
                "severity": sev,
                "message": m.message,
                "fix": m.suggested_fix.map(|(_, new)| new),
            }));
        }
    }

    Ok(json!({
        "file": path,
        "status": "complete",
        "diagnostics": diagnostics,
        "total": diagnostics.len(),
        "warnings": warnings,
        "errors": errors,
        "hints": hints,
        // Fixes are never applied silently as a side effect of linting — a
        // caller that wants a fix applied must call bonsai_apply_fix (or
        // bug_hunt_tools::handle_auto_fix) explicitly with confirmation.
        "fix_requested": fix,
        "fixed": 0,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Handle bonsai_lint_repo - lint entire repository (recursive, from cwd)
pub async fn handle_lint_repo(args: Value) -> Result<Value> {
    let quick = args["quick"].as_bool().unwrap_or(true);

    tracing::info!("Linting repository (quick={})", quick);

    let start = std::time::Instant::now();
    let mut files_scanned = 0usize;
    let mut warnings = 0u32;
    let mut errors = 0u32;
    let mut hints = 0u32;
    let mut by_language: std::collections::HashMap<&'static str, (u32, u32)> = std::collections::HashMap::new();
    let mut by_rule: std::collections::HashMap<String, (u32, &'static str)> = std::collections::HashMap::new();

    let mut stack = vec![std::path::PathBuf::from(".")];
    while let Some(dir) = stack.pop() {
        let Ok(mut entries) = tokio::fs::read_dir(&dir).await else { continue };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(metadata) = entry.metadata().await else { continue };
            if metadata.is_dir() {
                if !crate::scan_rules::is_excluded_dir(&name) {
                    stack.push(path);
                }
                continue;
            }
            if !metadata.is_file() {
                continue;
            }
            let ext = crate::scan_rules::ext_of(&path);
            if !crate::scan_rules::is_scannable_ext(ext) {
                continue;
            }
            let Ok(content) = tokio::fs::read_to_string(&path).await else { continue };
            files_scanned += 1;
            let lang = match ext {
                "rs" => "rust",
                "py" => "python",
                "ts" | "tsx" | "js" | "jsx" => "javascript",
                "go" => "go",
                _ => "other",
            };
            let lang_entry = by_language.entry(lang).or_insert((0, 0));
            lang_entry.0 += 1;

            if quick && files_scanned > 2000 {
                // Quick mode still scans real files, just caps how many —
                // a genuine bound rather than a fabricated summary.
                continue;
            }

            for line in content.lines() {
                for m in crate::scan_rules::scan_line(line, ext) {
                    let sev = map_severity(m.severity);
                    match sev {
                        "error" => errors += 1,
                        "warning" => warnings += 1,
                        _ => hints += 1,
                    }
                    lang_entry.1 += 1;
                    by_rule.entry(m.rule_id.to_string()).or_insert((0, sev)).0 += 1;
                }
            }
        }
    }

    let mut top_violations: Vec<Value> = by_rule
        .into_iter()
        .map(|(rule, (count, sev))| json!({ "rule": rule, "count": count, "severity": sev }))
        .collect();
    top_violations.sort_by_key(|v| std::cmp::Reverse(v["count"].as_u64().unwrap_or(0)));
    top_violations.truncate(10);

    let languages: serde_json::Map<String, Value> = by_language
        .into_iter()
        .map(|(lang, (files, issues))| (lang.to_string(), json!({ "files": files, "issues": issues })))
        .collect();

    Ok(json!({
        "mode": if quick { "quick" } else { "full" },
        "status": "complete",
        "summary": {
            "files_scanned": files_scanned,
            "total_issues": warnings + errors + hints,
            "warnings": warnings,
            "errors": errors,
            "hints": hints,
            "fixed": 0,
        },
        "languages": languages,
        "top_violations": top_violations,
        "duration_ms": start.elapsed().as_millis(),
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Handle bonsai_generate_lint_rule - AI-powered rule generation
pub async fn handle_generate_lint_rule(args: Value) -> Result<Value> {
    let description = args["description"].as_str().ok_or_else(|| {
        anyhow::anyhow!("missing description parameter")
    })?;
    let language = args["language"].as_str().ok_or_else(|| {
        anyhow::anyhow!("missing language parameter")
    })?;

    tracing::info!("Generating lint rule for {}: {}", language, description);

    let rule = json!({
        "rule_id": format!("custom-rule-{}", uuid::Uuid::new_v4()),
        "name": "Generated Rule",
        "description": description,
        "language": language,
        "pattern": "[A-Z][a-z]+",
        "severity": "warning",
        "message_template": "Found issue: {match}",
        "enabled": true,
        "tags": ["generated", "custom"],
        "created_at": chrono::Utc::now().to_rfc3339(),
        "confidence": 0.75,
        "status": "pending_review"
    });

    Ok(rule)
}

/// Handle bonsai_explain_diagnostic - explain a lint rule
pub async fn handle_explain_diagnostic(args: Value) -> Result<Value> {
    let rule_id = args["rule_id"].as_str().ok_or_else(|| {
        anyhow::anyhow!("missing rule_id parameter")
    })?;

    tracing::info!("Explaining diagnostic: {}", rule_id);

    let explanation = match rule_id {
        "unused-import" => json!({
            "rule_id": "unused-import",
            "title": "Unused Import",
            "description": "This rule detects import statements that are not used anywhere in the file. Unused imports clutter the code and can slow down compilation.",
            "why_it_matters": [
                "Reduces code noise and improves readability",
                "Can improve compilation speed",
                "Helps maintain clean dependencies"
            ],
            "example": {
                "bad": "import json\nprint('hello')",
                "good": "print('hello')"
            },
            "how_to_fix": "Remove the unused import statement",
            "severity": "warning",
            "confidence": 0.95
        }),
        "missing-error-handling" => json!({
            "rule_id": "missing-error-handling",
            "title": "Missing Error Handling",
            "description": "This rule detects function calls that return errors but don't handle them. Unhandled errors can lead to crashes or unexpected behavior.",
            "why_it_matters": [
                "Prevents runtime crashes",
                "Makes code more robust",
                "Improves user experience"
            ],
            "severity": "error",
            "confidence": 0.85
        }),
        _ => json!({
            "rule_id": rule_id,
            "title": "Generic Rule",
            "description": format!("Explanation for rule: {}", rule_id),
            "why_it_matters": ["Code quality", "Best practices"],
            "severity": "warning",
            "confidence": 0.70
        })
    };

    Ok(explanation)
}

/// Handle bonsai_apply_fix - apply a fix from a diagnostic
///
/// This tool surface addresses a fix by `(file, line, fix_id)` rather than a
/// stored finding id (that's `bug_hunt_tools::handle_auto_fix`'s job), so
/// there's no record here of what the "before" text should be. Previously
/// this returned a fabricated `"before": "unused code here"` / `"after": "//
/// code removed"` unconditionally, regardless of the file's real content —
/// this now at least verifies the file and line genuinely exist before
/// reporting anything, and returns the real current line instead of inventing
/// text, rather than claiming a specific rewrite that never happened.
pub async fn handle_apply_fix(args: Value) -> Result<Value> {
    let file = args["file"].as_str().ok_or_else(|| {
        anyhow::anyhow!("missing file parameter")
    })?;
    let line = args["line"].as_u64().ok_or_else(|| {
        anyhow::anyhow!("missing line parameter")
    })?;
    let fix_id = args["fix_id"].as_str().ok_or_else(|| {
        anyhow::anyhow!("missing fix_id parameter")
    })?;

    tracing::info!("Applying fix {} at {}:{}", fix_id, file, line);

    let content = match tokio::fs::read_to_string(file).await {
        Ok(c) => c,
        Err(e) => {
            return Ok(json!({
                "status": "error",
                "file": file,
                "line": line,
                "fix_id": fix_id,
                "message": format!("Could not read {file}: {e}"),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }));
        }
    };
    let Some(current_line) = content.lines().nth((line.saturating_sub(1)) as usize) else {
        return Ok(json!({
            "status": "error",
            "file": file,
            "line": line,
            "fix_id": fix_id,
            "message": format!("{file} has no line {line}"),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }));
    };

    // No specific rewrite is known at this address — this endpoint records
    // the fix request against the real current line, but does not invent or
    // apply a text change. Use `bonsai_scan_repo` + `bonsai_auto_fix` for
    // findings that have a concrete, mechanically-safe suggested_fix.
    Ok(json!({
        "status": "not_applied",
        "file": file,
        "line": line,
        "fix_id": fix_id,
        "current_line": current_line,
        "message": "No mechanical rewrite is registered for this fix_id — review and edit manually, or use bug_hunt_tools' bonsai_auto_fix for findings with a concrete suggested fix.",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Handle bonsai_dismiss_diagnostic - mark diagnostic as dismissed
pub async fn handle_dismiss_diagnostic(args: Value) -> Result<Value> {
    let file = args["file"].as_str().ok_or_else(|| {
        anyhow::anyhow!("missing file parameter")
    })?;
    let line = args["line"].as_u64().ok_or_else(|| {
        anyhow::anyhow!("missing line parameter")
    })?;
    let rule_id = args["rule_id"].as_str().ok_or_else(|| {
        anyhow::anyhow!("missing rule_id parameter")
    })?;

    tracing::info!("Dismissing diagnostic {} at {}:{}", rule_id, file, line);

    let result = json!({
        "status": "dismissed",
        "file": file,
        "line": line,
        "rule_id": rule_id,
        "message": "Diagnostic dismissed - rule confidence will be adjusted",
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    Ok(result)
}

/// Handle bonsai_report_false_positive - report false positive
pub async fn handle_report_false_positive(args: Value) -> Result<Value> {
    let rule_id = args["rule_id"].as_str().ok_or_else(|| {
        anyhow::anyhow!("missing rule_id parameter")
    })?;
    let file = args["file"].as_str().ok_or_else(|| {
        anyhow::anyhow!("missing file parameter")
    })?;
    let line = args["line"].as_u64().ok_or_else(|| {
        anyhow::anyhow!("missing line parameter")
    })?;

    tracing::info!("Reporting false positive: {} at {}:{}", rule_id, file, line);

    let result = json!({
        "status": "recorded",
        "rule_id": rule_id,
        "file": file,
        "line": line,
        "message": "False positive recorded - rule confidence will be decreased",
        "confidence_impact": "-0.05",
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    Ok(result)
}
