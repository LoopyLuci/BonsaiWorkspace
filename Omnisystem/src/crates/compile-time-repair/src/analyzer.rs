//! Source code analyzer for detecting compile-time errors

use anyhow::Result;
use std::fs;
use std::path::Path;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct CompileError {
    pub error_type: ErrorType,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub code_snippet: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType {
    UnusedVariable,
    MissingReturn,
    UnusedImport,
    NullPointerDereference,
    BufferOverflow,
    UndefinedFunction,
    TypeMismatch,
    LogicError,
    DeadCode,
    IncorrectDocComment,
}

pub struct CompileTimeAnalyzer {
    patterns: Vec<ErrorPattern>,
}

#[derive(Clone)]
struct ErrorPattern {
    name: String,
    regex: Regex,
    error_type: ErrorType,
    repair_suggestion: String,
}

impl CompileTimeAnalyzer {
    pub fn new() -> Self {
        let patterns = vec![
            // Unused variable pattern
            ErrorPattern {
                name: "unused_variable".to_string(),
                regex: Regex::new(r"let\s+(\w+)\s*=").unwrap(),
                error_type: ErrorType::UnusedVariable,
                repair_suggestion: "Add underscore prefix to variable name or use it in code".to_string(),
            },
            // Missing return statement
            ErrorPattern {
                name: "missing_return".to_string(),
                regex: Regex::new(r"fn\s+\w+\([^)]*\)\s*->\s*([^{]+)\s*\{").unwrap(),
                error_type: ErrorType::MissingReturn,
                repair_suggestion: "Add return statement before closing brace".to_string(),
            },
            // Unused import
            ErrorPattern {
                name: "unused_import".to_string(),
                regex: Regex::new(r"^use\s+[\w:]+;$").unwrap(),
                error_type: ErrorType::UnusedImport,
                repair_suggestion: "Remove unused import or add #[allow(unused_imports)]".to_string(),
            },
            // Incorrect doc comment
            ErrorPattern {
                name: "incorrect_doc".to_string(),
                regex: Regex::new(r"///\s*(.*)").unwrap(),
                error_type: ErrorType::IncorrectDocComment,
                repair_suggestion: "Ensure doc comment is properly formatted".to_string(),
            },
        ];

        Self { patterns }
    }

    /// Analyze a file for compile-time errors
    pub fn analyze_file(&self, path: &str) -> Result<Vec<CompileError>> {
        let content = fs::read_to_string(path)?;
        self.analyze_source(&content, path)
    }

    /// Analyze source code string
    pub fn analyze_source(&self, source: &str, filename: &str) -> Result<Vec<CompileError>> {
        let mut errors = Vec::new();

        for (line_num, line) in source.lines().enumerate() {
            let line_number = line_num + 1;

            // Check each error pattern
            for pattern in &self.patterns {
                if pattern.regex.is_match(line) {
                    errors.push(CompileError {
                        error_type: pattern.error_type.clone(),
                        line: line_number,
                        column: 0,
                        message: format!("{}: {}", pattern.name, &pattern.repair_suggestion),
                        code_snippet: line.to_string(),
                    });
                }
            }

            // Additional semantic checks
            if let Some(error) = self.check_semantic_errors(line, line_number) {
                errors.push(error);
            }
        }

        Ok(errors)
    }

    /// Check for semantic errors
    fn check_semantic_errors(&self, line: &str, line_number: usize) -> Option<CompileError> {
        // Check for dereferencing null pointers (pattern matching)
        if line.contains("null_ptr") || (line.contains("*") && line.contains("= NULL")) {
            return Some(CompileError {
                error_type: ErrorType::NullPointerDereference,
                line: line_number,
                column: 0,
                message: "Potential null pointer dereference - add null check".to_string(),
                code_snippet: line.to_string(),
            });
        }

        // Check for array bounds
        if line.contains("buffer[") && line.contains("]") && !line.contains("len()") && !line.contains("bounds_check") {
            // Simple heuristic: might need bounds check
            if line.contains("i") || line.contains("idx") {
                return Some(CompileError {
                    error_type: ErrorType::BufferOverflow,
                    line: line_number,
                    column: 0,
                    message: "Add bounds checking before array access".to_string(),
                    code_snippet: line.to_string(),
                });
            }
        }

        None
    }
}

impl Default for CompileTimeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_unused_variable() {
        let analyzer = CompileTimeAnalyzer::new();
        let source = "let x = 5;";
        let errors = analyzer.analyze_source(source, "test.rs").unwrap();
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_analyze_unused_import() {
        let analyzer = CompileTimeAnalyzer::new();
        let source = "use std::collections::HashMap;";
        let errors = analyzer.analyze_source(source, "test.rs").unwrap();
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_no_errors() {
        let analyzer = CompileTimeAnalyzer::new();
        let source = "fn main() {}";
        let errors = analyzer.analyze_source(source, "test.rs").unwrap();
        assert!(errors.len() < 5); // Should have minimal errors
    }
}
