//! Build engine - orchestrates compilation

use crate::error::Result;
use crate::config::Config;
use crate::core::BuildStats;
use crate::compiler::{LanguageCompiler, RustCompiler};
use std::time::Instant;

/// Build engine coordinator
#[derive(Debug)]
pub struct BuildEngine {
    pub config: Config,
}

impl BuildEngine {
    /// Create a new build engine
    pub async fn new(config: Config) -> Result<Self> {
        Ok(Self { config })
    }

    /// Build a project (auto-detect and compile)
    pub async fn build(&self) -> Result<BuildStats> {
        let start = Instant::now();

        // For now, assume Rust projects (Phase 2: multi-language detection)
        let compiler = RustCompiler::new(self.config.project_root.clone());

        // Check if compiler is available
        compiler.check_availability()?;

        // Compile
        let result = compiler.compile(&[], &self.config.target).await?;

        let duration = start.elapsed().as_millis() as u64;

        let mut stats = BuildStats::new();
        stats.total_units = 1;
        stats.success_units = if result.success { 1 } else { 0 };
        stats.failed_units = if !result.success { 1 } else { 0 };
        stats.duration_ms = duration;
        stats.output = result.output.clone();

        Ok(stats)
    }

    /// Clean build artifacts
    pub async fn clean(&self) -> Result<()> {
        use std::process::Command;

        let output = tokio::task::spawn_blocking({
            let root = self.config.project_root.clone();
            move || {
                Command::new("cargo")
                    .arg("clean")
                    .current_dir(&root)
                    .output()
            }
        })
        .await
        .map_err(|e| crate::error::Error::Config(format!("Failed to spawn cargo clean: {}", e)))?
        .map_err(|e| crate::error::Error::Config(format!("Cargo clean failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::error::Error::Config(format!("Clean failed: {}", stderr)));
        }

        Ok(())
    }

    /// Perform incremental build
    pub async fn incremental_build(&self) -> Result<BuildStats> {
        // For now, same as regular build (Phase 2: implement caching)
        self.build().await
    }
}
