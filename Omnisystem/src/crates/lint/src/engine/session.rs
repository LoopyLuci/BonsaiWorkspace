use crate::diagnostics::Diagnostic;
use crate::engine::{LintConfig, LintDb};
use crate::rules::RuleRegistry;
use anyhow::Result;
use std::path::Path;

/// A stateful linting session for interactive use (e.g., IDE plugin).
pub struct InteractiveSession {
    config: LintConfig,
    db: LintDb,
    registry: RuleRegistry,
}

impl InteractiveSession {
    pub fn new(config: LintConfig) -> Result<Self> {
        let db = LintDb::new(config.root.clone());
        let registry = RuleRegistry::load(&config.root)?;
        Ok(Self { config, db, registry })
    }

    /// Lint a single file quickly (no heavy initialization).
    pub fn lint_file(&self, path: &Path) -> Result<Vec<Diagnostic>> {
        let parsed = self.db.parse_file(path)?;
        let rules = self.registry.rules_for_language(&parsed.language);

        let mut diagnostics = Vec::new();
        for rule in rules {
            diagnostics.extend(rule.apply(&parsed.tree, &parsed.source, &parsed.path)?);
        }

        // Filter by confidence
        diagnostics.retain(|d| d.passes_threshold(self.config.confidence_threshold));

        Ok(diagnostics)
    }

    /// Get the list of active rules for a language.
    pub fn rules_for_language(&self, language: &str) -> Vec<String> {
        self.registry.rules_for_language(language)
            .iter()
            .map(|r| r.id().to_string())
            .collect()
    }

    /// Reload the rule registry (useful if .bonsai/rules/ changed).
    pub fn reload_rules(&mut self) -> Result<()> {
        self.registry = RuleRegistry::load(&self.config.root)?;
        Ok(())
    }

    /// Clear caches (for testing or memory pressure).
    pub fn clear_caches(&self) {
        self.db.clear_cache();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_interactive_session() -> Result<()> {
        let temp = TempDir::new()?;
        fs::write(temp.path().join("test.rs"), "fn main() {}")?;

        let config = LintConfig {
            root: temp.path().to_path_buf(),
            ..Default::default()
        };

        let session = InteractiveSession::new(config)?;
        let diags = session.lint_file(&temp.path().join("test.rs"))?;

        // Should lint without errors
        println!("Diagnostics: {:?}", diags);
        Ok(())
    }
}
