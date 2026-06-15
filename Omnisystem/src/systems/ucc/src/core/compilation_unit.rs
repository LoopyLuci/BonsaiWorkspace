//! Compilation unit - represents a single compilation target

use crate::language::Language;
use std::path::{Path, PathBuf};
use std::collections::HashSet;

/// A compilation unit represents a single compilable target (e.g., a crate, module, or binary)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilationUnit {
    pub id: String,
    pub name: String,
    pub language: Language,
    pub root_path: PathBuf,
    pub source_files: Vec<PathBuf>,
    pub dependencies: HashSet<String>, // IDs of dependent units
    pub is_library: bool,
    pub is_test: bool,
    pub estimated_duration_ms: u128,
}

impl std::hash::Hash for CompilationUnit {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl CompilationUnit {
    /// Create a new compilation unit
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        language: Language,
        root_path: PathBuf,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            language,
            root_path,
            source_files: Vec::new(),
            dependencies: HashSet::new(),
            is_library: false,
            is_test: false,
            estimated_duration_ms: 1000, // Default 1 second estimate
        }
    }

    /// Add a source file
    pub fn add_source_file(&mut self, path: impl AsRef<Path>) {
        self.source_files.push(path.as_ref().to_path_buf());
    }

    /// Add a dependency on another compilation unit
    pub fn add_dependency(&mut self, unit_id: impl Into<String>) {
        self.dependencies.insert(unit_id.into());
    }

    /// Check if this unit has no dependencies
    pub fn is_independent(&self) -> bool {
        self.dependencies.is_empty()
    }

    /// Get dependency count
    pub fn dependency_count(&self) -> usize {
        self.dependencies.len()
    }

    /// Set as library
    pub fn set_library(mut self) -> Self {
        self.is_library = true;
        self
    }

    /// Set as test
    pub fn set_test(mut self) -> Self {
        self.is_test = true;
        self
    }

    /// Set estimated duration
    pub fn set_estimated_duration(mut self, duration_ms: u128) -> Self {
        self.estimated_duration_ms = duration_ms;
        self
    }
}

/// Unit state during compilation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitState {
    /// Waiting for dependencies to compile
    Pending,
    /// Currently being compiled
    Compiling,
    /// Compilation succeeded
    Succeeded,
    /// Compilation failed
    Failed,
    /// Loaded from cache (didn't need to compile)
    CacheHit,
}

/// Unit compilation result
#[derive(Debug, Clone)]
pub struct UnitResult {
    pub unit_id: String,
    pub state: UnitState,
    pub duration_ms: u128,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub output: String,
    pub cache_hit: bool,
}

impl UnitResult {
    pub fn new(unit_id: impl Into<String>) -> Self {
        Self {
            unit_id: unit_id.into(),
            state: UnitState::Pending,
            duration_ms: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
            output: String::new(),
            cache_hit: false,
        }
    }

    pub fn success(&self) -> bool {
        matches!(self.state, UnitState::Succeeded | UnitState::CacheHit)
    }

    pub fn failed(&self) -> bool {
        self.state == UnitState::Failed
    }

    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compilation_unit_creation() {
        let unit = CompilationUnit::new("core", "Core Library", Language::Rust, PathBuf::from("."));
        assert_eq!(unit.id, "core");
        assert_eq!(unit.name, "Core Library");
        assert_eq!(unit.language, Language::Rust);
    }

    #[test]
    fn test_add_dependency() {
        let mut unit = CompilationUnit::new("main", "Main", Language::Rust, PathBuf::from("."));
        unit.add_dependency("core");
        assert_eq!(unit.dependency_count(), 1);
        assert!(!unit.is_independent());
    }

    #[test]
    fn test_is_independent() {
        let unit = CompilationUnit::new("standalone", "Standalone", Language::Rust, PathBuf::from("."));
        assert!(unit.is_independent());
    }

    #[test]
    fn test_unit_result() {
        let mut result = UnitResult::new("test_unit");
        result.state = UnitState::Succeeded;
        assert!(result.success());
        assert!(!result.failed());
    }
}
