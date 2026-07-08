//! Shared static-analysis rule engine used by both `lint_tools` (the
//! `bonsai_lint_file`/`bonsai_lint_repo` MCP tools) and `bug_hunt_tools` (the
//! `bonsai_scan_repo` family). Previously each had its own copy-pasted,
//! partial, or entirely mocked rule set. A single rule set here means a fix
//! or a new rule benefits both tool surfaces at once.

use std::path::Path;

#[derive(Debug, Clone)]
pub struct RuleMatch {
    pub rule_id: &'static str,
    pub severity: &'static str, // critical | high | medium | low | info
    pub category: &'static str,
    pub message: String,
    pub column: u32,
    pub fixable: bool,
    /// If mechanically fixable, the exact (old, new) text to replace — only
    /// applied by `bug_hunt_tools::handle_auto_fix` when `old` appears
    /// exactly once in the file, so a fix never guesses at ambiguous matches.
    pub suggested_fix: Option<(String, String)>,
}

/// Scan a single line for known problem patterns. `ext` is the file
/// extension (without the dot) used to gate language-specific checks.
pub fn scan_line(line: &str, ext: &str) -> Vec<RuleMatch> {
    let mut out = Vec::new();
    let trimmed = line.trim();

    if trimmed.is_empty() || trimmed.starts_with("//!") {
        return out;
    }

    // ── Universal: task markers ────────────────────────────────────────
    if let Some(col) = line.find("TODO").or_else(|| line.find("FIXME")) {
        out.push(RuleMatch {
            rule_id: "lint:task-marker",
            severity: "info",
            category: "maintenance",
            message: "Found task marker (TODO/FIXME)".to_string(),
            column: col as u32,
            fixable: false,
            suggested_fix: None,
        });
    }

    // ── Universal: placeholder/stub implementations ────────────────────
    // Directly targets the failure mode this whole codebase has hit
    // repeatedly: a function whose doc-comment or body admits it's fake
    // ("simulate", "mock", "placeholder", "would call", "not yet
    // implemented") while still returning a normal-looking success value.
    const STUB_MARKERS: &[&str] = &[
        "would call", "simulate", "simulated", "not yet implemented",
        "placeholder — real", "placeholder - real", "phase 1: stub",
        "phase 2: stub", "todo: implement", "not actually",
    ];
    let lower = trimmed.to_lowercase();
    if let Some(marker) = STUB_MARKERS.iter().find(|m| lower.contains(*m)) {
        out.push(RuleMatch {
            rule_id: "lint:stub-implementation",
            severity: "high",
            category: "incomplete",
            message: format!("Comment admits this is a stub/placeholder (matched \"{marker}\")"),
            column: 0,
            fixable: false,
            suggested_fix: None,
        });
    }

    // ── Universal: hardcoded secrets ────────────────────────────────────
    for key in ["api_key", "apikey", "password", "secret", "access_token"] {
        if let Some(idx) = lower.find(key) {
            let after = &trimmed[idx.min(trimmed.len())..];
            if let Some(eq) = after.find('=') {
                let value_part = after[eq + 1..].trim_start();
                let looks_like_literal = value_part.starts_with('"') || value_part.starts_with('\'');
                let is_placeholder = ["\"\"", "\"changeme\"", "\"xxx\"", "\"todo\"", "none", "option"]
                    .iter()
                    .any(|p| value_part.to_lowercase().starts_with(p));
                let is_read_from_env = lower.contains("env::var") || lower.contains("std::env") || lower.contains("getenv");
                if looks_like_literal && !is_placeholder && !is_read_from_env && value_part.len() > 3 {
                    out.push(RuleMatch {
                        rule_id: "security:hardcoded-secret",
                        severity: "critical",
                        category: "security",
                        message: format!("Possible hardcoded secret (matched key \"{key}\")"),
                        column: idx as u32,
                        fixable: false,
                        suggested_fix: None,
                    });
                    break;
                }
            }
        }
    }

    // ── Language-specific ───────────────────────────────────────────────
    match ext {
        "rs" => {
            if let Some(col) = trimmed.find("unimplemented!") {
                out.push(RuleMatch {
                    rule_id: "rust:unimplemented",
                    severity: "critical",
                    category: "incomplete",
                    message: "unimplemented!() will panic at runtime if this path executes".to_string(),
                    column: col as u32,
                    fixable: false,
                    suggested_fix: None,
                });
            }
            if let Some(col) = trimmed.find("todo!(") {
                out.push(RuleMatch {
                    rule_id: "rust:todo-macro",
                    severity: "high",
                    category: "incomplete",
                    message: "todo!() will panic at runtime if this path executes".to_string(),
                    column: col as u32,
                    fixable: false,
                    suggested_fix: None,
                });
            }
            if trimmed.contains("dbg!(") {
                out.push(RuleMatch {
                    rule_id: "rust:debug-macro",
                    severity: "low",
                    category: "cleanliness",
                    message: "dbg!() left in code — prints to stderr and is usually debug-only".to_string(),
                    column: 0,
                    fixable: false,
                    suggested_fix: None,
                });
            }
        }
        "ts" | "tsx" | "js" | "jsx" => {
            if trimmed.starts_with("console.log(") || trimmed.contains(" console.log(") {
                out.push(RuleMatch {
                    rule_id: "js:console-log",
                    severity: "low",
                    category: "cleanliness",
                    message: "console.log left in code".to_string(),
                    column: 0,
                    fixable: false,
                    suggested_fix: None,
                });
            }
            // A genuinely safe, mechanical rewrite: `var` is function-scoped
            // and long superseded by `let`/`const`. Uses the *entire original
            // line* as the fix's `old` text (not just the `var ` substring),
            // since `handle_auto_fix` only applies a fix when `old` matches
            // exactly once in the file — matching on the full line is far
            // more likely to be unique than matching on `var ` alone, which
            // could appear many times across a real file.
            if trimmed.starts_with("var ") {
                let fixed_line = line.replacen("var ", "let ", 1);
                out.push(RuleMatch {
                    rule_id: "js:var-keyword",
                    severity: "low",
                    category: "modernization",
                    message: "`var` is function-scoped; prefer `let` or `const`".to_string(),
                    column: 0,
                    fixable: true,
                    suggested_fix: Some((line.to_string(), fixed_line)),
                });
            }
        }
        "py" => {
            if trimmed == "pass" {
                out.push(RuleMatch {
                    rule_id: "python:bare-pass",
                    severity: "info",
                    category: "incomplete",
                    message: "Bare `pass` — possibly an unimplemented function body".to_string(),
                    column: 0,
                    fixable: false,
                    suggested_fix: None,
                });
            }
        }
        _ => {}
    }

    out
}

/// Directories never worth descending into for a source scan.
pub fn is_excluded_dir(name: &str) -> bool {
    matches!(
        name,
        "target" | "node_modules" | ".git" | "dist" | "build" | ".venv" | "venv" | "__pycache__" | ".next" | "vendor"
    )
}

/// Extensions this scanner understands. Anything else is skipped so binary
/// files, images, etc. are never read as text.
pub fn is_scannable_ext(ext: &str) -> bool {
    matches!(ext, "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go")
}

pub fn ext_of(path: &Path) -> &str {
    path.extension().and_then(|e| e.to_str()).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_todo_and_fixme() {
        assert_eq!(scan_line("// TODO: fix this", "rs")[0].rule_id, "lint:task-marker");
        assert_eq!(scan_line("# FIXME later", "py")[0].rule_id, "lint:task-marker");
        assert!(scan_line("let x = 1;", "rs").is_empty());
    }

    #[test]
    fn detects_rust_unimplemented_and_todo_macro() {
        let m = scan_line("fn f() { unimplemented!() }", "rs");
        assert!(m.iter().any(|r| r.rule_id == "rust:unimplemented" && r.severity == "critical"));

        let m = scan_line("fn f() { todo!(\"later\") }", "rs");
        assert!(m.iter().any(|r| r.rule_id == "rust:todo-macro" && r.severity == "high"));
    }

    #[test]
    fn ignores_unimplemented_in_other_languages() {
        // The rust-specific rule must not fire on non-.rs files even if the
        // literal text appears (e.g. inside a string or comment).
        let m = scan_line("// unimplemented!() in the rust version", "py");
        assert!(!m.iter().any(|r| r.rule_id == "rust:unimplemented"));
    }

    #[test]
    fn detects_stub_implementation_markers() {
        let cases = [
            "// Simulate scan (would call actual orchestrator)",
            "// Phase 1: stub — real implementation calls bonsai-crdt directly",
            "// not yet implemented",
        ];
        for line in cases {
            let m = scan_line(line, "rs");
            assert!(
                m.iter().any(|r| r.rule_id == "lint:stub-implementation"),
                "expected stub marker on: {line}"
            );
        }
    }

    #[test]
    fn detects_hardcoded_secret_but_not_env_lookup_or_placeholder() {
        let bad = scan_line(r#"let api_key = "sk-live-abcdef123456";"#, "rs");
        assert!(bad.iter().any(|r| r.rule_id == "security:hardcoded-secret"));

        let via_env = scan_line(r#"let api_key = std::env::var("API_KEY").unwrap();"#, "rs");
        assert!(!via_env.iter().any(|r| r.rule_id == "security:hardcoded-secret"));

        let placeholder = scan_line(r#"let password = "";"#, "rs");
        assert!(!placeholder.iter().any(|r| r.rule_id == "security:hardcoded-secret"));
    }

    #[test]
    fn var_keyword_produces_a_unique_full_line_fix() {
        let line = "    var count = 0;";
        let m = scan_line(line, "js");
        let hit = m.iter().find(|r| r.rule_id == "js:var-keyword").expect("expected js:var-keyword match");
        assert!(hit.fixable);
        let (old, new) = hit.suggested_fix.clone().expect("expected a suggested_fix");
        assert_eq!(old, line);
        assert_eq!(new, "    let count = 0;");
    }

    #[test]
    fn excluded_dirs_and_scannable_ext() {
        assert!(is_excluded_dir("target"));
        assert!(is_excluded_dir("node_modules"));
        assert!(!is_excluded_dir("src"));

        assert!(is_scannable_ext("rs"));
        assert!(is_scannable_ext("py"));
        assert!(!is_scannable_ext("png"));
        assert!(!is_scannable_ext("gguf"));
    }

    #[test]
    fn ext_of_extracts_extension() {
        assert_eq!(ext_of(Path::new("src/main.rs")), "rs");
        assert_eq!(ext_of(Path::new("README")), "");
    }
}
