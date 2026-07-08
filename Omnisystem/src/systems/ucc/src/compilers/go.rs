//! Go Compiler - go build integration
//!
//! Compiles Go projects using the native `go build` command.
//! Supports module detection (go.mod), cross-compilation via GOOS/GOARCH.

use crate::error::Result;
use crate::compiler::LanguageCompiler;
use crate::core::{CompileTarget, CompileResult};
use crate::language::Language;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Go Compiler using native `go build`
pub struct GoCompiler {
    go_path: Option<PathBuf>,
    project_root: PathBuf,
}

impl GoCompiler {
    /// Create a new Go compiler instance
    pub fn new(project_root: PathBuf) -> Self {
        let go_path = which::which("go").ok();

        Self {
            go_path,
            project_root,
        }
    }

    /// Get GOOS value for target
    fn get_goos(target: &CompileTarget) -> Option<&'static str> {
        match target.triple.as_str() {
            triple if triple.contains("linux") => Some("linux"),
            triple if triple.contains("darwin") => Some("darwin"),
            triple if triple.contains("windows") => Some("windows"),
            triple if triple.contains("freebsd") => Some("freebsd"),
            _ => None,
        }
    }

    /// Get GOARCH value for target
    fn get_goarch(target: &CompileTarget) -> Option<&'static str> {
        match target.triple.as_str() {
            triple if triple.contains("x86_64") => Some("amd64"),
            triple if triple.contains("aarch64") => Some("arm64"),
            triple if triple.contains("i686") => Some("386"),
            triple if triple.contains("arm") => Some("arm"),
            _ => None,
        }
    }

    /// Check if go.mod exists (Go module project)
    fn has_go_mod(&self) -> bool {
        self.project_root.join("go.mod").exists()
    }
}

impl Default for GoCompiler {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

impl LanguageCompiler for GoCompiler {
    async fn compile(
        &self,
        _sources: &[&Path],
        target: &CompileTarget,
    ) -> Result<CompileResult> {
        let go_path = self.go_path.clone().ok_or_else(|| {
            crate::error::Error::CompilerNotFound {
                compiler: "go".to_string(),
                language: "Go".to_string(),
            }
        })?;

        let project_root = self.project_root.clone();
        let target_clone = target.clone();

        let (output, proj_root) = tokio::task::spawn_blocking(move || {
            let mut cmd = Command::new(&go_path);

            cmd.current_dir(&project_root);
            cmd.arg("build");

            // Set target environment if cross-compiling
            if let Some(goos) = Self::get_goos(&target_clone) {
                cmd.env("GOOS", goos);
            }

            if let Some(goarch) = Self::get_goarch(&target_clone) {
                cmd.env("GOARCH", goarch);
            }

            // Add verbose flag for more info
            cmd.arg("-v");

            (cmd.output(), project_root)
        })
        .await
        .map_err(|e| crate::error::Error::Config(format!("Failed to spawn go: {}", e)))?;

        let output = output
            .map_err(|e| crate::error::Error::Config(format!("Go invocation failed: {}", e)))?;

        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let mut result = CompileResult::new(Language::Go, target.clone());
        result.output = format!("{}\n{}", stdout, stderr);
        result.success = success;

        if !success {
            let errors: Vec<String> = stderr
                .lines()
                .filter(|line| line.contains("error"))
                .map(|s| s.to_string())
                .collect();
            result.errors = errors;
        } else {
            // Go build creates a binary in the project root
            result.artifacts.push(proj_root.join("main"));
        }

        Ok(result)
    }

    fn check_availability(&self) -> Result<()> {
        if self.go_path.is_some() {
            Ok(())
        } else {
            Err(crate::error::Error::Config(
                "Go compiler not found in PATH".to_string(),
            ))
        }
    }

    fn get_version(&self) -> Result<String> {
        let go_path = self.go_path.clone().ok_or_else(|| {
            crate::error::Error::Config("Go not found".to_string())
        })?;

        let output = Command::new(&go_path)
            .arg("version")
            .output()
            .map_err(|e| crate::error::Error::Config(format!("Failed to get Go version: {}", e)))?;

        Ok(String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_compiler_creation() {
        let compiler = GoCompiler::new(PathBuf::from("."));
        // go should be available in most environments
        let _ = compiler.check_availability();
    }

    #[test]
    fn test_goos_detection() {
        let target_linux = CompileTarget::new("x86_64-unknown-linux-gnu");
        assert_eq!(GoCompiler::get_goos(&target_linux), Some("linux"));

        let target_mac = CompileTarget::new("x86_64-apple-darwin");
        assert_eq!(GoCompiler::get_goos(&target_mac), Some("darwin"));
    }

    #[test]
    fn test_goarch_detection() {
        let target_64 = CompileTarget::new("x86_64-unknown-linux-gnu");
        assert_eq!(GoCompiler::get_goarch(&target_64), Some("amd64"));

        let target_arm = CompileTarget::new("aarch64-unknown-linux-gnu");
        assert_eq!(GoCompiler::get_goarch(&target_arm), Some("arm64"));
    }
}
