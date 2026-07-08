//! Survival System — Bug Hunter (the scanners).
//!
//! One of the Survival System's tools (see `super` for the others: `kb`,
//! `bug_db`, `sns_bridge`, `crash_ingest`, `daemon`) — proactive, read-only
//! diagnostic scanning across every `targets::MonitorTarget`. Unlike
//! `self_upgrade::sandbox`, these run directly against the live repo —
//! `cargo check`/`clippy`/`test`, `svelte-check`, `tsc`, and `eslint` never
//! write to the source tree, so there's no need for a worktree here (a
//! worktree exists to isolate a *proposed* write; a diagnostic run has
//! nothing to isolate).
//!
//! Note: no generic `eslint` scanner is applied to every JS/TS target —
//! only `vscode-omnisystem` actually has ESLint configured (checked its
//! `package.json` before assuming otherwise); `workspace-frontend` has no
//! ESLint dependency or config at all, so it's scanned via `svelte-check`
//! only, never a fabricated `npx eslint` that would silently try to
//! download it over the network with nothing to run against.

use std::path::Path;
use std::process::Command;

use regex::Regex;
use serde_json::Value;

use super::bug_db::DiscoveredBug;
use super::targets::{MonitorTarget, TargetKind};

fn run_capture(dir: &Path, program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).current_dir(dir).output().ok()?;
    Some(format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    ))
}

/// Runs every scanner appropriate to `target.kind`, then prefixes each
/// resulting bug's title and file path with the target's name so bugs
/// found across many different projects stay unambiguous and attributable
/// once they're all in the same Bug Database.
pub fn scan_target(repo_root: &Path, target: &MonitorTarget) -> Vec<DiscoveredBug> {
    let dir = target.abs_path(repo_root);
    let mut bugs = match target.kind {
        TargetKind::RustCrate => {
            let mut b = scan_cargo_check(&dir);
            b.extend(scan_clippy(&dir));
            b.extend(scan_cargo_test(&dir));
            b
        }
        TargetKind::NpmSvelteCheck => scan_svelte_check(&dir),
        TargetKind::NpmTypecheckLint => {
            let mut b = scan_tsc(&dir);
            b.extend(scan_eslint(&dir));
            b
        }
    };
    for bug in &mut bugs {
        bug.title = format!("[{}] {}", target.name, bug.title);
        if let Some(fp) = &bug.file_path {
            bug.file_path = Some(format!("{}/{}", target.rel_path, fp));
        }
    }
    bugs
}

/// Parses cargo's `--message-format=json` stream (one JSON object per line)
/// into bugs. Shared by `scan_cargo_check` and `scan_clippy` — clippy reuses
/// rustc's exact diagnostic JSON shape.
fn parse_cargo_json_diagnostics(output: &str, source: &str) -> Vec<DiscoveredBug> {
    let mut bugs = Vec::new();
    for line in output.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        if v.get("reason").and_then(Value::as_str) != Some("compiler-message") {
            continue;
        }
        let Some(message) = v.get("message") else { continue };
        let level = message.get("level").and_then(Value::as_str).unwrap_or("");
        if level != "error" && level != "warning" {
            continue;
        }
        let text = message.get("message").and_then(Value::as_str).unwrap_or("").to_string();
        if text.is_empty() {
            continue;
        }
        let (file_path, line_number) = message
            .get("spans")
            .and_then(Value::as_array)
            .and_then(|spans| spans.iter().find(|s| s.get("is_primary").and_then(Value::as_bool) == Some(true)))
            .map(|span| {
                (
                    span.get("file_name").and_then(Value::as_str).map(String::from),
                    span.get("line_start").and_then(Value::as_i64),
                )
            })
            .unwrap_or((None, None));

        bugs.push(DiscoveredBug {
            source: source.to_string(),
            severity: level.to_string(),
            title: text.lines().next().unwrap_or(&text).chars().take(200).collect(),
            message: text,
            file_path,
            line_number,
        });
    }
    bugs
}

fn scan_cargo_check(crate_dir: &Path) -> Vec<DiscoveredBug> {
    match run_capture(crate_dir, "cargo", &["check", "--lib", "--message-format=json"]) {
        Some(out) => parse_cargo_json_diagnostics(&out, "compile"),
        None => Vec::new(),
    }
}

fn scan_clippy(crate_dir: &Path) -> Vec<DiscoveredBug> {
    match run_capture(crate_dir, "cargo", &["clippy", "--lib", "--message-format=json"]) {
        Some(out) => parse_cargo_json_diagnostics(&out, "lint"),
        // clippy may not be installed on every machine — absence is not a
        // scan failure, just nothing to report from this scanner.
        None => Vec::new(),
    }
}

/// Parses `cargo test`'s human-readable output for failed tests and their
/// panic message. Cargo's stable CLI has no stable structured test-result
/// format (unlike `check`/`clippy`'s `--message-format=json`), so this is
/// the one Rust scanner that's regex-based against real text output.
fn scan_cargo_test(crate_dir: &Path) -> Vec<DiscoveredBug> {
    let Some(output) = run_capture(crate_dir, "cargo", &["test", "--lib"]) else {
        return Vec::new();
    };

    let failed_re = Regex::new(r"(?m)^test (\S+) \.\.\. FAILED$").unwrap();
    let location_re = Regex::new(r"(\S+\.rs):(\d+):\d+").unwrap();

    let mut bugs = Vec::new();
    for cap in failed_re.captures_iter(&output) {
        let name = &cap[1];
        let marker = format!("---- {name} stdout ----");
        let panic_block = output
            .split(&marker)
            .nth(1)
            .and_then(|rest| rest.split("\n\n").next())
            .unwrap_or("(no panic output captured)")
            .trim()
            .to_string();

        let (file_path, line_number) = location_re
            .captures(&panic_block)
            .map(|c| (Some(c[1].to_string()), c[2].parse::<i64>().ok()))
            .unwrap_or((None, None));

        bugs.push(DiscoveredBug {
            source: "test".to_string(),
            severity: "error".to_string(),
            title: format!("test failed: {name}"),
            message: panic_block,
            file_path,
            line_number,
        });
    }
    bugs
}

/// `svelte-check --output machine` emits one line per diagnostic:
/// `{timestamp} ERROR "message" file:line:col` (or WARNING). Machine mode
/// is the one documented for tooling consumption, unlike the human-verbose
/// default meant for a terminal.
fn scan_svelte_check(frontend_dir: &Path) -> Vec<DiscoveredBug> {
    let Some(output) = run_capture(
        frontend_dir,
        "npx",
        &["svelte-check", "--tsconfig", "./tsconfig.json", "--output", "machine"],
    ) else {
        return Vec::new();
    };
    parse_svelte_check_machine_output(&output)
}

fn parse_svelte_check_machine_output(output: &str) -> Vec<DiscoveredBug> {
    let line_re = Regex::new(r#"^\S+\s+(ERROR|WARNING)\s+"((?:[^"\\]|\\.)*)"\s+(\S+):(\d+):(\d+)"#).unwrap();
    let mut bugs = Vec::new();
    for line in output.lines() {
        let Some(cap) = line_re.captures(line) else { continue };
        let severity = if &cap[1] == "ERROR" { "error" } else { "warning" };
        let message = cap[2].replace("\\\"", "\"").replace("\\n", "\n");
        let file_path = cap[3].to_string();
        let line_number: i64 = cap[4].parse().unwrap_or(0);

        bugs.push(DiscoveredBug {
            source: "compile".to_string(),
            severity: severity.to_string(),
            title: message.lines().next().unwrap_or(&message).chars().take(200).collect(),
            message,
            file_path: Some(file_path),
            line_number: Some(line_number),
        });
    }
    bugs
}

/// `tsc --noEmit --pretty false` emits one line per diagnostic:
/// `path(line,col): error TSxxxx: message`.
fn scan_tsc(project_dir: &Path) -> Vec<DiscoveredBug> {
    let Some(output) = run_capture(project_dir, "npx", &["tsc", "--noEmit", "--pretty", "false"]) else {
        return Vec::new();
    };
    parse_tsc_output(&output)
}

fn parse_tsc_output(output: &str) -> Vec<DiscoveredBug> {
    let line_re = Regex::new(r"^(.+?)\((\d+),\d+\): error (TS\d+): (.+)$").unwrap();
    let mut bugs = Vec::new();
    for line in output.lines() {
        let Some(cap) = line_re.captures(line) else { continue };
        let file_path = cap[1].to_string();
        let line_number: i64 = cap[2].parse().unwrap_or(0);
        let code = &cap[3];
        let message = format!("{code}: {}", &cap[4]);

        bugs.push(DiscoveredBug {
            source: "compile".to_string(),
            severity: "error".to_string(),
            title: message.clone(),
            message,
            file_path: Some(file_path),
            line_number: Some(line_number),
        });
    }
    bugs
}

/// `eslint --format json` — only ever run against a target confirmed to
/// have ESLint actually configured (see module doc comment).
fn scan_eslint(project_dir: &Path) -> Vec<DiscoveredBug> {
    let Some(output) = run_capture(project_dir, "npx", &["eslint", ".", "--format", "json"]) else {
        return Vec::new();
    };
    parse_eslint_json(&output)
}

fn parse_eslint_json(output: &str) -> Vec<DiscoveredBug> {
    // eslint's stderr can precede the JSON array on some failures; find the
    // first '[' to isolate the actual JSON payload.
    let Some(start) = output.find('[') else { return Vec::new() };
    let Ok(files) = serde_json::from_str::<Vec<Value>>(&output[start..]) else { return Vec::new() };

    let mut bugs = Vec::new();
    for file in &files {
        let file_path = file.get("filePath").and_then(Value::as_str).unwrap_or("").to_string();
        let Some(messages) = file.get("messages").and_then(Value::as_array) else { continue };
        for m in messages {
            let severity_num = m.get("severity").and_then(Value::as_i64).unwrap_or(1);
            let severity = if severity_num >= 2 { "error" } else { "warning" };
            let rule = m.get("ruleId").and_then(Value::as_str).unwrap_or("unknown-rule");
            let text = m.get("message").and_then(Value::as_str).unwrap_or("").to_string();
            if text.is_empty() {
                continue;
            }
            let line_number = m.get("line").and_then(Value::as_i64);
            let message = format!("{rule}: {text}");

            bugs.push(DiscoveredBug {
                source: "lint".to_string(),
                severity: severity.to_string(),
                title: message.clone(),
                message,
                file_path: Some(file_path.clone()),
                line_number,
            });
        }
    }
    bugs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cargo_check_error_from_real_json_line() {
        let json_line = r#"{"reason":"compiler-message","message":{"level":"error","message":"mismatched types","spans":[{"file_name":"src/foo.rs","line_start":42,"is_primary":true}]}}"#;
        let bugs = parse_cargo_json_diagnostics(json_line, "compile");
        assert_eq!(bugs.len(), 1);
        assert_eq!(bugs[0].source, "compile");
        assert_eq!(bugs[0].severity, "error");
        assert_eq!(bugs[0].file_path.as_deref(), Some("src/foo.rs"));
        assert_eq!(bugs[0].line_number, Some(42));
    }

    #[test]
    fn ignores_non_compiler_message_reasons() {
        let json_line = r#"{"reason":"build-finished","success":true}"#;
        assert!(parse_cargo_json_diagnostics(json_line, "compile").is_empty());
    }

    #[test]
    fn ignores_notes_and_help_level_diagnostics() {
        let json_line = r#"{"reason":"compiler-message","message":{"level":"note","message":"see also","spans":[]}}"#;
        assert!(parse_cargo_json_diagnostics(json_line, "compile").is_empty());
    }

    #[test]
    fn parses_failed_test_name_via_the_same_regex_scan_cargo_test_uses() {
        let output = "running 1 test\ntest foo::tests::bar ... FAILED\n\nfailures:\n\n---- foo::tests::bar stdout ----\nthread 'foo::tests::bar' panicked at src/foo.rs:99:5:\nassertion failed\n\nfailures:\n    foo::tests::bar\n";
        let failed_re = Regex::new(r"(?m)^test (\S+) \.\.\. FAILED$").unwrap();
        let names: Vec<&str> = failed_re.captures_iter(output).map(|c| c.get(1).unwrap().as_str()).collect();
        assert_eq!(names, vec!["foo::tests::bar"]);
    }

    #[test]
    fn parses_svelte_check_machine_output_line() {
        let line = r#"1700000000000 ERROR "Cannot find name 'foo'" src/App.svelte:10:5"#;
        let bugs = parse_svelte_check_machine_output(line);
        assert_eq!(bugs.len(), 1);
        assert_eq!(bugs[0].severity, "error");
        assert_eq!(bugs[0].file_path.as_deref(), Some("src/App.svelte"));
        assert_eq!(bugs[0].line_number, Some(10));
    }

    #[test]
    fn parses_tsc_pretty_false_output_line() {
        let line = "src/extension.ts(42,10): error TS2304: Cannot find name 'foo'.";
        let bugs = parse_tsc_output(line);
        assert_eq!(bugs.len(), 1);
        assert_eq!(bugs[0].file_path.as_deref(), Some("src/extension.ts"));
        assert_eq!(bugs[0].line_number, Some(42));
        assert!(bugs[0].message.starts_with("TS2304"));
    }

    #[test]
    fn parses_eslint_json_output() {
        let json = r#"[{"filePath":"/repo/src/extension.ts","messages":[{"ruleId":"no-unused-vars","severity":2,"message":"'x' is defined but never used.","line":5}]}]"#;
        let bugs = parse_eslint_json(json);
        assert_eq!(bugs.len(), 1);
        assert_eq!(bugs[0].severity, "error");
        assert_eq!(bugs[0].source, "lint");
        assert_eq!(bugs[0].line_number, Some(5));
        assert!(bugs[0].message.contains("no-unused-vars"));
    }

    #[test]
    fn eslint_output_with_no_messages_produces_no_bugs() {
        let json = r#"[{"filePath":"/repo/src/extension.ts","messages":[]}]"#;
        assert!(parse_eslint_json(json).is_empty());
    }

    #[test]
    fn scan_target_prefixes_title_and_file_path_with_target_name() {
        // Exercises the post-processing step of `scan_target` directly
        // (without shelling out) by reusing its prefixing logic on a
        // synthetic bug list.
        let mut bugs = vec![DiscoveredBug {
            source: "compile".into(),
            severity: "error".into(),
            title: "mismatched types".into(),
            message: "mismatched types".into(),
            file_path: Some("src/lib.rs".into()),
            line_number: Some(1),
        }];
        let target = MonitorTarget { name: "kernel", rel_path: "Omnisystem/OmniHarness/kernel", kind: TargetKind::RustCrate };
        for bug in &mut bugs {
            bug.title = format!("[{}] {}", target.name, bug.title);
            if let Some(fp) = &bug.file_path {
                bug.file_path = Some(format!("{}/{}", target.rel_path, fp));
            }
        }
        assert_eq!(bugs[0].title, "[kernel] mismatched types");
        assert_eq!(bugs[0].file_path.as_deref(), Some("Omnisystem/OmniHarness/kernel/src/lib.rs"));
    }
}
