//! Repair pattern database

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairPattern {
    pub id: String,
    pub name: String,
    pub description: String,
    pub error_types: Vec<String>,
    pub fix_template: String,
    pub confidence: f64,
    pub test_cases: Vec<TestCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub input: String,
    pub expected_output: String,
}

pub struct PatternDatabase {
    patterns: Vec<RepairPattern>,
}

impl PatternDatabase {
    pub fn new() -> Self {
        let patterns = vec![
            // Unused variable pattern
            RepairPattern {
                id: "unused_var_prefix".to_string(),
                name: "Add underscore prefix".to_string(),
                description: "Prefix unused variables with underscore".to_string(),
                error_types: vec!["UnusedVariable".to_string()],
                fix_template: "let _{name} = {value};".to_string(),
                confidence: 0.95,
                test_cases: vec![
                    TestCase {
                        input: "let x = 5;".to_string(),
                        expected_output: "let _x = 5;".to_string(),
                    },
                ],
            },
            // Missing return pattern
            RepairPattern {
                id: "missing_return_add".to_string(),
                name: "Add return statement".to_string(),
                description: "Add return statement to function".to_string(),
                error_types: vec!["MissingReturn".to_string()],
                fix_template: "return {expr};".to_string(),
                confidence: 0.85,
                test_cases: vec![],
            },
            // Unused import pattern
            RepairPattern {
                id: "remove_unused_import".to_string(),
                name: "Remove or suppress import".to_string(),
                description: "Remove unused import or add #[allow] attribute".to_string(),
                error_types: vec!["UnusedImport".to_string()],
                fix_template: "#[allow(unused_imports)]\nuse {module};".to_string(),
                confidence: 0.90,
                test_cases: vec![],
            },
            // Null pointer check pattern
            RepairPattern {
                id: "add_null_check".to_string(),
                name: "Add null pointer check".to_string(),
                description: "Wrap dereference in null check".to_string(),
                error_types: vec!["NullPointerDereference".to_string()],
                fix_template: "if let Some(val) = ptr { /* use val */ }".to_string(),
                confidence: 0.75,
                test_cases: vec![],
            },
            // Buffer overflow pattern
            RepairPattern {
                id: "add_bounds_check".to_string(),
                name: "Add bounds checking".to_string(),
                description: "Check array bounds before access".to_string(),
                error_types: vec!["BufferOverflow".to_string()],
                fix_template: "if index < array.len() { /* access array[index] */ }".to_string(),
                confidence: 0.70,
                test_cases: vec![],
            },
        ];

        Self { patterns }
    }

    pub fn get_patterns_for_error(&self, error_type: &str) -> Vec<&RepairPattern> {
        self.patterns
            .iter()
            .filter(|p| p.error_types.contains(&error_type.to_string()))
            .collect()
    }

    pub fn get_pattern(&self, id: &str) -> Option<&RepairPattern> {
        self.patterns.iter().find(|p| p.id == id)
    }

    pub fn add_pattern(&mut self, pattern: RepairPattern) {
        self.patterns.push(pattern);
    }
}

impl Default for PatternDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_database_creation() {
        let db = PatternDatabase::new();
        assert!(!db.patterns.is_empty());
    }

    #[test]
    fn test_get_pattern() {
        let db = PatternDatabase::new();
        let pattern = db.get_pattern("unused_var_prefix");
        assert!(pattern.is_some());
    }

    #[test]
    fn test_get_patterns_for_error() {
        let db = PatternDatabase::new();
        let patterns = db.get_patterns_for_error("UnusedVariable");
        assert!(!patterns.is_empty());
    }
}
