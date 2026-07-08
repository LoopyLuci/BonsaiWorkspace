/// Stub Detector – Identifies incomplete code, placeholders, and anti-patterns
use regex::Regex;
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StubFinding {
    pub file_path: String,
    pub line_number: usize,
    pub finding_type: StubType,
    pub code_snippet: String,
    pub severity: Severity,
    pub suggested_fix: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StubType {
    TodoComment,
    UnimplementedMacro,
    PanicMacro,
    EmptyFunctionBody,
    PlaceholderReturn,
    UnwrapCall,
    HardcodedValue,
    IgnoredTest,
    SkippedTest,
    FakeImplementation,
    MockOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Critical,   // unimplemented! panic! in production code
    High,       // unwrap, todo! in non-test code
    Medium,     // empty functions, hardcoded values
    Low,        // ignored tests, comments
}

impl std::fmt::Display for StubType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StubType::TodoComment => write!(f, "TODO Comment"),
            StubType::UnimplementedMacro => write!(f, "unimplemented!() macro"),
            StubType::PanicMacro => write!(f, "panic!() macro"),
            StubType::EmptyFunctionBody => write!(f, "Empty function body"),
            StubType::PlaceholderReturn => write!(f, "Placeholder return"),
            StubType::UnwrapCall => write!(f, "Unwrap call"),
            StubType::HardcodedValue => write!(f, "Hardcoded value"),
            StubType::IgnoredTest => write!(f, "Ignored test"),
            StubType::SkippedTest => write!(f, "Skipped test"),
            StubType::FakeImplementation => write!(f, "Fake implementation"),
            StubType::MockOnly => write!(f, "Mock-only implementation"),
        }
    }
}

pub struct StubDetector {
    patterns: Vec<(Regex, StubType, Severity)>,
}

impl StubDetector {
    pub fn new() -> Self {
        let patterns = vec![
            // Critical patterns
            (Regex::new(r"unimplemented!\s*\(").unwrap(), StubType::UnimplementedMacro, Severity::Critical),
            (Regex::new(r"panic!\s*\(").unwrap(), StubType::PanicMacro, Severity::Critical),

            // High severity patterns
            (Regex::new(r"\.unwrap\(\)").unwrap(), StubType::UnwrapCall, Severity::High),
            (Regex::new(r"todo!\s*\(").unwrap(), StubType::TodoComment, Severity::High),

            // Medium severity patterns
            (Regex::new(r"#\[ignore\]").unwrap(), StubType::IgnoredTest, Severity::Medium),
            (Regex::new(r"#\[skip\]").unwrap(), StubType::SkippedTest, Severity::Medium),
            (Regex::new(r"Ok\(.*\)\s*//\s*TODO|Ok\(.*\)\s*//\s*FIXME").unwrap(), StubType::PlaceholderReturn, Severity::Medium),

            // Low severity patterns
            (Regex::new(r"//\s*TODO:|//\s*FIXME:|//\s*XXX:|//\s*HACK:").unwrap(), StubType::TodoComment, Severity::Low),
        ];

        Self { patterns }
    }

    pub fn scan_line(&self, line: &str, line_number: usize, file_path: &str) -> Vec<StubFinding> {
        let mut findings = Vec::new();

        // Skip comments and test-only code
        if line.trim_start().starts_with("//") && !line.contains("SAFETY:") {
            // Check for TODO/FIXME comments
            if let Some(caps) = Regex::new(r"//\s*(TODO|FIXME|XXX|HACK):?\s*(.*)").unwrap().captures(line) {
                let comment = caps.get(2).map_or("", |m| m.as_str());
                findings.push(StubFinding {
                    file_path: file_path.to_string(),
                    line_number,
                    finding_type: StubType::TodoComment,
                    code_snippet: line.trim().to_string(),
                    severity: Severity::Low,
                    suggested_fix: format!("Address the TODO: {}", comment),
                });
            }
        }

        // Skip test-only code
        if line.contains("#[cfg(test)]") || line.contains("#[test]") {
            return findings;
        }

        // Scan for patterns
        for (pattern, finding_type, severity) in &self.patterns {
            if pattern.is_match(line) {
                let suggested_fix = self.suggest_fix(*finding_type, line);
                findings.push(StubFinding {
                    file_path: file_path.to_string(),
                    line_number,
                    finding_type: *finding_type,
                    code_snippet: line.trim().to_string(),
                    severity: *severity,
                    suggested_fix,
                });
            }
        }

        findings
    }

    fn suggest_fix(&self, finding_type: StubType, line: &str) -> String {
        match finding_type {
            StubType::UnimplementedMacro => {
                "Replace unimplemented!() with actual implementation".to_string()
            }
            StubType::PanicMacro => {
                "Replace panic!() with Result error handling or graceful degradation".to_string()
            }
            StubType::UnwrapCall => {
                "Use Result error handling: ? operator or match statement".to_string()
            }
            StubType::TodoComment => {
                "Complete the TODO item or remove if resolved".to_string()
            }
            StubType::IgnoredTest => {
                "Re-enable test and ensure it passes: #[ignore] → remove".to_string()
            }
            StubType::SkippedTest => {
                "Complete the test implementation or remove #[skip]".to_string()
            }
            StubType::EmptyFunctionBody => {
                "Implement the function body or mark as intentional with Ok(())".to_string()
            }
            StubType::PlaceholderReturn => {
                "Replace placeholder return with actual implementation".to_string()
            }
            StubType::HardcodedValue => {
                "Extract hardcoded value to a configuration constant".to_string()
            }
            StubType::FakeImplementation => {
                "Replace fake implementation with production code".to_string()
            }
            StubType::MockOnly => {
                "Complete the real implementation or mark clearly as mock".to_string()
            }
        }
    }

    /// Detect empty function bodies
    pub fn detect_empty_function(&self, lines: &[&str], start_idx: usize) -> Option<StubFinding> {
        if start_idx >= lines.len() {
            return None;
        }

        let line = lines[start_idx];

        // Look for function signature
        if line.contains("fn ") || line.contains("pub fn ") || line.contains("async fn ") {
            // Check if body is empty or just returns Ok(())
            if let Some(next_line) = lines.get(start_idx + 1) {
                let trimmed = next_line.trim();
                if trimmed == "{" || trimmed == "Ok(())" || trimmed == "}" {
                    return Some(StubFinding {
                        file_path: "".to_string(),
                        line_number: start_idx + 1,
                        finding_type: StubType::EmptyFunctionBody,
                        code_snippet: line.trim().to_string(),
                        severity: Severity::Medium,
                        suggested_fix: "Implement the function body".to_string(),
                    });
                }
            }
        }

        None
    }

    /// Detect hardcoded test values that look like placeholders
    pub fn detect_hardcoded_value(&self, line: &str, line_number: usize, file_path: &str) -> Option<StubFinding> {
        // Pattern: literal numbers/strings that look like placeholders
        if Regex::new(r#""placeholder|0|""#).unwrap().is_match(line) && !line.contains("//") {
            return Some(StubFinding {
                file_path: file_path.to_string(),
                line_number,
                finding_type: StubType::HardcodedValue,
                code_snippet: line.trim().to_string(),
                severity: Severity::Medium,
                suggested_fix: "Use a configuration constant or realistic test value".to_string(),
            });
        }

        None
    }
}

impl Default for StubDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_unimplemented() {
        let detector = StubDetector::new();
        let findings = detector.scan_line("unimplemented!()", 1, "test.rs");
        assert!(!findings.is_empty());
        assert_eq!(findings[0].finding_type, StubType::UnimplementedMacro);
    }

    #[test]
    fn test_detects_todo() {
        let detector = StubDetector::new();
        let findings = detector.scan_line("// TODO: implement this", 1, "test.rs");
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_detects_unwrap() {
        let detector = StubDetector::new();
        let findings = detector.scan_line("result.unwrap()", 1, "test.rs");
        assert!(!findings.is_empty());
        assert_eq!(findings[0].finding_type, StubType::UnwrapCall);
    }
}
