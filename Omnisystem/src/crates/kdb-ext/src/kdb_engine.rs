use serde::{Deserialize, Serialize};
use std::collections::Vec;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityPattern {
    pub vulnerability_type: String,
    pub cve: Option<String>,
    pub description: String,
}

pub struct KnowledgeDatabase {
    patterns: Vec<VulnerabilityPattern>,
}

impl KnowledgeDatabase {
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    pub fn add_pattern(&mut self, pattern: VulnerabilityPattern) {
        self.patterns.push(pattern);
    }

    pub fn search(&self, query: &str) -> Vec<VulnerabilityPattern> {
        self.patterns
            .iter()
            .filter(|p| {
                p.vulnerability_type.to_lowercase().contains(&query.to_lowercase())
                    || p.description.to_lowercase().contains(&query.to_lowercase())
            })
            .cloned()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.patterns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
}

impl Default for KnowledgeDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kdb_creation() {
        let db = KnowledgeDatabase::new();
        assert!(db.is_empty());
    }

    #[test]
    fn test_add_pattern() {
        let mut db = KnowledgeDatabase::new();
        let pattern = VulnerabilityPattern {
            vulnerability_type: "XSS".to_string(),
            cve: None,
            description: "Cross-site scripting".to_string(),
        };
        db.add_pattern(pattern);
        assert_eq!(db.len(), 1);
    }

    #[test]
    fn test_search_patterns() {
        let mut db = KnowledgeDatabase::new();
        db.add_pattern(VulnerabilityPattern {
            vulnerability_type: "SQL Injection".to_string(),
            cve: Some("CVE-2024-0001".to_string()),
            description: "Database injection".to_string(),
        });
        let results = db.search("SQL");
        assert_eq!(results.len(), 1);
    }
}
