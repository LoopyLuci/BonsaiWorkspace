//! Bug Hunter MCP Tool Handlers.
//!
//! Every handler in this file used to return hardcoded, canned JSON
//! regardless of input — `handle_get_finding` returned the *same* fake SQL
//! injection finding no matter what `finding_id` was passed, `handle_auto_fix`
//! claimed success without touching any file, and `handle_generate_report`
//! returned fixed numbers (`"total_findings": 87`) unconditionally. This
//! rewrite performs a real recursive filesystem scan using the shared rule
//! engine in `scan_rules`, persists results in memory keyed by `scan_id`, and
//! only claims a fix was applied when a real, unambiguous text replacement
//! was written to disk.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::scan_rules::{ext_of, is_excluded_dir, is_scannable_ext, scan_line};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub scan_id: String,
    pub severity: String,
    pub category: String,
    pub rule_id: String,
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub message: String,
    pub fixable: bool,
    /// Concrete (old, new) text — only set when the rule engine produced an
    /// unambiguous mechanical fix. Most rules here are detection-only
    /// (a hardcoded secret or a `todo!()` cannot be safely auto-rewritten),
    /// which `handle_auto_fix` reports honestly rather than pretending to fix.
    pub suggested_fix: Option<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScanRecord {
    scan_id: String,
    path: String,
    mode: String,
    timestamp: String,
    duration_ms: u128,
    files_scanned: usize,
    findings: Vec<Finding>,
}

fn store() -> &'static Mutex<HashMap<String, ScanRecord>> {
    static STORE: OnceLock<Mutex<HashMap<String, ScanRecord>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Iteratively (not recursively — avoids boxing an async fn) walk `root`,
/// skipping build/vendor directories, and run the shared rule engine over
/// every scannable source file it finds.
async fn scan_repo_real(root: &Path, exclude: &[String], scan_id: &str) -> Result<(Vec<Finding>, usize)> {
    let mut findings = Vec::new();
    let mut files_scanned = 0usize;
    let mut stack = vec![root.to_path_buf()];
    let mut next_id = 0u64;

    while let Some(dir) = stack.pop() {
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(_) => continue,
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(meta) = entry.metadata().await else { continue };

            if meta.is_dir() {
                let rel = path.to_string_lossy().replace('\\', "/");
                let excluded = is_excluded_dir(&name)
                    || exclude.iter().any(|p| rel.contains(p.trim_end_matches("/**").trim_end_matches('*')));
                if !excluded {
                    stack.push(path);
                }
                continue;
            }
            if !meta.is_file() {
                continue;
            }
            let ext = ext_of(&path);
            if !is_scannable_ext(ext) {
                continue;
            }
            let Ok(content) = tokio::fs::read_to_string(&path).await else {
                continue;
            };
            files_scanned += 1;
            let file_str = path.to_string_lossy().replace('\\', "/");
            for (line_num, line) in content.lines().enumerate() {
                for m in scan_line(line, ext) {
                    next_id += 1;
                    findings.push(Finding {
                        id: format!("{scan_id}-f{next_id}"),
                        scan_id: scan_id.to_string(),
                        severity: m.severity.to_string(),
                        category: m.category.to_string(),
                        rule_id: m.rule_id.to_string(),
                        file: file_str.clone(),
                        line: (line_num + 1) as u32,
                        column: m.column,
                        message: m.message,
                        fixable: m.fixable,
                        suggested_fix: m.suggested_fix,
                    });
                }
            }
        }
    }

    Ok((findings, files_scanned))
}

fn severity_rank(sev: &str) -> u8 {
    match sev {
        "critical" => 4,
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

/// Handle `bonsai_scan_repo` — real recursive repository scan.
///
/// Runs synchronously to completion (rather than the old fake
/// `"status": "started"` with nothing actually running in the background) so
/// the returned summary is real data from the moment the call returns.
pub async fn handle_scan_repo(args: Value) -> Result<Value> {
    let path = args["path"].as_str().unwrap_or(".").to_string();
    let mode = args["mode"].as_str().unwrap_or("full").to_string();
    let exclude: Vec<String> = args["exclude_patterns"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let root = PathBuf::from(&path);
    if !root.exists() {
        return Err(anyhow!("path does not exist: {path}"));
    }

    tracing::info!("Bug Hunt: scanning {} (mode={})", path, mode);
    let scan_id = format!("scan-{}", uuid::Uuid::new_v4());
    let start = std::time::Instant::now();

    let (mut findings, files_scanned) = scan_repo_real(&root, &exclude, &scan_id).await?;
    findings.sort_by(|a, b| severity_rank(&b.severity).cmp(&severity_rank(&a.severity)));
    let duration_ms = start.elapsed().as_millis();

    let mut by_severity: HashMap<String, usize> = HashMap::new();
    for f in &findings {
        *by_severity.entry(f.severity.clone()).or_insert(0) += 1;
    }
    let fixable_count = findings.iter().filter(|f| f.fixable).count();

    let record = ScanRecord {
        scan_id: scan_id.clone(),
        path: path.clone(),
        mode: mode.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        duration_ms,
        files_scanned,
        findings: findings.clone(),
    };
    store().lock().unwrap().insert(scan_id.clone(), record);

    Ok(json!({
        "scan_id": scan_id,
        "path": path,
        "mode": mode,
        "status": "completed",
        "files_scanned": files_scanned,
        "total_findings": findings.len(),
        "fixable_findings": fixable_count,
        "by_severity": by_severity,
        "duration_ms": duration_ms,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// Handle `bonsai_list_findings` — retrieve real findings from a real scan.
pub async fn handle_list_findings(args: Value) -> Result<Value> {
    let scan_id = args["scan_id"].as_str().ok_or_else(|| anyhow!("missing scan_id parameter"))?;
    let severity = args["severity"].as_str();
    let limit = args["limit"].as_u64().unwrap_or(50) as usize;

    let store = store().lock().unwrap();
    let record = store.get(scan_id).ok_or_else(|| anyhow!("unknown scan_id: {scan_id}"))?;

    let filtered: Vec<&Finding> = record
        .findings
        .iter()
        .filter(|f| severity.map(|s| f.severity == s).unwrap_or(true))
        .take(limit)
        .collect();

    Ok(json!({
        "scan_id": scan_id,
        "severity_filter": severity,
        "limit": limit,
        "findings": filtered,
        "total": record.findings.iter().filter(|f| severity.map(|s| f.severity == s).unwrap_or(true)).count(),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

fn find_finding(finding_id: &str) -> Option<Finding> {
    let store = store().lock().unwrap();
    store
        .values()
        .flat_map(|r| r.findings.iter())
        .find(|f| f.id == finding_id)
        .cloned()
}

/// Handle `bonsai_get_finding` — real lookup, not the same canned finding
/// every time regardless of `finding_id`.
pub async fn handle_get_finding(args: Value) -> Result<Value> {
    let finding_id = args["finding_id"].as_str().ok_or_else(|| anyhow!("missing finding_id parameter"))?;
    let finding = find_finding(finding_id).ok_or_else(|| anyhow!("unknown finding_id: {finding_id}"))?;

    let mut value = serde_json::to_value(&finding)?;
    if let Some(obj) = value.as_object_mut() {
        obj.insert("timestamp".into(), json!(chrono::Utc::now().to_rfc3339()));
    }
    Ok(value)
}

/// Handle `bonsai_auto_fix` — applies a real, literal text replacement only
/// when the finding has an unambiguous `suggested_fix` and the old text
/// appears exactly once in the file. Everything else honestly reports that
/// it can't be auto-fixed instead of claiming success with no effect.
pub async fn handle_auto_fix(args: Value) -> Result<Value> {
    let finding_id = args["finding_id"].as_str().ok_or_else(|| anyhow!("missing finding_id parameter"))?;
    let confirm = args["confirm"].as_bool().unwrap_or(false);

    let finding = find_finding(finding_id).ok_or_else(|| anyhow!("unknown finding_id: {finding_id}"))?;

    let Some((old, new)) = finding.suggested_fix.clone() else {
        return Ok(json!({
            "finding_id": finding_id,
            "status": "not_fixable",
            "message": format!(
                "Rule '{}' is detection-only — it has no safe mechanical rewrite, so this finding needs a human fix.",
                finding.rule_id
            ),
        }));
    };

    if !confirm {
        return Ok(json!({
            "finding_id": finding_id,
            "status": "needs_confirmation",
            "message": "Fix requires explicit confirmation",
            "preview": format!("Replace:\n{old}\nWith:\n{new}"),
        }));
    }

    let content = tokio::fs::read_to_string(&finding.file).await
        .map_err(|e| anyhow!("could not read {}: {e}", finding.file))?;
    let occurrences = content.matches(old.as_str()).count();
    if occurrences != 1 {
        return Ok(json!({
            "finding_id": finding_id,
            "status": "ambiguous",
            "message": format!(
                "Expected exactly one occurrence of the flagged text in {}, found {}. Refusing to guess which one to change.",
                finding.file, occurrences
            ),
        }));
    }

    let updated = content.replacen(old.as_str(), &new, 1);
    tokio::fs::write(&finding.file, &updated).await
        .map_err(|e| anyhow!("could not write {}: {e}", finding.file))?;

    Ok(json!({
        "finding_id": finding_id,
        "status": "fixed",
        "message": "Fix applied successfully",
        "files_modified": [finding.file],
        "changes": [{ "file": finding.file, "line": finding.line, "before": old, "after": new }],
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// Handle `bonsai_explain_diagnostic` — real explanation templated from the
/// real finding's rule/category, not a fixed SQL-injection writeup for
/// everything.
pub async fn handle_explain_diagnostic(args: Value) -> Result<Value> {
    let finding_id = args["finding_id"].as_str().ok_or_else(|| anyhow!("missing finding_id parameter"))?;
    let finding = find_finding(finding_id).ok_or_else(|| anyhow!("unknown finding_id: {finding_id}"))?;

    let (title, explanation, mitigation): (&str, String, Vec<&str>) = match finding.rule_id.as_str() {
        "security:hardcoded-secret" => (
            "Hardcoded secret",
            format!(
                "Line {} in {} appears to assign a literal credential value directly in source. \
                 Anyone with read access to the repository (or its git history) can recover it.",
                finding.line, finding.file
            ),
            vec!["Move the value to an environment variable or secrets manager", "Rotate the exposed credential"],
        ),
        "rust:unimplemented" | "rust:todo-macro" => (
            "Incomplete implementation",
            format!("{} at {}:{} will panic the process if this code path is ever reached at runtime.", finding.message, finding.file, finding.line),
            vec!["Implement the missing logic", "Or return a proper Result/Option error instead of panicking"],
        ),
        "lint:stub-implementation" => (
            "Placeholder implementation",
            format!(
                "The comment at {}:{} admits the surrounding code doesn't do what its name/signature implies — \
                 it likely returns a plausible-looking but fake result.",
                finding.file, finding.line
            ),
            vec!["Replace the stub with a real implementation", "Or clearly surface the limitation to callers instead of a fake success value"],
        ),
        "lint:task-marker" => (
            "Task marker",
            format!("A TODO/FIXME comment at {}:{} — tracked but not yet acted on.", finding.file, finding.line),
            vec!["Resolve or convert to a tracked issue"],
        ),
        _ => (
            "Code quality finding",
            finding.message.clone(),
            vec!["Review the flagged line and address the pattern"],
        ),
    };

    Ok(json!({
        "finding_id": finding_id,
        "title": title,
        "explanation": explanation,
        "risk_level": finding.severity,
        "category": finding.category,
        "mitigation": mitigation,
        "rule_id": finding.rule_id,
        "file": finding.file,
        "line": finding.line,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// Handle `bonsai_prioritize_findings` — real ranking of the real findings
/// from a scan, by severity weight adjusted for fix effort (a fixable
/// finding is cheaper to resolve than one needing a human).
pub async fn handle_prioritize_findings(args: Value) -> Result<Value> {
    let scan_id = args["scan_id"].as_str().ok_or_else(|| anyhow!("missing scan_id parameter"))?;
    let strategy = args["strategy"].as_str().unwrap_or("impact").to_string();

    let store = store().lock().unwrap();
    let record = store.get(scan_id).ok_or_else(|| anyhow!("unknown scan_id: {scan_id}"))?;

    let mut ranked: Vec<Value> = record
        .findings
        .iter()
        .map(|f| {
            let impact = severity_rank(&f.severity) as f64 * 2.5;
            let effort = if f.fixable { 1.0 } else { 4.0 };
            let priority_score = impact / effort;
            json!({
                "finding_id": f.id,
                "severity": f.severity,
                "impact_score": impact,
                "effort_score": effort,
                "priority_score": priority_score,
            })
        })
        .collect();
    ranked.sort_by(|a, b| {
        b["priority_score"].as_f64().unwrap_or(0.0)
            .partial_cmp(&a["priority_score"].as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (i, item) in ranked.iter_mut().enumerate() {
        item["priority"] = json!(i + 1);
    }

    Ok(json!({
        "scan_id": scan_id,
        "strategy": strategy,
        "findings_by_priority": ranked,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// Handle `bonsai_generate_report` — computed from the real stored scan,
/// not fixed numbers returned regardless of what (if anything) was scanned.
pub async fn handle_generate_report(args: Value) -> Result<Value> {
    let scan_id = args["scan_id"].as_str().ok_or_else(|| anyhow!("missing scan_id parameter"))?;
    let format = args["format"].as_str().unwrap_or("json").to_string();

    let store = store().lock().unwrap();
    let record = store.get(scan_id).ok_or_else(|| anyhow!("unknown scan_id: {scan_id}"))?;

    let mut by_severity: HashMap<String, usize> = HashMap::new();
    for f in &record.findings {
        *by_severity.entry(f.severity.clone()).or_insert(0) += 1;
    }
    let fixable = record.findings.iter().filter(|f| f.fixable).count();

    Ok(json!({
        "scan_id": scan_id,
        "format": format,
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "title": "Bug Hunt Report",
        "summary": {
            "total_findings": record.findings.len(),
            "critical": by_severity.get("critical").copied().unwrap_or(0),
            "high": by_severity.get("high").copied().unwrap_or(0),
            "medium": by_severity.get("medium").copied().unwrap_or(0),
            "low": by_severity.get("low").copied().unwrap_or(0),
            "info": by_severity.get("info").copied().unwrap_or(0),
            "auto_fixable": fixable,
            "requires_review": record.findings.len().saturating_sub(fixable),
        },
        "statistics": {
            "scan_duration_ms": record.duration_ms,
            "files_scanned": record.files_scanned,
            "path": record.path,
            "mode": record.mode,
        },
        "message": format!("Report generated for scan {scan_id}"),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// End-to-end: scan a real temp directory containing a real bug, then
    /// list/get/explain/prioritize/report against the real stored result —
    /// this is the exact chain that used to return the same canned SQL
    /// injection finding regardless of input.
    #[tokio::test]
    async fn scan_finds_real_issues_and_downstream_calls_use_real_data() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("lib.rs");
        tokio::fs::write(&file_path, "fn f() {\n    unimplemented!()\n}\n// TODO: revisit\n")
            .await
            .unwrap();

        let scan = handle_scan_repo(json!({ "path": dir.path().to_str().unwrap() }))
            .await
            .unwrap();
        assert_eq!(scan["status"], "completed");
        assert_eq!(scan["files_scanned"], 1);
        let scan_id = scan["scan_id"].as_str().unwrap().to_string();
        assert!(scan["total_findings"].as_u64().unwrap() >= 2);

        let list = handle_list_findings(json!({ "scan_id": scan_id })).await.unwrap();
        let findings = list["findings"].as_array().unwrap();
        assert!(!findings.is_empty());
        let real_finding_id = findings[0]["id"].as_str().unwrap().to_string();
        // Every finding must actually belong to this scan and this file —
        // not a fixed "finding-1"/"finding-2" fabricated regardless of input.
        for f in findings {
            assert_eq!(f["scan_id"], scan_id);
            assert!(f["file"].as_str().unwrap().ends_with("lib.rs"));
        }

        let got = handle_get_finding(json!({ "finding_id": real_finding_id })).await.unwrap();
        assert_eq!(got["id"], real_finding_id);

        // An unknown finding_id must fail loudly, not silently return the
        // same fake finding as any other id.
        assert!(handle_get_finding(json!({ "finding_id": "not-a-real-id" })).await.is_err());

        let explained = handle_explain_diagnostic(json!({ "finding_id": real_finding_id })).await.unwrap();
        assert_eq!(explained["finding_id"], real_finding_id);

        let prioritized = handle_prioritize_findings(json!({ "scan_id": scan_id })).await.unwrap();
        assert!(!prioritized["findings_by_priority"].as_array().unwrap().is_empty());

        let report = handle_generate_report(json!({ "scan_id": scan_id })).await.unwrap();
        // Must reflect the real count from this scan, not a fixed 87/3/12/etc.
        assert_eq!(report["summary"]["total_findings"], findings.len());
    }

    /// The one real mechanical fix (`js:var-keyword`) must actually rewrite
    /// the file on disk when confirmed, and must not apply anything without
    /// confirmation first.
    #[tokio::test]
    async fn auto_fix_applies_a_real_unambiguous_rewrite() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("app.js");
        tokio::fs::write(&file_path, "function f() {\n    var total = 0;\n    return total;\n}\n")
            .await
            .unwrap();

        let scan = handle_scan_repo(json!({ "path": dir.path().to_str().unwrap() })).await.unwrap();
        let scan_id = scan["scan_id"].as_str().unwrap().to_string();
        let list = handle_list_findings(json!({ "scan_id": scan_id })).await.unwrap();
        let findings = list["findings"].as_array().unwrap();
        let var_finding = findings
            .iter()
            .find(|f| f["rule_id"] == "js:var-keyword")
            .expect("expected a js:var-keyword finding");
        let finding_id = var_finding["id"].as_str().unwrap().to_string();

        // Without confirm=true, nothing on disk should change.
        let preview = handle_auto_fix(json!({ "finding_id": finding_id, "confirm": false })).await.unwrap();
        assert_eq!(preview["status"], "needs_confirmation");
        let unchanged = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert!(unchanged.contains("var total"));

        // With confirmation, the real file must actually be rewritten.
        let fixed = handle_auto_fix(json!({ "finding_id": finding_id, "confirm": true })).await.unwrap();
        assert_eq!(fixed["status"], "fixed");
        let changed = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert!(changed.contains("let total"));
        assert!(!changed.contains("var total"));
    }

    /// A finding with no `suggested_fix` (the common case — most rules are
    /// detection-only) must be reported as genuinely not fixable, never as a
    /// fake success.
    #[tokio::test]
    async fn auto_fix_declines_when_no_mechanical_fix_exists() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("lib.rs");
        tokio::fs::write(&file_path, "fn f() { unimplemented!() }\n").await.unwrap();

        let scan = handle_scan_repo(json!({ "path": dir.path().to_str().unwrap() })).await.unwrap();
        let scan_id = scan["scan_id"].as_str().unwrap().to_string();
        let list = handle_list_findings(json!({ "scan_id": scan_id })).await.unwrap();
        let finding_id = list["findings"][0]["id"].as_str().unwrap().to_string();

        let result = handle_auto_fix(json!({ "finding_id": finding_id, "confirm": true })).await.unwrap();
        assert_eq!(result["status"], "not_fixable");
    }

    #[tokio::test]
    async fn scan_repo_rejects_nonexistent_path() {
        let result = handle_scan_repo(json!({ "path": "/definitely/does/not/exist/anywhere" })).await;
        assert!(result.is_err());
    }
}
