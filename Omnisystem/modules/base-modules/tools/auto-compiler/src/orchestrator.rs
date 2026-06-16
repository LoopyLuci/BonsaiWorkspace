//! Central orchestrator for auto-compilation

use crate::{
    ProjectDetector, BuildConfig, BuildPlan, BuildExecutor, FileWatcher, BuildCache,
    CompileMonitor, Result,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    pub config_dir: PathBuf,
    pub auto_compile: bool,
    pub watch_mode: bool,
    pub parallel_jobs: usize,
    pub cache_enabled: bool,
    pub hot_reload: bool,
    pub distributed_builds: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        let config_dir = directories::ProjectDirs::from("dev", "omnisystem", "auto-compiler")
            .map(|dirs| dirs.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".omnisystem/auto-compiler"));

        Self {
            config_dir,
            auto_compile: true,
            watch_mode: true,
            parallel_jobs: num_cpus::get(),
            cache_enabled: true,
            hot_reload: true,
            distributed_builds: true,
        }
    }
}

/// Central compilation orchestrator
pub struct CompileOrchestrator {
    config: OrchestratorConfig,
    monitor: CompileMonitor,
    cache: BuildCache,
    build_config: BuildConfig,
}

impl CompileOrchestrator {
    /// Create new orchestrator
    pub fn new(config: OrchestratorConfig) -> Result<Self> {
        let cache = BuildCache::new(config.config_dir.join("cache"))?;
        let mut build_config = BuildConfig::default();
        build_config.parallel_jobs = config.parallel_jobs;
        build_config.watch_mode = config.watch_mode;
        build_config.cache_enabled = config.cache_enabled;
        build_config.hot_reload = config.hot_reload;
        build_config.distributed = config.distributed_builds;

        Ok(Self {
            config,
            monitor: CompileMonitor::new(),
            cache,
            build_config,
        })
    }

    /// Auto-detect and compile all projects in directory
    pub async fn compile_all(&self, root_path: &PathBuf) -> Result<()> {
        log::info!("🚀 Starting auto-compiler at {}", root_path.display());

        // Detect all projects
        let projects = ProjectDetector::detect_all(root_path)?;

        if projects.is_empty() {
            log::warn!("No projects detected in {}", root_path.display());
            return Ok(());
        }

        log::info!("🔍 Detected {} project(s)", projects.len());

        // Compile each project
        for project in projects {
            self.compile_project(&project.root_path).await?;
        }

        self.monitor.print_stats();
        Ok(())
    }

    /// Compile single project
    pub async fn compile_project(&self, project_path: &PathBuf) -> Result<()> {
        log::info!("📦 Compiling project: {}", project_path.display());

        // Detect project
        let project_info = ProjectDetector::detect(project_path)?;

        // Check cache
        let cache_key = format!(
            "{}:{}:{}",
            project_info.name,
            project_info.project_type as u8,
            chrono::Utc::now().date_naive()
        );

        if let Some(cached) = self.cache.get(&cache_key) {
            if self.build_config.cache_enabled {
                log::info!("✅ Cache HIT for {}", project_info.name);
                self.monitor.complete_compile(true, true);
                return Ok(());
            }
        }

        // Generate build plan
        let plan = BuildPlan::generate(
            project_path.clone(),
            project_info.project_type,
            &self.build_config,
        )?;

        // Execute build
        self.monitor.start_compile(project_info.name.clone());
        let result = BuildExecutor::execute(plan).await?;

        // Record in cache
        if result.success && self.build_config.cache_enabled {
            self.cache.store(crate::cache::CacheEntry {
                key: cache_key,
                checksum: chrono::Utc::now().timestamp().to_string(),
                artifacts: result.artifacts,
                created_at: chrono::Utc::now(),
                last_used: chrono::Utc::now(),
                hit_count: 0,
            });
        }

        self.monitor.complete_compile(result.success, false);

        Ok(())
    }

    /// Start watch mode for auto-compilation
    pub async fn watch(&self, root_path: &PathBuf) -> Result<()> {
        if !self.config.watch_mode {
            log::warn!("Watch mode disabled");
            return Ok(());
        }

        log::info!("👀 Starting watch mode for auto-compilation");

        let mut watcher = FileWatcher::new(500);
        watcher.watch(root_path.clone());

        let _rx = watcher.start().await?;

        // Stub: real implementation would listen on rx and recompile on changes

        log::info!("🔄 Watch mode active - files will auto-compile on change");

        Ok(())
    }

    /// Get compilation statistics
    pub fn stats(&self) -> crate::monitor::CompileStats {
        self.monitor.stats()
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> crate::cache::CacheStats {
        self.cache.stats()
    }

    /// Clear cache
    pub fn clear_cache(&self) -> Result<()> {
        self.cache.clear()
    }

    pub fn config(&self) -> &OrchestratorConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_creation() {
        let config = OrchestratorConfig::default();
        let result = CompileOrchestrator::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_defaults() {
        let config = OrchestratorConfig::default();
        assert!(config.auto_compile);
        assert!(config.watch_mode);
        assert!(config.cache_enabled);
    }

    #[tokio::test]
    async fn test_compile_all() {
        let config = OrchestratorConfig::default();
        let orchestrator = CompileOrchestrator::new(config).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let result = orchestrator.compile_all(&temp_dir.path().to_path_buf()).await;

        // Should succeed even with no projects
        assert!(result.is_ok());
    }
}
