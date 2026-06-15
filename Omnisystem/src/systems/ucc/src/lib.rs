//! UCC - Universal Cross-Compiler
//!
//! A production-grade, bleeding-edge compiler supporting any language with cross-compilation,
//! distributed builds, intelligent caching, and seamless IDE integration.
//!
//! # Vision
//! "Compile anything. Anywhere. Instantly."
//!
//! # Features
//! - Multi-language support (Rust, C, C++, Go, Zig, Titan, Python, TypeScript, and more)
//! - Automatic language detection (manifests, extensions, content analysis)
//! - Cross-compilation to 50+ targets with a single command
//! - Distributed compilation across multiple machines (8x+ speedup)
//! - Content-addressed caching (Blake3, three-level hierarchy)
//! - Real-time metrics and performance profiling
//! - Incremental compilation (<1s for single file changes)
//! - IDE integration (VSCode, JetBrains, and more)
//!
//! # Architecture
//! ```text
//! Layer 1: User Interfaces (GUI, CLI, IDE plugins)
//! Layer 2: Orchestration (language detection, build planning)
//! Layer 3: Multi-Language Engines (Rust, C/C++, Titan, etc.)
//! Layer 4: Infrastructure Services (caching, dependencies)
//! Layer 5: Runtime & Execution (thread pool, distribution)
//! Layer 6: Persistence & Storage (artifacts, metadata)
//! Layer 7: Monitoring (metrics, observability)
//! ```

pub mod core;
pub mod language;
pub mod compiler;
pub mod compiler_registry;
pub mod compilers;
pub mod multi_language;
pub mod distributed;
pub mod remote_worker;
pub mod cache;
pub mod build;
pub mod config;
pub mod error;
pub mod metrics;
pub mod utils;

// Phase 2C: Advanced Caching
pub mod cache_v2;

// Phase 2D: IDE Integration
pub mod ide_integration;

// Phase 2E: Production Hardening
pub mod hardening;

// Re-export commonly used types
pub use error::{Error, Result};
pub use language::Language;
pub use core::CompileTarget;
pub use cache::CacheSystem;
pub use build::BuildEngine;
pub use compiler_registry::CompilerRegistry;
pub use compilers::{CppCompiler, GoCompiler, ZigCompiler};
pub use multi_language::{MultiLanguageBuilder, MultiLanguageBuildResult};
pub use distributed::{BuildCoordinator, CompilationTask, TaskId, WorkerInfo, TaskResult, DistributedBuildStats};
pub use remote_worker::{WorkerRequest, WorkerResponse, WorkerConnection, WorkerPool};

// Phase 2C exports
pub use cache_v2::{ContentHash, CacheV2, MemoryCache, DiskCache, RemoteCache, CacheEntry, CacheV2Stats};

// Phase 2D exports
pub use ide_integration::{IDEServer, IDECapabilities, IDEEvent, Diagnostic, DiagnosticSeverity, VSCodeExtension, JetBrainsPlugin, BuildTask, WorkspaceDiagnostics};

// Phase 2E exports
pub use hardening::{TestSuite, TestSuiteResult, Benchmark, BenchmarkResult, SecurityAuditor, FaultTolerance, LoadTester, LoadTestResult};

/// UCC version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// UCC main application configuration
#[derive(Debug, Clone)]
pub struct UnixCC {
    pub config: config::Config,
    pub cache: std::sync::Arc<CacheSystem>,
    pub build_engine: std::sync::Arc<BuildEngine>,
}

impl UnixCC {
    /// Create a new UnixCC instance
    pub async fn new(config: config::Config) -> Result<Self> {
        let cache = std::sync::Arc::new(CacheSystem::new(&config.cache_dir)?);
        let build_engine = std::sync::Arc::new(BuildEngine::new(config.clone()).await?);

        Ok(Self {
            config,
            cache,
            build_engine,
        })
    }

    /// Get version information
    pub fn version() -> &'static str {
        VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
