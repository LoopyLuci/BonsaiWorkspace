use crate::diagnostics::{Diagnostic, Fix, Range, Position, Severity};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tree_sitter::Tree;

/// A static linting rule defined in YAML or TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticRule {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub languages: Vec<String>,
    pub pattern: String,               // AST-grep or regex pattern
    pub pattern_type: PatternType,
    pub message: String,
    pub severity: SeverityLevel,
    pub fix: Option<FixDefinition>,
    pub enabled: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub category: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatternType {
    AstGrep,
    Regex,
    Structural,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixDefinition {
    pub replace: Option<String>,
    pub insert: Option<String>,
    pub delete: Option<bool>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SeverityLevel {
    Note,
    Hint,
    Warning,
    Error,
    Fatal,
}

impl From<SeverityLevel> for Severity {
    fn from(level: SeverityLevel) -> Self {
        match level {
            SeverityLevel::Note => Severity::Note,
            SeverityLevel::Hint => Severity::Hint,
            SeverityLevel::Warning => Severity::Warning,
            SeverityLevel::Error => Severity::Error,
            SeverityLevel::Fatal => Severity::Fatal,
        }
    }
}

impl StaticRule {
    /// Create a new static rule.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        languages: Vec<String>,
        pattern: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            languages,
            pattern: pattern.into(),
            pattern_type: PatternType::AstGrep,
            message: message.into(),
            severity: SeverityLevel::Warning,
            fix: None,
            enabled: true,
            tags: Vec::new(),
            category: String::new(),
        }
    }

    /// Load a rule from a YAML file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_yaml::from_str(&content).map_err(|e| anyhow!("Failed to parse rule file: {}", e))
    }

    /// Save a rule to a YAML file.
    pub fn to_file(&self, path: &Path) -> Result<()> {
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn with_fix(mut self, fix: FixDefinition) -> Self {
        self.fix = Some(fix);
        self
    }

    pub fn with_severity(mut self, severity: SeverityLevel) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Apply the rule to a parsed tree and return diagnostics.
    pub fn apply_to_tree(&self, tree: &Tree, source: &str, file_path: &Path) -> Result<Vec<Diagnostic>> {
        if !self.enabled {
            return Ok(Vec::new());
        }

        let mut diagnostics = Vec::new();

        match self.pattern_type {
            PatternType::Regex => {
                let regex = regex::Regex::new(&self.pattern)?;
                for mat in regex.find_iter(source) {
                    let (line, col) = byte_offset_to_line_col(source, mat.start());
                    let (end_line, end_col) = byte_offset_to_line_col(source, mat.end());

                    let range = Range::new(
                        Position { line: line as u32, column: col as u32 },
                        Position { line: end_line as u32, column: end_col as u32 },
                        mat.start(),
                        mat.end(),
                    );

                    let mut diag = Diagnostic::new(
                        &self.id,
                        &self.message,
                        self.severity.into(),
                        file_path.to_path_buf(),
                        range,
                    );

                    if let Some(fix_def) = &self.fix {
                        if let Some(replace) = &fix_def.replace {
                            diag.fix = Some(Fix::Replace(replace.clone()));
                        } else if let Some(insert) = &fix_def.insert {
                            diag.fix = Some(Fix::Insert(insert.clone()));
                        } else if fix_def.delete.is_some() {
                            diag.fix = Some(Fix::Delete);
                        }
                    }

                    diagnostics.push(diag);
                }
            }
            PatternType::AstGrep | PatternType::Structural => {
                // AST-grep patterns: match structural patterns in the AST
                // For now, a simple node-based matching
                diagnostics.extend(self.match_ast_grep(tree, source, file_path)?);
            }
        }

        Ok(diagnostics)
    }

    fn match_ast_grep(&self, tree: &Tree, source: &str, file_path: &Path) -> Result<Vec<Diagnostic>> {
        // Placeholder: In production, use ast-grep library for actual pattern matching
        let mut diagnostics = Vec::new();

        // Example: Look for specific node types
        let root = tree.root_node();
        let mut cursor = root.walk();

        self.visit_node(&root, source, file_path, &mut diagnostics);

        Ok(diagnostics)
    }

    fn visit_node(&self, node: &tree_sitter::Node, source: &str, file_path: &Path, diagnostics: &mut Vec<Diagnostic>) {
        // Simple recursive traversal
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            // Pattern matching logic would go here
            // For now, collect all nodes and match against pattern
            self.visit_node(&child, source, file_path, diagnostics);
        }
    }
}

/// Convert byte offset to line and column.
fn byte_offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    for (i, ch) in source.chars().enumerate() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_rule_creation() {
        let rule = StaticRule::new(
            "test-rule",
            "Test Rule",
            vec!["rust".to_string()],
            r"fn \w+\(\) \{\}",
            "Found empty function",
        );

        assert_eq!(rule.id, "test-rule");
        assert_eq!(rule.languages, vec!["rust"]);
        assert!(!rule.pattern.is_empty());
    }

    #[test]
    fn test_byte_offset_to_line_col() {
        let source = "line1\nline2\nline3";
        let (line, col) = byte_offset_to_line_col(source, 0);
        assert_eq!((line, col), (0, 0));

        let (line, col) = byte_offset_to_line_col(source, 6); // After 'line1\n'
        assert_eq!((line, col), (1, 0));

        let (line, col) = byte_offset_to_line_col(source, 12); // After 'line1\nline2\n'
        assert_eq!((line, col), (2, 0));
    }
}
