pub mod static_rule;
pub mod native_rule;
pub mod ai_rule_gen;

use crate::diagnostics::{Diagnostic, Severity};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use tree_sitter::Tree;

pub use static_rule::StaticRule;
pub use native_rule::NativeRule;

/// Trait for rules that can be applied to parsed code.
pub trait Rule: Send + Sync {
    /// Unique identifier for this rule.
    fn id(&self) -> &str;

    /// Human-readable name.
    fn name(&self) -> &str;

    /// Supported languages (e.g., ["rust", "python"]).
    fn languages(&self) -> &[&str];

    /// Apply the rule to a parsed tree and return diagnostics.
    fn apply(&self, tree: &Tree, source: &str, file: &Path) -> Result<Vec<Diagnostic>>;
}

/// Rule confidence and metadata (from ETL).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMetadata {
    pub rule_id: String,
    pub confidence: f32,
    pub severity: Severity,
    pub enabled: bool,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl RuleMetadata {
    pub fn new(rule_id: String) -> Self {
        Self {
            rule_id,
            confidence: 0.75,
            severity: Severity::Warning,
            enabled: true,
            last_updated: chrono::Utc::now(),
        }
    }
}

/// Registry of all active rules with confidence tracking.
pub struct RuleRegistry {
    static_rules: Vec<Arc<StaticRule>>,
    native_rules: Vec<Arc<dyn NativeRule>>,
    metadata: Arc<RwLock<HashMap<String, RuleMetadata>>>,
}

impl RuleRegistry {
    pub fn new() -> Self {
        Self {
            static_rules: Vec::new(),
            native_rules: Vec::new(),
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load all rules from .bonsai/rules/ directory.
    pub fn load(root: &Path) -> Result<Self> {
        let mut registry = Self::new();

        let rules_dir = root.join(".bonsai").join("rules");
        if rules_dir.exists() {
            for entry in std::fs::read_dir(&rules_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map(|e| e == "yaml" || e == "yml").unwrap_or(false) {
                    match StaticRule::from_file(&path) {
                        Ok(rule) => {
                            let rule_id = rule.id.clone();
                            registry.add_static_rule(rule);
                            // Initialize metadata for this rule
                            registry.metadata.write().insert(rule_id, RuleMetadata::new(rule.id.clone()));
                        },
                        Err(e) => tracing::warn!("Failed to load rule {:?}: {:?}", path, e),
                    }
                }
            }
        }

        // TODO: Load native rules from plugins

        Ok(registry)
    }

    pub fn add_static_rule(&mut self, rule: StaticRule) {
        let rule_id = rule.id.clone();
        self.static_rules.push(Arc::new(rule));
        self.metadata.write().insert(rule_id, RuleMetadata::new(rule_id));
    }

    pub fn add_native_rule(&mut self, rule: Arc<dyn NativeRule>) {
        let rule_id = rule.id().to_string();
        self.native_rules.push(rule);
        self.metadata.write().insert(rule_id, RuleMetadata::new(rule_id));
    }

    /// Get all active rules for a given language.
    pub fn rules_for_language(&self, language: &str) -> Vec<Arc<dyn Rule>> {
        let metadata = self.metadata.read();
        let mut rules: Vec<Arc<dyn Rule>> = self.static_rules
            .iter()
            .filter(|r| {
                r.languages.contains(&language.to_string())
                    && metadata
                        .get(&r.id)
                        .map(|m| m.enabled)
                        .unwrap_or(true)
            })
            .map(|r| r.clone() as Arc<dyn Rule>)
            .collect();

        // Add native rules
        for nr in &self.native_rules {
            if nr.languages().contains(&language)
                && metadata
                    .get(nr.id())
                    .map(|m| m.enabled)
                    .unwrap_or(true)
            {
                // Wrap in a trait object
                // TODO: Proper trait object wrapping
            }
        }

        rules
    }

    /// Get rule metadata including confidence score.
    pub fn get_metadata(&self, rule_id: &str) -> Option<RuleMetadata> {
        self.metadata.read().get(rule_id).cloned()
    }

    /// Update rule confidence (called by ETL).
    pub fn update_confidence(&self, rule_id: &str, confidence: f32) -> Result<()> {
        let mut metadata = self.metadata.write();
        if let Some(meta) = metadata.get_mut(rule_id) {
            meta.confidence = confidence.clamp(0.0, 1.0);
            meta.last_updated = chrono::Utc::now();
            tracing::info!(
                "Updated rule {} confidence to {:.2}",
                rule_id,
                meta.confidence
            );
            Ok(())
        } else {
            Err(anyhow::anyhow!("Rule not found: {}", rule_id))
        }
    }

    /// Update rule severity (called by ETL).
    pub fn set_severity(&self, rule_id: &str, severity: Severity) -> Result<()> {
        let mut metadata = self.metadata.write();
        if let Some(meta) = metadata.get_mut(rule_id) {
            meta.severity = severity.clone();
            meta.last_updated = chrono::Utc::now();
            tracing::info!("Updated rule {} severity to {:?}", rule_id, meta.severity);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Rule not found: {}", rule_id))
        }
    }

    /// Enable/disable a rule (called by ETL).
    pub fn set_enabled(&self, rule_id: &str, enabled: bool) -> Result<()> {
        let mut metadata = self.metadata.write();
        if let Some(meta) = metadata.get_mut(rule_id) {
            meta.enabled = enabled;
            meta.last_updated = chrono::Utc::now();
            tracing::info!("Rule {} {}", rule_id, if enabled { "enabled" } else { "disabled" });
            Ok(())
        } else {
            Err(anyhow::anyhow!("Rule not found: {}", rule_id))
        }
    }

    /// Get all rule metadata.
    pub fn get_all_metadata(&self) -> HashMap<String, RuleMetadata> {
        self.metadata.read().clone()
    }
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = RuleRegistry::new();
        assert_eq!(registry.get_all_metadata().len(), 0);
    }

    #[test]
    fn test_rule_metadata() {
        let registry = RuleRegistry::new();
        registry.add_static_rule(StaticRule {
            id: "test-rule".to_string(),
            name: "Test Rule".to_string(),
            description: "A test rule".to_string(),
            languages: vec!["rust".to_string()],
            pattern: "test".to_string(),
            severity: Severity::Warning,
            message_template: "Test message".to_string(),
            examples: None,
        });

        let metadata = registry.get_metadata("test-rule");
        assert!(metadata.is_some());
        assert_eq!(metadata.unwrap().confidence, 0.75);
    }

    #[test]
    fn test_update_confidence() {
        let registry = RuleRegistry::new();
        registry.add_static_rule(StaticRule {
            id: "test-rule".to_string(),
            name: "Test Rule".to_string(),
            description: "A test rule".to_string(),
            languages: vec!["rust".to_string()],
            pattern: "test".to_string(),
            severity: Severity::Warning,
            message_template: "Test message".to_string(),
            examples: None,
        });

        registry.update_confidence("test-rule", 0.92).unwrap();
        let metadata = registry.get_metadata("test-rule").unwrap();
        assert_eq!(metadata.confidence, 0.92);
    }
}
