//! Runtime types and versions

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeType {
    Rust,
    Python,
    Go,
    Java,
    Kotlin,
    CppLlvm,
    CSharp,
    Swift,
    Ruby,
    Php,
    Node,
    Scala,
    R,
    Clojure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeVersion {
    pub runtime_type: RuntimeType,
    pub version: String,
    pub url: String,
    pub checksum_sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runtime {
    pub runtime_type: RuntimeType,
    pub version: String,
    pub path: std::path::PathBuf,
    pub executable: std::path::PathBuf,
    pub installed_at: chrono::DateTime<chrono::Utc>,
}

impl RuntimeType {
    pub fn name(&self) -> &'static str {
        match self {
            RuntimeType::Rust => "rust",
            RuntimeType::Python => "python",
            RuntimeType::Go => "go",
            RuntimeType::Java => "java",
            RuntimeType::Kotlin => "kotlin",
            RuntimeType::CppLlvm => "llvm",
            RuntimeType::CSharp => "dotnet",
            RuntimeType::Swift => "swift",
            RuntimeType::Ruby => "ruby",
            RuntimeType::Php => "php",
            RuntimeType::Node => "node",
            RuntimeType::Scala => "scala",
            RuntimeType::R => "r",
            RuntimeType::Clojure => "clojure",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            RuntimeType::Rust => "Rust programming language",
            RuntimeType::Python => "Python interpreter",
            RuntimeType::Go => "Go programming language",
            RuntimeType::Java => "Java Virtual Machine",
            RuntimeType::Kotlin => "Kotlin compiler",
            RuntimeType::CppLlvm => "LLVM C/C++ compiler",
            RuntimeType::CSharp => ".NET runtime",
            RuntimeType::Swift => "Swift compiler",
            RuntimeType::Ruby => "Ruby interpreter",
            RuntimeType::Php => "PHP interpreter",
            RuntimeType::Node => "Node.js runtime",
            RuntimeType::Scala => "Scala compiler",
            RuntimeType::R => "R statistical environment",
            RuntimeType::Clojure => "Clojure compiler",
        }
    }

    pub fn all() -> &'static [RuntimeType] {
        &[
            RuntimeType::Rust,
            RuntimeType::Python,
            RuntimeType::Go,
            RuntimeType::Java,
            RuntimeType::Kotlin,
            RuntimeType::CppLlvm,
            RuntimeType::CSharp,
            RuntimeType::Swift,
            RuntimeType::Ruby,
            RuntimeType::Php,
            RuntimeType::Node,
            RuntimeType::Scala,
            RuntimeType::R,
            RuntimeType::Clojure,
        ]
    }
}

impl std::fmt::Display for RuntimeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_types() {
        assert_eq!(RuntimeType::Rust.name(), "rust");
        assert_eq!(RuntimeType::Python.name(), "python");
        assert_eq!(RuntimeType::Go.name(), "go");
    }

    #[test]
    fn test_all_runtimes() {
        assert_eq!(RuntimeType::all().len(), 14);
    }
}
