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
pub struct SanitizerReport {
    pub asan_issues: usize,
    pub msan_issues: usize,
    pub tsan_issues: usize,
    pub lsan_issues: usize,
    pub total_issues: usize,
    pub issues: Vec<MemoryIssue>,
    pub duration_secs: f64,
}

impl SanitizerReport {
    /// Build a report by tallying `issues` into their sanitizer buckets
    /// (ASAN: buffer overflow / use-after-free, MSAN: uninitialized memory,
    /// TSAN: data race, LSAN: memory leak) rather than requiring the caller
    /// to compute the counts by hand.
    pub fn from_issues(issues: Vec<MemoryIssue>, duration_secs: f64) -> Self {
        let mut report = Self {
            asan_issues: 0,
            msan_issues: 0,
            tsan_issues: 0,
            lsan_issues: 0,
            total_issues: issues.len(),
            issues,
            duration_secs,
        };

        for issue in &report.issues {
            match issue.issue_type {
                IssueType::BufferOverflow | IssueType::UseAfterFree => report.asan_issues += 1,
                IssueType::UninitializedMemory => report.msan_issues += 1,
                IssueType::DataRace => report.tsan_issues += 1,
                IssueType::MemoryLeak => report.lsan_issues += 1,
            }
        }

        report
    }

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
        let report = SanitizerReport {
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
        let report = SanitizerReport {
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

    #[test]
    fn test_report_from_issues_tallies_buckets() {
        let issues = vec![
            MemoryIssue {
                issue_type: IssueType::BufferOverflow,
                address: 0x1000,
                size: 8,
                description: "overflow".to_string(),
            },
            MemoryIssue {
                issue_type: IssueType::UseAfterFree,
                address: 0x2000,
                size: 8,
                description: "uaf".to_string(),
            },
            MemoryIssue {
                issue_type: IssueType::UninitializedMemory,
                address: 0x3000,
                size: 8,
                description: "uninit".to_string(),
            },
            MemoryIssue {
                issue_type: IssueType::DataRace,
                address: 0x4000,
                size: 8,
                description: "race".to_string(),
            },
            MemoryIssue {
                issue_type: IssueType::MemoryLeak,
                address: 0x5000,
                size: 8,
                description: "leak".to_string(),
            },
        ];

        let report = SanitizerReport::from_issues(issues, 3.0);
        assert_eq!(report.total_issues, 5);
        assert_eq!(report.asan_issues, 2); // BufferOverflow + UseAfterFree
        assert_eq!(report.msan_issues, 1);
        assert_eq!(report.tsan_issues, 1);
        assert_eq!(report.lsan_issues, 1);
    }
}
