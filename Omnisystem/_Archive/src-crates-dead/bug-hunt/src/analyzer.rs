/// Trait for language-specific analyzers.

use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

use crate::finding::Finding;

/// Base trait for all analyzers.
#[async_trait]
pub trait LanguageAnalyzer: Send + Sync {
    /// Name of the analyzer (e.g., "rust_clippy", "eslint").
    fn name(&self) -> &str;

    /// File extensions this analyzer handles (e.g., [".rs", ".toml"]).
    fn supported_extensions(&self) -> Vec<&str>;

    /// Analyze a single file.
    async fn analyze_file(&self, file_path: &Path) -> Result<Vec<Finding>>;

    /// Analyze an entire repository.
    async fn analyze_repo(&self, repo_path: &Path) -> Result<Vec<Finding>>;
}
