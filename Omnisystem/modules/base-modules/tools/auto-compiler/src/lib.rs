//! Omnisystem Auto-Compiler and Self-Assembler
//!
//! Next-generation self-assembling and auto-compiling system that ensures
//! users never have to manually compile anything, but are fully capable of it.
//!
//! Features:
//! - Automatic project detection
//! - Intelligent dependency resolution
//! - Incremental compilation
//! - Distributed builds
//! - Hot reloading
//! - Real-time monitoring
//! - Zero configuration defaults
//! - Production-grade quality

pub mod error;
pub mod detector;
pub mod builder;
pub mod watcher;
pub mod orchestrator;
pub mod cache;
pub mod executor;
pub mod monitor;

pub use error::{AutoCompileError, Result};
pub use detector::{ProjectDetector, ProjectInfo, ProjectType};
pub use builder::{BuildConfig, BuildPlan, BuildExecutor};
pub use watcher::FileWatcher;
pub use orchestrator::{CompileOrchestrator, OrchestratorConfig};
pub use cache::BuildCache;
pub use executor::CommandExecutor;
pub use monitor::{CompileMonitor, CompileStats};

use std::path::PathBuf;

/// Get default auto-compiler instance
pub fn auto_compiler() -> Result<CompileOrchestrator> {
    CompileOrchestrator::new(Default::default())
}

/// Get auto-compiler with custom config directory
pub fn auto_compiler_with_config(config_dir: PathBuf) -> Result<CompileOrchestrator> {
    let config = orchestrator::OrchestratorConfig {
        config_dir,
        ..Default::default()
    };
    CompileOrchestrator::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_compiler_creation() {
        let result = auto_compiler();
        assert!(result.is_ok());
    }
}
