pub mod incremental;
pub mod parallel;
pub mod session;
pub mod persistent_cache;
pub mod dependency_graph;

use crate::diagnostics::{Diagnostic, LintSummary};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintConfig {
    /// Root directory of the workspace.
    pub root: PathBuf,

    /// Optional list of files to lint. If None, lint all discoverable files.
    pub target_files: Option<Vec<PathBuf>>,

    /// File patterns to exclude (glob patterns).
    pub exclude_patterns: Vec<String>,

    /// Minimum confidence threshold for accepting diagnostics (0.0–1.0).
    pub confidence_threshold: f32,

    /// Enable AI-powered false-positive filtering.
    pub ai_filtering: bool,

    /// Enable spellchecking and grammar checking.
    pub spell_check: bool,

    /// Number of parallel worker threads (0 = use all available cores).
    pub num_workers: usize,

    /// Enable incremental mode (only re-lint changed files).
    pub incremental: bool,

    /// Path to a previous lint database (for incremental diffing).
    pub previous_db: Option<PathBuf>,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            target_files: None,
            exclude_patterns: vec![],
            confidence_threshold: 0.7,
            ai_filtering: true,
            spell_check: true,
            num_workers: 0,
            incremental: true,
            previous_db: None,
        }
    }
}

/// The main linting engine.
pub struct LintEngine {
    config: LintConfig,
    db: incremental::LintDb,
    registry: crate::rules::RuleRegistry,
}

impl LintEngine {
    pub fn new(config: LintConfig) -> Result<Self> {
        let db = incremental::LintDb::new(config.root.clone());
        let registry = crate::rules::RuleRegistry::load(&config.root)?;

        Ok(Self { config, db, registry })
    }

    /// Run linting on the configured files.
    pub async fn lint(&self) -> Result<Vec<Diagnostic>> {
        let start = Instant::now();
        tracing::info!("Starting lint session on {:?}", self.config.root);

        // Determine files to lint
        let files = self.discover_files()?;
        tracing::info!("Discovered {} files to lint", files.len());

        // Parse and run rules in parallel
        let mut diagnostics = parallel::lint_files_parallel(
            &self.db,
            &files,
            &self.registry,
            self.config.num_workers,
        )?;

        // Filter by confidence threshold
        diagnostics.retain(|d| d.passes_threshold(self.config.confidence_threshold));

        // Optional: AI-powered false-positive filtering
        if self.config.ai_filtering {
            tracing::info!("Running AI-powered false-positive filtering");
            // TODO: Integrate with BonsAI for ML-based filtering
        }

        let duration = start.elapsed();
        tracing::info!("Lint completed in {:?}; found {} diagnostics", duration, diagnostics.len());

        // Emit summary to Universe
        self.emit_summary(&diagnostics, duration)?;

        Ok(diagnostics)
    }

    fn discover_files(&self) -> Result<Vec<PathBuf>> {
        if let Some(files) = &self.config.target_files {
            return Ok(files.clone());
        }

        let mut files = Vec::new();
        for entry in walkdir::WalkDir::new(&self.config.root)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                // Skip if matches exclude pattern
                let skip = self.config.exclude_patterns.iter().any(|pattern| {
                    // Simple glob matching
                    path.to_string_lossy().contains(pattern)
                });
                if !skip {
                    files.push(path.to_path_buf());
                }
            }
        }
        Ok(files)
    }

    fn emit_summary(&self, diagnostics: &[Diagnostic], duration: std::time::Duration) -> Result<()> {
        let mut by_severity = std::collections::HashMap::new();
        let mut by_rule = std::collections::HashMap::new();

        for diag in diagnostics {
            *by_severity.entry(diag.severity.to_string()).or_insert(0) += 1;
            *by_rule.entry(diag.rule_id.clone()).or_insert(0) += 1;
        }

        let summary = LintSummary {
            total_files: self.discover_files()?.len(),
            total_diagnostics: diagnostics.len(),
            by_severity,
            by_rule,
            duration_ms: duration.as_millis(),
        };

        tracing::info!("Lint summary: {:?}", summary);

        // TODO: Emit to Universe
        Ok(())
    }
}

/// A lint session for stateful interaction.
pub struct LintSession {
    config: LintConfig,
    db: incremental::LintDb,
    registry: crate::rules::RuleRegistry,
}

impl LintSession {
    pub fn new(config: LintConfig) -> Result<Self> {
        let db = incremental::LintDb::new(config.root.clone());
        let registry = crate::rules::RuleRegistry::load(&config.root)?;
        Ok(Self { config, db, registry })
    }

    /// Lint a single file (fast).
    pub fn lint_file(&self, path: &Path) -> Result<Vec<Diagnostic>> {
        let tree = self.db.parse_file(path)?;
        let lang = incremental::detect_language(path);

        let rules = self.registry.rules_for_language(&lang);
        let mut diagnostics = Vec::new();

        for rule in rules {
            diagnostics.extend(rule.apply(&tree)?);
        }

        Ok(diagnostics)
    }
}
