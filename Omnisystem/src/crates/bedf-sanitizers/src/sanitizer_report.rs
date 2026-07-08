use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueType {
    BufferOverflow,
    UseAfterFree,
    UninitializedMemory,
    DataRace,
    MemoryLeak,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryIssue {
    pub issue_type: IssueType,
    pub address: u64,
    pub size: usize,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitzerReport {
    pub asan_issues: usize,
    pub msan_issues: usize,
    pub tsan_issues: usize,
    pub lsan_issues: usize,
    pub total_issues: usize,
    pub issues: Vec<MemoryIssue>,
    pub duration_secs: f64,
}

impl SanitzerReport {
    pub fn summary(&self) -> String {
        format!(
            "Sanitizer Report: {} issues found (ASAN: {}, MSAN: {}, TSAN: {}, LSAN: {}) in {:.2}s",
            self.total_issues, self.asan_issues, self.msan_issues, self.tsan_issues, self.lsan_issues, self.duration_secs
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_issue_creation() {
        let issue = MemoryIssue {
            issue_type: IssueType::BufferOverflow,
            address: 0x1000,
            size: 200,
            description: "Buffer overflow detected".to_string(),
        };
        assert_eq!(issue.issue_type, IssueType::BufferOverflow);
    }

    #[test]
    fn test_report_creation() {
        let report = SanitzerReport {
            asan_issues: 1,
            msan_issues: 0,
            tsan_issues: 0,
            lsan_issues: 0,
            total_issues: 1,
            issues: vec![],
            duration_secs: 1.5,
        };
        assert_eq!(report.total_issues, 1);
    }

    #[test]
    fn test_report_summary() {
        let report = SanitzerReport {
            asan_issues: 2,
            msan_issues: 1,
            tsan_issues: 0,
            lsan_issues: 0,
            total_issues: 3,
            issues: vec![],
            duration_secs: 2.5,
        };
        let summary = report.summary();
        assert!(summary.contains("3 issues"));
        assert!(summary.contains("ASAN: 2"));
    }
}
