/// Repository Scanner – Audits entire codebase for stubs and issues
use crate::stub_detector::{StubDetector, StubFinding, StubType, Severity};
use std::path::{Path, PathBuf};
use std::fs;
use walkdir::WalkDir;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanResult {
    pub total_files_scanned: usize,
    pub total_findings: usize,
    pub findings_by_severity: SeverityCounts,
    pub findings_by_type: TypeCounts,
    pub findings: Vec<StubFinding>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SeverityCounts {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TypeCounts {
    pub todo_comments: usize,
    pub unimplemented_macros: usize,
    pub panic_macros: usize,
    pub unwrap_calls: usize,
    pub empty_functions: usize,
    pub other: usize,
}

pub struct RepositoryScanner {
    detector: StubDetector,
    root_path: PathBuf,
}

impl RepositoryScanner {
    pub fn new(root_path: impl AsRef<Path>) -> Self {
        Self {
            detector: StubDetector::new(),
            root_path: root_path.as_ref().to_path_buf(),
        }
    }

    pub fn scan(&self) -> Result<ScanResult, Box<dyn std::error::Error>> {
        let mut findings = Vec::new();
        let mut file_count = 0;
        let mut severity_counts = SeverityCounts::default();
        let mut type_counts = TypeCounts::default();

        for entry in WalkDir::new(&self.root_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let path = e.path();
                path.extension()
                    .map(|ext| ext == "rs" || ext == "toml" || ext == "yaml" || ext == "yml")
                    .unwrap_or(false)
            })
            .filter(|e| {
                !e.path()
                    .to_string_lossy()
                    .contains("target")
            })
        {
            let path = entry.path();
            match fs::read_to_string(path) {
                Ok(content) => {
                    file_count += 1;
                    for (line_number, line) in content.lines().enumerate() {
                        let mut line_findings = self.detector.scan_line(line, line_number + 1, &path.to_string_lossy());

                        for finding in &mut line_findings {
                            finding.file_path = path.to_string_lossy().to_string();

                            match finding.severity {
                                Severity::Critical => severity_counts.critical += 1,
                                Severity::High => severity_counts.high += 1,
                                Severity::Medium => severity_counts.medium += 1,
                                Severity::Low => severity_counts.low += 1,
                            }

                            match finding.finding_type {
                                StubType::TodoComment => type_counts.todo_comments += 1,
                                StubType::UnimplementedMacro => type_counts.unimplemented_macros += 1,
                                StubType::PanicMacro => type_counts.panic_macros += 1,
                                StubType::UnwrapCall => type_counts.unwrap_calls += 1,
                                StubType::EmptyFunctionBody => type_counts.empty_functions += 1,
                                _ => type_counts.other += 1,
                            }

                            findings.push(finding.clone());
                        }
                    }
                }
                Err(_) => {
                    // Skip files that can't be read
                }
            }
        }

        let total_findings = findings.len();
        Ok(ScanResult {
            total_files_scanned: file_count,
            total_findings,
            findings_by_severity: severity_counts,
            findings_by_type: type_counts,
            findings,
        })
    }

    pub fn filter_by_severity(&self, findings: Vec<StubFinding>, min_severity: Severity) -> Vec<StubFinding> {
        findings
            .into_iter()
            .filter(|f| f.severity >= min_severity)
            .collect()
    }

    pub fn filter_by_type(&self, findings: Vec<StubFinding>, finding_type: StubType) -> Vec<StubFinding> {
        findings
            .into_iter()
            .filter(|f| f.finding_type == finding_type)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_creates_result() {
        // Would need a test directory with sample files
        // This is a placeholder for actual integration tests
    }
}
