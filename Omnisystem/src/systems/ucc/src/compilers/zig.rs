//! Zig Compiler - zig build integration
//!
//! Compiles Zig projects using the Zig build system.
//! Supports 60+ targets natively via Zig's cross-compilation framework.

use crate::error::Result;
use crate::compiler::LanguageCompiler;
use crate::core::{CompileTarget, CompileResult};
use crate::language::Language;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Zig Compiler using native `zig build`
pub struct ZigCompiler {
    zig_path: Option<PathBuf>,
    project_root: PathBuf,
}

impl ZigCompiler {
    /// Create a new Zig compiler instance
    pub fn new(project_root: PathBuf) -> Self {
        let zig_path = which::which("zig").ok();

        Self {
            zig_path,
            project_root,
        }
    }

    /// Convert UCC target triple to Zig target triple
    fn ucc_to_zig_target(target: &CompileTarget) -> String {
        // Zig supports native (.) or explicit targets
        // For now, use the target as-is
        // Full mapping would require more complex translation
        target.triple.clone()
    }

    /// Check if build.zig exists
    fn has_build_zig(&self) -> bool {
        self.project_root.join("build.zig").exists()
    }
}

impl Default for ZigCompiler {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

impl LanguageCompiler for ZigCompiler {
    async fn compile(
        &self,
        _sources: &[&Path],
        target: &CompileTarget,
    ) -> Result<CompileResult> {
        let zig_path = self.zig_path.clone().ok_or_else(|| {
            crate::error::Error::CompilerNotFound {
                compiler: "zig".to_string(),
                language: "Zig".to_string(),
            }
        })?;

        let project_root = self.project_root.clone();
        let target_clone = target.clone();
        let target_str = Self::ucc_to_zig_target(&target_clone);

        let (output, proj_root) = tokio::task::spawn_blocking(move || {
            let mut cmd = Command::new(&zig_path);

            cmd.current_dir(&project_root);
            cmd.arg("build");

            // Set cross-compilation target if not native
            if target_str != "native" {
                cmd.arg("-Dtarget=".to_string() + &target_str);
            }

            // Release mode for optimization
            cmd.arg("-Doptimize=ReleaseFast");

            (cmd.output(), project_root)
        })
        .await
        .map_err(|e| crate::error::Error::Config(format!("Failed to spawn zig: {}", e)))?;

        let output = output
            .map_err(|e| crate::error::Error::Config(format!("Zig invocation failed: {}", e)))?;

        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let mut result = CompileResult::new(Language::Zig, target.clone());
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
            // Zig build output is typically in zig-cache/bin/
            result.artifacts.push(proj_root.join("zig-cache/bin"));
        }

        Ok(result)
    }

    fn check_availability(&self) -> Result<()> {
        if self.zig_path.is_some() {
            Ok(())
        } else {
            Err(crate::error::Error::Config(
                "Zig compiler not found in PATH".to_string(),
            ))
        }
    }

    fn get_version(&self) -> Result<String> {
        let zig_path = self.zig_path.clone().ok_or_else(|| {
            crate::error::Error::Config("Zig not found".to_string())
        })?;

        let output = Command::new(&zig_path)
            .arg("version")
            .output()
            .map_err(|e| crate::error::Error::Config(format!("Failed to get Zig version: {}", e)))?;

        Ok(String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zig_compiler_creation() {
        let compiler = ZigCompiler::new(PathBuf::from("."));
        // Zig might not be installed everywhere
        let _ = compiler.check_availability();
    }

    #[test]
    fn test_build_zig_detection() {
        let compiler = ZigCompiler::new(PathBuf::from("."));
        // Check if build.zig exists in current directory
        let has_build = compiler.has_build_zig();
        assert!(!has_build); // Unlikely to have build.zig in test dir
    }

    #[test]
    fn test_ucc_to_zig_target() {
        let target = CompileTarget::new("x86_64-linux-gnu");
        let zig_target = ZigCompiler::ucc_to_zig_target(&target);
        assert_eq!(zig_target, "x86_64-linux-gnu");
    }
}
