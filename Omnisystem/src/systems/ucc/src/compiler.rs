//! Compiler module - language-specific compilers

use crate::error::Result;
use crate::language::Language;
use crate::core::{CompileTarget, CompileResult};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Trait for language-specific compilers
pub trait LanguageCompiler: Send + Sync {
    /// Compile sources
    async fn compile(
        &self,
        sources: &[&Path],
        target: &CompileTarget,
    ) -> Result<CompileResult>;

    /// Check if compiler is available
    fn check_availability(&self) -> Result<()>;

    /// Get compiler version
    fn get_version(&self) -> Result<String>;
}

/// Rust compiler integration
pub struct RustCompiler {
    project_root: PathBuf,
}

impl RustCompiler {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    /// Find the Cargo.toml for the project
    fn find_cargo_manifest(&self) -> Result<PathBuf> {
        let manifest = self.project_root.join("Cargo.toml");
        if manifest.exists() {
            Ok(manifest)
        } else {
            Err(crate::error::Error::Config(
                format!("No Cargo.toml found in {}", self.project_root.display()),
            ))
        }
    }

    /// Invoke cargo build with proper arguments
    async fn invoke_cargo(&self, target: &CompileTarget, release: bool) -> Result<CompileResult> {
        let mut cmd = Command::new("cargo");

        cmd.current_dir(&self.project_root);

        if release {
            cmd.arg("build").arg("--release");
        } else {
            cmd.arg("build");
        }

        // Add target triple if not native
        let target_triple = &target.triple;
        if target_triple != "x86_64-pc-windows-msvc"
            && target_triple != "x86_64-unknown-linux-gnu"
            && target_triple != "x86_64-apple-darwin" {
            cmd.arg("--target").arg(target_triple);
        }

        let output = tokio::task::spawn_blocking(move || cmd.output())
            .await
            .map_err(|e| crate::error::Error::Config(format!("Failed to spawn cargo: {}", e)))?
            .map_err(|e| crate::error::Error::CompilationFailed {
                language: "Rust".to_string(),
                message: format!("Cargo invocation failed: {}", e),
                output: String::new(),
            })?;

        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let mut result = CompileResult::new(Language::Rust, target.clone());
        result.output = format!("{}\n{}", stdout, stderr);
        result.success = success;

        if !success {
            // Parse errors from compiler output
            let errors: Vec<String> = stderr
                .lines()
                .filter(|line| line.contains("error"))
                .map(|s| s.to_string())
                .collect();
            result.errors = errors;
        }

        Ok(result)
    }
}

impl LanguageCompiler for RustCompiler {
    async fn compile(
        &self,
        _sources: &[&Path],
        target: &CompileTarget,
    ) -> Result<CompileResult> {
        self.find_cargo_manifest()?;
        self.invoke_cargo(target, false).await
    }

    fn check_availability(&self) -> Result<()> {
        let output = Command::new("cargo")
            .arg("--version")
            .output()
            .map_err(|e| crate::error::Error::Config(format!("cargo not found: {}", e)))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(crate::error::Error::Config(
                "cargo not in PATH or not executable".to_string(),
            ))
        }
    }

    fn get_version(&self) -> Result<String> {
        let output = Command::new("cargo")
            .arg("--version")
            .output()
            .map_err(|e| crate::error::Error::Config(format!("Failed to get cargo version: {}", e)))?;

        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(version)
    }
}

impl Default for RustCompiler {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}
