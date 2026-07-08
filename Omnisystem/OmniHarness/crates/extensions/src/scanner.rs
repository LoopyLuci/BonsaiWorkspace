//! Security Scanner — multi-stage static analysis + LLM stub for extension code review.
//!
//! Produces a structured `SecurityReport` with plain-language findings
//! understandable by non-technical users.

use std::path::{Path, PathBuf};

use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use walkdir::WalkDir;

use crate::manifest::SecurityVerdict;

// ── Finding severity ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "ℹ️  Info"),
            Severity::Low => write!(f, "🔵 Low"),
            Severity::Medium => write!(f, "🟡 Medium"),
            Severity::High => write!(f, "🟠 High"),
            Severity::Critical => write!(f, "🔴 Critical"),
        }
    }
}

// ── A single finding ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub severity: Severity,
    /// Path relative to the repo root.
    pub file: String,
    pub line: Option<usize>,
    /// The matched code snippet (≤ 120 chars).
    pub snippet: String,
    /// Technical description (for developers).
    pub technical: String,
    /// Plain-English explanation (for non-technical users).
    pub plain_english: String,
    /// Whether the user can explicitly allow this finding.
    pub allowable: bool,
}

// ── Full security report ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    pub extension_id: String,
    pub extension_version: String,
    pub scanned_at: chrono::DateTime<chrono::Utc>,
    pub verdict: SecurityVerdict,
    /// 0 = no risk, 100 = maximum risk.
    pub risk_score: u8,
    pub findings: Vec<Finding>,
    /// Number of files scanned.
    pub files_scanned: usize,
    /// Aggregate plain-English summary (shown at the top of the report card).
    pub summary: String,
    /// "What this means for you" — 2-3 sentence non-technical explanation.
    pub user_message: String,
    /// 0–100: how easy the code was to understand (high = transparent).
    pub transparency_score: u8,
    /// Content hash of the scanned directory.
    pub content_hash: String,
}

impl SecurityReport {
    pub fn critical_count(&self) -> usize {
        self.findings.iter().filter(|f| f.severity == Severity::Critical).count()
    }
    pub fn high_count(&self) -> usize {
        self.findings.iter().filter(|f| f.severity == Severity::High).count()
    }
    pub fn medium_count(&self) -> usize {
        self.findings.iter().filter(|f| f.severity == Severity::Medium).count()
    }
    pub fn low_count(&self) -> usize {
        self.findings.iter().filter(|f| f.severity == Severity::Low).count()
    }
}

// ── Static-analysis rules ─────────────────────────────────────────────────────

struct Rule {
    pattern: Regex,
    severity: Severity,
    technical: &'static str,
    plain_english: &'static str,
    allowable: bool,
}

fn build_rules() -> Vec<Rule> {
    let specs: &[(&str, Severity, &str, &str, bool)] = &[
        // Critical: destructive file system
        (r"(?i)rm\s+-rf\s*/|del\s+/f\s+/s", Severity::Critical,
         "Destructive filesystem deletion (rm -rf / or del /f /s)",
         "This code could delete important files on your computer.", false),
        // Critical: command execution
        (r"(?i)(eval\s*\(|exec\s*\(|os\.system\s*\(|subprocess\.call\s*\(|popen\s*\()",
         Severity::Critical,
         "Arbitrary command execution via eval/exec/system/popen",
         "This code can run arbitrary programs on your device — a common technique in malware.", false),
        // Critical: pipe-to-shell pattern (curl | sh)
        (r#"(?i)(curl|wget)\s+.+\|\s*sh|bash\s+-c\s*["'].*curl"#,
         Severity::Critical,
         "Remote code execution via curl/wget piped to shell",
         "This code downloads and immediately runs programs from the internet — a very dangerous pattern.", false),
        // High: privilege escalation
        (r"(?i)(sudo\s|chmod\s+777|os\.setuid|setgid|privilege)",
         Severity::High,
         "Potential privilege escalation",
         "This code tries to gain elevated system permissions, which could allow it to bypass security controls.", false),
        // High: data exfiltration patterns
        (r#"(?i)(send_file|upload_data|exfiltrat|requests\.post\(\s*["']https?://\d+\.\d+\.\d+)"#,
         Severity::High,
         "Potential data exfiltration",
         "This code appears to send data to an external server. Make sure you trust where your data is going.", false),
        // Medium: obfuscation
        (r#"(?i)(base64\.b64decode.*exec|__import__\s*\(\s*["']os["']|\\x[0-9a-f]{2}\\x[0-9a-f]{2}\\x[0-9a-f]{2})"#,
         Severity::Medium,
         "Code obfuscation detected",
         "Part of this code is hidden or encoded, which is sometimes used to disguise malicious behaviour.", true),
        // Medium: network to non-declared host
        (r#"(?i)(https?://(?!localhost|127\.0\.0\.1)[\w\-\.]+\.[a-z]{2,})"#,
         Severity::Medium,
         "Network request to external host",
         "This code contacts an external server. Check that the domain is listed in the extension's declared permissions.", true),
        // Low: process spawning
        (r"(?i)(std::process::Command|child_process\.exec|spawn\(|ProcessBuilder)",
         Severity::Low,
         "Process spawning",
         "This code starts other programs. This is often legitimate (e.g., running cargo audit), but worth noting.", true),
        // Low: env var access
        (r"(?i)(std::env::|process\.env\.|os\.environ)",
         Severity::Low,
         "Environment variable access",
         "This code reads environment variables, which may include sensitive information like API keys.", true),
    ];

    specs
        .iter()
        .filter_map(|(pattern, severity, technical, plain_english, allowable)| {
            Regex::new(pattern).ok().map(|re| Rule {
                pattern: re,
                severity: *severity,
                technical,
                plain_english,
                allowable: *allowable,
            })
        })
        .collect()
}

// ── Scanner ───────────────────────────────────────────────────────────────────

/// Extensions whose content we skip when scanning.
const SKIP_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "svg", "ico", "wasm", "ttf", "woff", "woff2"];
/// Files whose names we always skip.
const SKIP_FILES: &[&str] = &["package-lock.json", "yarn.lock", "Cargo.lock"];
/// Maximum file size to scan (256 KB).
const MAX_SCAN_BYTES: u64 = 256 * 1024;

pub struct SecurityScanner {
    rules: Vec<Rule>,
}

impl SecurityScanner {
    pub fn new() -> Self {
        Self { rules: build_rules() }
    }

    /// Scan a directory and return a full `SecurityReport`.
    pub async fn scan(
        &self,
        dir: &Path,
        extension_id: &str,
        extension_version: &str,
    ) -> SecurityReport {
        info!(path = %dir.display(), "[scanner] starting scan");
        let mut findings: Vec<Finding> = Vec::new();
        let mut files_scanned: usize = 0;
        let mut total_bytes: u64 = 0;
        let mut hasher = blake3::Hasher::new();

        for entry in WalkDir::new(dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();

            // Skip binary / lock files
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if SKIP_EXTENSIONS.contains(&ext) {
                continue;
            }
            let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if SKIP_FILES.contains(&fname) {
                continue;
            }

            // Skip files that are too large
            let Ok(meta) = path.metadata() else { continue };
            if meta.len() > MAX_SCAN_BYTES {
                warn!(path = %path.display(), "[scanner] skipping large file");
                continue;
            }

            let Ok(content) = tokio::fs::read_to_string(path).await else { continue };
            total_bytes += meta.len();
            files_scanned += 1;

            // Contribute to content hash
            hasher.update(content.as_bytes());

            let rel_path = path
                .strip_prefix(dir)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");

            // Apply all rules line-by-line
            for (line_num, line) in content.lines().enumerate() {
                for rule in &self.rules {
                    if let Some(m) = rule.pattern.find(line) {
                        let snippet = &line[m.start()..m.end().min(line.len())];
                        let snippet = snippet.chars().take(120).collect::<String>();
                        findings.push(Finding {
                            severity: rule.severity,
                            file: rel_path.clone(),
                            line: Some(line_num + 1),
                            snippet,
                            technical: rule.technical.to_string(),
                            plain_english: rule.plain_english.to_string(),
                            allowable: rule.allowable,
                        });
                        // Only report the highest-severity match per line per file
                        break;
                    }
                }
            }
        }

        let content_hash = hex::encode(hasher.finalize().as_bytes());

        // Deduplicate: keep only the highest severity per (file, line) pair
        findings.sort_by(|a, b| {
            (&a.file, &a.line).cmp(&(&b.file, &b.line))
                .then(b.severity.cmp(&a.severity))
        });
        findings.dedup_by(|a, b| a.file == b.file && a.line == b.line);

        // Calculate risk score and verdict
        let (risk_score, verdict) = calculate_risk(&findings);
        let transparency_score = calculate_transparency(files_scanned, &findings);

        let summary = build_summary(&findings, files_scanned);
        let user_message = build_user_message(&verdict, &findings);

        info!(
            files = files_scanned,
            findings = findings.len(),
            risk_score,
            "[scanner] scan complete"
        );

        SecurityReport {
            extension_id: extension_id.to_string(),
            extension_version: extension_version.to_string(),
            scanned_at: chrono::Utc::now(),
            verdict,
            risk_score,
            findings,
            files_scanned,
            summary,
            user_message,
            transparency_score,
            content_hash,
        }
    }
}

fn calculate_risk(findings: &[Finding]) -> (u8, SecurityVerdict) {
    let critical = findings.iter().filter(|f| f.severity == Severity::Critical && !f.allowable).count();
    let high = findings.iter().filter(|f| f.severity == Severity::High && !f.allowable).count();
    let medium = findings.iter().filter(|f| f.severity == Severity::Medium).count();

    let score = (critical * 40 + high * 20 + medium * 5).min(100) as u8;

    let verdict = if critical > 0 || score >= 80 {
        SecurityVerdict::Risky
    } else if high > 0 || score >= 40 {
        SecurityVerdict::Caution
    } else {
        SecurityVerdict::Safe
    };

    (score, verdict)
}

fn calculate_transparency(files: usize, findings: &[Finding]) -> u8 {
    if files == 0 {
        return 50;
    }
    let obfuscation = findings.iter().filter(|f| f.technical.contains("obfuscat")).count();
    let base = 100u8;
    let deduct = (obfuscation * 15).min(50) as u8;
    base.saturating_sub(deduct)
}

fn build_summary(findings: &[Finding], files: usize) -> String {
    let c = findings.iter().filter(|f| f.severity == Severity::Critical).count();
    let h = findings.iter().filter(|f| f.severity == Severity::High).count();
    let m = findings.iter().filter(|f| f.severity == Severity::Medium).count();
    let l = findings.iter().filter(|f| f.severity == Severity::Low).count();
    format!(
        "Scanned {files} files. Found {} critical, {} high, {} medium, {} low/info issues.",
        c, h, m, l
    )
}

fn build_user_message(verdict: &SecurityVerdict, findings: &[Finding]) -> String {
    match verdict {
        SecurityVerdict::Safe => {
            "This extension appears safe to install. It behaves as described and our analysis found \
            no signs of malicious behaviour. You can install it with confidence, though you should \
            always review what permissions it requests."
                .to_string()
        }
        SecurityVerdict::Caution => {
            let count = findings.iter().filter(|f| f.severity >= Severity::High).count();
            format!(
                "This extension raised {} concern(s) worth reviewing before installing. \
                This doesn't mean it's malicious, but we recommend reading the findings below \
                and making sure you're comfortable with what the extension does.",
                count
            )
        }
        SecurityVerdict::Risky => {
            "This extension contains patterns commonly associated with malicious software. \
            We strongly recommend NOT installing it unless you fully understand the code and \
            trust the author. If you're unsure, ask someone technical to review the findings below."
                .to_string()
        }
        SecurityVerdict::Blocked => {
            "This extension has been blocked because it contains code that is almost certainly \
            malicious. Do not install it."
                .to_string()
        }
        SecurityVerdict::Unreviewed => "This extension has not been reviewed yet.".to_string(),
    }
}

impl Default for SecurityScanner {
    fn default() -> Self {
        Self::new()
    }
}
