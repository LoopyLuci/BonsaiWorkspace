//! Core types and structures for UnixCC

use crate::language::Language;
use std::path::PathBuf;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

pub mod compilation_unit;
pub mod dependency_graph;
pub mod build_plan;

pub use compilation_unit::CompilationUnit;
pub use dependency_graph::DependencyGraph;
pub use build_plan::BuildPlan;

/// Compilation target
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CompileTarget {
    pub triple: String, // e.g., "x86_64-unknown-linux-gnu"
}

impl CompileTarget {
    pub fn new(triple: impl Into<String>) -> Self {
        Self {
            triple: triple.into(),
        }
    }

    /// Native target (current platform)
    pub fn native() -> Self {
        Self {
            triple: Self::get_native_triple(),
        }
    }

    /// Get the native target triple for current platform
    fn get_native_triple() -> String {
        #[cfg(target_arch = "x86_64")]
        #[cfg(target_os = "linux")]
        return "x86_64-unknown-linux-gnu".to_string();

        #[cfg(target_arch = "x86_64")]
        #[cfg(target_os = "windows")]
        return "x86_64-pc-windows-msvc".to_string();

        #[cfg(target_arch = "x86_64")]
        #[cfg(target_os = "macos")]
        return "x86_64-apple-darwin".to_string();

        #[cfg(target_arch = "aarch64")]
        #[cfg(target_os = "macos")]
        return "aarch64-apple-darwin".to_string();

        "unknown-unknown-unknown".to_string()
    }

    /// Get common targets
    pub fn common_targets() -> Vec<CompileTarget> {
        vec![
            CompileTarget::new("x86_64-unknown-linux-gnu"),
            CompileTarget::new("x86_64-pc-windows-msvc"),
            CompileTarget::new("x86_64-apple-darwin"),
            CompileTarget::new("aarch64-apple-darwin"),
            CompileTarget::new("aarch64-unknown-linux-gnu"),
            CompileTarget::new("wasm32-unknown-unknown"),
        ]
    }
}

impl std::fmt::Display for CompileTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.triple)
    }
}

/// Compilation output
#[derive(Debug, Clone)]
pub struct CompileResult {
    pub success: bool,
    pub language: Language,
    pub target: CompileTarget,
    pub artifacts: Vec<PathBuf>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub duration_ms: u128,
    pub output: String,
    pub timestamp: DateTime<Utc>,
}

impl CompileResult {
    pub fn new(language: Language, target: CompileTarget) -> Self {
        Self {
            success: true,
            language,
            target,
            artifacts: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            duration_ms: 0,
            output: String::new(),
            timestamp: Utc::now(),
        }
    }

    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }
}

/// Build statistics
#[derive(Debug, Clone)]
pub struct BuildStats {
    pub total_units: usize,
    pub compiled_units: usize,
    pub cached_units: usize,
    pub failed_units: usize,
    pub success_units: usize,
    pub total_duration_ms: u128,
    pub compilation_time_ms: u128,
    pub cache_time_ms: u128,
    pub linking_time_ms: u128,
    pub cache_hit_rate: f32,
    pub parallelization_factor: f32,
    pub duration_ms: u64,
    pub output: String,
}

impl BuildStats {
    pub fn new() -> Self {
        Self {
            total_units: 0,
            compiled_units: 0,
            cached_units: 0,
            failed_units: 0,
            success_units: 0,
            total_duration_ms: 0,
            compilation_time_ms: 0,
            cache_time_ms: 0,
            linking_time_ms: 0,
            cache_hit_rate: 0.0,
            parallelization_factor: 1.0,
            duration_ms: 0,
            output: String::new(),
        }
    }

    pub fn success_rate(&self) -> f32 {
        if self.total_units == 0 {
            100.0
        } else {
            ((self.total_units - self.failed_units) as f32 / self.total_units as f32) * 100.0
        }
    }

    pub fn error_count(&self) -> usize {
        self.failed_units
    }
}

impl Default for BuildStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_target_native() {
        let target = CompileTarget::native();
        assert!(!target.triple.is_empty());
    }

    #[test]
    fn test_compile_target_display() {
        let target = CompileTarget::new("x86_64-unknown-linux-gnu");
        assert_eq!(target.to_string(), "x86_64-unknown-linux-gnu");
    }

    #[test]
    fn test_compile_result() {
        let result = CompileResult::new(Language::Rust, CompileTarget::native());
        assert!(result.success);
        assert_eq!(result.error_count(), 0);
    }

    #[test]
    fn test_build_stats() {
        let stats = BuildStats::new();
        assert_eq!(stats.success_rate(), 100.0);
    }
}
