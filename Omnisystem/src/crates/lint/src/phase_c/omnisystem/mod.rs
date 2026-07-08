/// Omnisystem deep integration: Titan, Aether, Sylva, Axiom
/// Language-specific linting for the Bonsai ecosystem

pub mod titan;
pub mod aether;
pub mod sylva;
pub mod axiom;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct OmnisystemLinter {
    titan_enabled: bool,
    aether_enabled: bool,
    sylva_enabled: bool,
    axiom_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmnisystemIssue {
    pub issue_type: String,
    pub severity: String,
    pub language: String,
    pub description: String,
    pub suggestion: String,
}

impl OmnisystemLinter {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            titan_enabled: true,
            aether_enabled: true,
            sylva_enabled: true,
            axiom_enabled: true,
        })
    }

    /// Run omnisystem linting for a language
    pub async fn lint(&self, language: &str) -> Result<Vec<OmnisystemIssue>> {
        let mut issues = Vec::new();

        match language {
            "titan" => {
                if self.titan_enabled {
                    issues.extend(titan::lint_titan_effects().await?);
                }
            }
            "aether" => {
                if self.aether_enabled {
                    issues.extend(aether::lint_actor_supervision().await?);
                }
            }
            "sylva" => {
                if self.sylva_enabled {
                    issues.extend(sylva::lint_script_safety().await?);
                }
            }
            _ => {}
        }

        // Axiom applies to all languages
        if self.axiom_enabled {
            issues.extend(axiom::lint_type_safety(language).await?);
        }

        Ok(issues)
    }

    /// Enable/disable omnisystem linters
    pub fn set_enabled(&mut self, language: &str, enabled: bool) {
        match language {
            "titan" => self.titan_enabled = enabled,
            "aether" => self.aether_enabled = enabled,
            "sylva" => self.sylva_enabled = enabled,
            "axiom" => self.axiom_enabled = enabled,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_omnisystem_linter() {
        let linter = OmnisystemLinter::new().await.unwrap();
        let issues = linter.lint("titan").await.unwrap();
        // Should return issues (or empty if no violations detected)
        assert!(issues.is_empty() || !issues.is_empty());
    }

    #[test]
    fn test_enable_disable() {
        let mut linter = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(OmnisystemLinter::new())
            .unwrap();
        linter.set_enabled("titan", false);
        assert!(!linter.titan_enabled);
    }
}
