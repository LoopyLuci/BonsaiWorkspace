use crate::diagnostics::Diagnostic;
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

/// Trait for native Rust-based linting rules.
/// These are used for deep semantic checks that require access to the full symbol graph or type information.
#[async_trait]
pub trait NativeRule: Send + Sync {
    /// Unique identifier for this rule.
    fn id(&self) -> &str;

    /// Human-readable name.
    fn name(&self) -> &str;

    /// Supported languages.
    fn languages(&self) -> &[&str];

    /// Check a file and return diagnostics.
    async fn check(&self, file: &Path, source: &str) -> Result<Vec<Diagnostic>>;
}

/// Example: A rule that checks for functions with too many parameters.
pub struct ExcessiveParameterRule {
    max_params: usize,
}

impl ExcessiveParameterRule {
    pub fn new(max_params: usize) -> Self {
        Self { max_params }
    }
}

#[async_trait]
impl NativeRule for ExcessiveParameterRule {
    fn id(&self) -> &str {
        "excessive-parameters"
    }

    fn name(&self) -> &str {
        "Function has too many parameters"
    }

    fn languages(&self) -> &[&str] {
        &["rust", "python", "typescript", "javascript"]
    }

    async fn check(&self, file: &Path, _source: &str) -> Result<Vec<Diagnostic>> {
        // Placeholder: In production, parse and count function parameters
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_excessive_parameter_rule() -> Result<()> {
        let rule = ExcessiveParameterRule::new(5);
        assert_eq!(rule.id(), "excessive-parameters");
        assert_eq!(rule.languages(), &["rust", "python", "typescript", "javascript"]);

        let diags = rule.check(Path::new("test.rs"), "").await?;
        assert_eq!(diags.len(), 0); // Empty source, so no issues

        Ok(())
    }
}
