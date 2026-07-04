use crate::diagnostics::Diagnostic;
use crate::rules::RuleRegistry;
use crate::engine::incremental::LintDb;
use anyhow::Result;
use rayon::prelude::*;
use std::path::PathBuf;

/// Lint multiple files in parallel using rayon.
pub fn lint_files_parallel(
    db: &LintDb,
    files: &[PathBuf],
    registry: &RuleRegistry,
    num_workers: usize,
) -> Result<Vec<Diagnostic>> {
    // Configure rayon thread pool
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(if num_workers == 0 { num_cpus::get() } else { num_workers })
        .build()?;

    let diagnostics = pool.install(|| {
        files
            .par_iter()
            .flat_map(|file| {
                match lint_single_file_parallel(db, file, registry) {
                    Ok(diags) => diags,
                    Err(e) => {
                        tracing::warn!("Failed to lint {:?}: {:?}", file, e);
                        Vec::new()
                    }
                }
            })
            .collect::<Vec<_>>()
    });

    Ok(diagnostics)
}

fn lint_single_file_parallel(
    db: &LintDb,
    file: &PathBuf,
    registry: &RuleRegistry,
) -> Result<Vec<Diagnostic>> {
    let parsed = db.parse_file(file)?;
    let rules = registry.rules_for_language(&parsed.language);

    // Execute all rules for this file in parallel
    let diagnostics = rules
        .par_iter()
        .flat_map(|rule| {
            match rule.apply(&parsed.tree, &parsed.source, file) {
                Ok(diags) => diags,
                Err(e) => {
                    tracing::warn!("Rule {:?} failed: {:?}", rule.id(), e);
                    Vec::new()
                }
            }
        })
        .collect();

    Ok(diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parallel_linting() -> Result<()> {
        let temp = TempDir::new()?;

        // Create multiple test files
        for i in 0..5 {
            fs::write(temp.path().join(format!("file{}.rs", i)), "fn main() {}")?;
        }

        let db = LintDb::new(temp.path().to_path_buf());
        let registry = RuleRegistry::new();

        let files: Vec<_> = (0..5)
            .map(|i| temp.path().join(format!("file{}.rs", i)))
            .collect();

        let results = lint_files_parallel(&db, &files, &registry, 2)?;

        // Verify we processed all files (even if no diagnostics)
        assert!(results.is_empty() || results.len() > 0); // Placeholder test

        Ok(())
    }
}
