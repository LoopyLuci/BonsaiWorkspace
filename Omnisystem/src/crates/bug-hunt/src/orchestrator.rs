/// Main orchestrator for coordinating all analysis engines.

use anyhow::Result;
use log::{info, debug};
use std::path::Path;
use std::sync::Arc;

use crate::analyzer::LanguageAnalyzer;
use crate::cache::ScanCache;
use crate::finding::{Finding, ScanReport, ScanSummary};
use crate::engines::RustStaticLintAnalyzer;

/// Main orchestrator that coordinates all analysis engines.
pub struct BugHuntOrchestrator {
    cache: ScanCache,
    repo_path: std::path::PathBuf,
    analyzers: Vec<Arc<dyn LanguageAnalyzer>>,
}

impl BugHuntOrchestrator {
    /// Create a new orchestrator.
    pub fn new(cache_dir: std::path::PathBuf, repo_path: std::path::PathBuf) -> Result<Self> {
        let cache = ScanCache::new(cache_dir)?;

        // Initialize analyzers for supported languages
        let mut analyzers: Vec<Arc<dyn LanguageAnalyzer>> = Vec::new();

        // Rust analyzer
        analyzers.push(Arc::new(RustStaticLintAnalyzer::new(repo_path.clone())));

        // TODO: Add more analyzers (TypeScript, Python, Go, etc.)

        Ok(Self {
            cache,
            repo_path,
            analyzers,
        })
    }

    /// Run a full scan of the repository.
    pub async fn scan_full(&mut self) -> Result<ScanReport> {
        info!("Starting full repository scan");

        let mut all_findings = Vec::new();

        // Run all analyzers
        for analyzer in &self.analyzers {
            debug!("Running analyzer: {}", analyzer.name());
            match analyzer.analyze_repo(&self.repo_path).await {
                Ok(findings) => {
                    info!(
                        "Analyzer {} found {} issues",
                        analyzer.name(),
                        findings.len()
                    );
                    all_findings.extend(findings);
                }
                Err(e) => {
                    debug!("Analyzer {} failed: {}", analyzer.name(), e);
                    // Continue with other analyzers even if one fails
                }
            }
        }

        // Cache results
        let repo_name = self
            .repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        self.cache.put(self.repo_path.clone(), all_findings.clone())?;
        self.cache.save()?;

        let summary = ScanSummary::from_findings(&all_findings, repo_name);

        Ok(ScanReport {
            summary,
            issues: all_findings,
        })
    }

    /// Run an incremental scan (only changed files).
    pub async fn scan_incremental(&mut self) -> Result<ScanReport> {
        info!("Starting incremental repository scan");

        // For now, fall back to full scan
        // TODO: Implement proper incremental scanning with file change detection
        self.scan_full().await
    }

    /// Get the number of cached entries.
    pub fn cache_size(&self) -> (usize, usize) {
        self.cache.stats()
    }

    /// Clear the cache.
    pub fn clear_cache(&mut self) -> Result<()> {
        self.cache.clear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestrator_creation() -> Result<()> {
        let temp_dir = std::env::temp_dir().join("bonsai_orchestrator_test");
        let repo_dir = std::env::temp_dir().join("bonsai_test_repo");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::remove_dir_all(&repo_dir);
        std::fs::create_dir_all(&repo_dir)?;

        let orchestrator = BugHuntOrchestrator::new(temp_dir, repo_dir)?;
        assert_eq!(orchestrator.analyzers.len(), 1); // At least Rust analyzer
        Ok(())
    }
}
