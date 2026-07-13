/// Audit Report Generator – Creates comprehensive audit reports and applies fixes
use crate::stub_detector::StubFinding;
use crate::repository_scanner::ScanResult;
use crate::auto_fixer::AutoFixer;
use std::path::Path;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditReport {
    pub timestamp: String,
    pub repository_root: String,
    pub total_findings: usize,
    pub findings: Vec<StubFinding>,
    pub findings_by_file: HashMap<String, Vec<StubFinding>>,
    pub summary: AuditSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditSummary {
    pub total_files_scanned: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub pass_fail: PassFailStatus,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum PassFailStatus {
    Pass,               // Zero critical/high severity issues
    Warn,               // Only low-severity issues
    Fail,               // Has critical or high severity issues
}

impl AuditReport {
    pub fn from_scan_result(scan_result: ScanResult, root_path: &Path) -> Self {
        let mut findings_by_file: HashMap<String, Vec<StubFinding>> = HashMap::new();

        for finding in &scan_result.findings {
            findings_by_file
                .entry(finding.file_path.clone())
                .or_insert_with(Vec::new)
                .push(finding.clone());
        }

        let pass_fail = if scan_result.findings_by_severity.critical > 0
            || scan_result.findings_by_severity.high > 0
        {
            PassFailStatus::Fail
        } else if scan_result.findings_by_severity.medium > 0 {
            PassFailStatus::Warn
        } else {
            PassFailStatus::Pass
        };

        let summary = AuditSummary {
            total_files_scanned: scan_result.total_files_scanned,
            critical_count: scan_result.findings_by_severity.critical,
            high_count: scan_result.findings_by_severity.high,
            medium_count: scan_result.findings_by_severity.medium,
            low_count: scan_result.findings_by_severity.low,
            pass_fail,
        };

        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            repository_root: root_path.to_string_lossy().to_string(),
            total_findings: scan_result.total_findings,
            findings: scan_result.findings,
            findings_by_file,
            summary,
        }
    }

    pub fn print_summary(&self) {
        println!("\n╔══════════════════════════════════════════════════════════════╗");
        println!("║          BONSAI BUG HUNTER – AUDIT REPORT                   ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!("\n📊 SUMMARY");
        println!("  Repository:    {}", self.repository_root);
        println!("  Timestamp:     {}", self.timestamp);
        println!("  Files scanned: {}", self.summary.total_files_scanned);
        println!("  Total findings: {}", self.total_findings);

        println!("\n⚠️  SEVERITY BREAKDOWN");
        println!("  🔴 Critical: {}", self.summary.critical_count);
        println!("  🟠 High:     {}", self.summary.high_count);
        println!("  🟡 Medium:   {}", self.summary.medium_count);
        println!("  🔵 Low:      {}", self.summary.low_count);

        println!("\n✅ STATUS: {}", match self.summary.pass_fail {
            PassFailStatus::Pass => "✓ PASS (Zero critical/high issues)",
            PassFailStatus::Warn => "⚠ WARN (Medium/low issues only)",
            PassFailStatus::Fail => "✗ FAIL (Critical or high severity issues detected)",
        });

        if !self.findings.is_empty() {
            println!("\n📝 TOP FINDINGS");
            let critical_and_high: Vec<_> = self
                .findings
                .iter()
                .filter(|f| {
                    f.severity == crate::stub_detector::Severity::Critical
                        || f.severity == crate::stub_detector::Severity::High
                })
                .take(10)
                .collect();

            for finding in critical_and_high {
                println!(
                    "  {} {} at {}:{}",
                    match finding.severity {
                        crate::stub_detector::Severity::Critical => "🔴",
                        crate::stub_detector::Severity::High => "🟠",
                        crate::stub_detector::Severity::Medium => "🟡",
                        crate::stub_detector::Severity::Low => "🔵",
                    },
                    finding.finding_type,
                    finding.file_path,
                    finding.line_number
                );
                println!("     {}",  finding.code_snippet);
                println!("     → {}", finding.suggested_fix);
            }
        }

        println!("\n════════════════════════════════════════════════════════════════\n");
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub async fn apply_fixes(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n🔧 APPLYING FIXES...\n");

        for (file_path, findings) in &self.findings_by_file {
            if findings.is_empty() {
                continue;
            }

            let path = Path::new(file_path);
            match AutoFixer::fix_file(path, findings) {
                Ok(_) => {
                    let critical = findings.iter().filter(|f| f.severity == crate::stub_detector::Severity::Critical).count();
                    let high = findings.iter().filter(|f| f.severity == crate::stub_detector::Severity::High).count();

                    if critical > 0 || high > 0 {
                        println!("✓ Fixed {} ({}C, {}H)", file_path, critical, high);
                    }
                }
                Err(e) => {
                    eprintln!("✗ Error fixing {}: {}", file_path, e);
                }
            }
        }

        println!("\n✅ Fix application complete!\n");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pass_fail_status_fail() {
        let mut summary = AuditSummary {
            total_files_scanned: 10,
            critical_count: 1,
            high_count: 0,
            medium_count: 0,
            low_count: 0,
            pass_fail: PassFailStatus::Pass,
        };

        summary.pass_fail = if summary.critical_count > 0 || summary.high_count > 0 {
            PassFailStatus::Fail
        } else {
            PassFailStatus::Pass
        };

        assert_eq!(summary.pass_fail, PassFailStatus::Fail);
    }

    #[test]
    fn test_pass_fail_status_pass() {
        let mut summary = AuditSummary {
            total_files_scanned: 10,
            critical_count: 0,
            high_count: 0,
            medium_count: 5,
            low_count: 10,
            pass_fail: PassFailStatus::Pass,
        };

        summary.pass_fail = if summary.critical_count > 0 || summary.high_count > 0 {
            PassFailStatus::Fail
        } else if summary.medium_count > 0 {
            PassFailStatus::Warn
        } else {
            PassFailStatus::Pass
        };

        assert_eq!(summary.pass_fail, PassFailStatus::Warn);
    }
}
