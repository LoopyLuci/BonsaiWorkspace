//! C/C++ Compiler - GCC and Clang integration
//!
//! Supports compilation of C and C++ projects with both GCC and Clang.
//! Auto-detects .c, .cpp, .cc, .cxx files and links them together.

use crate::error::Result;
use crate::compiler::LanguageCompiler;
use crate::core::{CompileTarget, CompileResult};
use crate::language::Language;
use std::path::{Path, PathBuf};
use std::process::Command;

/// C/C++ Compiler using GCC or Clang
pub struct CppCompiler {
    gcc_path: Option<PathBuf>,
    clang_path: Option<PathBuf>,
    prefer_clang: bool,
}

impl CppCompiler {
    /// Create a new C/C++ compiler instance
    pub fn new() -> Self {
        let gcc_path = which::which("gcc").ok().or_else(|| which::which("g++").ok());
        let clang_path = which::which("clang").ok().or_else(|| which::which("clang++").ok());

        Self {
            gcc_path,
            clang_path,
            prefer_clang: false,
        }
    }

    /// Prefer Clang over GCC
    pub fn prefer_clang(mut self) -> Self {
        self.prefer_clang = true;
        self
    }

    /// Get the compiler command to use
    fn get_compiler(&self, is_cpp: bool) -> Option<PathBuf> {
        if self.prefer_clang {
            self.clang_path.clone().or_else(|| {
                if is_cpp {
                    which::which("clang++").ok()
                } else {
                    which::which("clang").ok()
                }
            })
        } else {
            self.gcc_path.clone().or_else(|| {
                if is_cpp {
                    which::which("g++").ok()
                } else {
                    which::which("gcc").ok()
                }
            })
        }
    }

    /// Detect if source is C++ (vs C)
    fn is_cpp_file(path: &Path) -> bool {
        match path.extension().and_then(|e| e.to_str()) {
            Some("cpp") | Some("cc") | Some("cxx") | Some("c++") | Some("hpp") | Some("h++") => {
                true
            }
            Some("c") | Some("h") => false,
            _ => false,
        }
    }

    /// Get compile flags for target architecture
    fn get_target_flags(target: &CompileTarget) -> Vec<String> {
        vec![
            "-march=native".to_string(),
            format!("-DTARGET={}", target.triple),
        ]
    }

    /// Get optimization flags
    fn get_optimization_flags(level: u8) -> Vec<String> {
        match level {
            0 => vec!["-O0".to_string()],
            1 => vec!["-O1".to_string()],
            2 => vec!["-O2".to_string()],
            3 => vec!["-O3".to_string()],
            _ => vec!["-O3".to_string()],
        }
    }
}

impl Default for CppCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageCompiler for CppCompiler {
    async fn compile(
        &self,
        sources: &[&Path],
        target: &CompileTarget,
    ) -> Result<CompileResult> {
        let has_cpp = sources.iter().any(|p| Self::is_cpp_file(p));
        let compiler = self.get_compiler(has_cpp).ok_or_else(|| {
            crate::error::Error::CompilerNotFound {
                compiler: if has_cpp { "g++" } else { "gcc" }.to_string(),
                language: "C/C++".to_string(),
            }
        })?;

        let mut cmd = Command::new(&compiler);

        // Add source files
        for source in sources {
            cmd.arg(source);
        }

        // Add output
        cmd.arg("-o")
            .arg("a.out");

        // Add optimization (default: O2)
        cmd.args(Self::get_optimization_flags(2));

        // Add target flags
        cmd.args(Self::get_target_flags(target));

        // Add standard flags
        if has_cpp {
            cmd.arg("-std=c++17");
        } else {
            cmd.arg("-std=c99");
        }

        let output = tokio::task::spawn_blocking({
            move || cmd.output()
        })
        .await
        .map_err(|e| crate::error::Error::Config(format!("Failed to spawn compiler: {}", e)))?
        .map_err(|e| crate::error::Error::Config(format!("Compiler invocation failed: {}", e)))?;

        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let mut result = CompileResult::new(
            if has_cpp {
                Language::Cpp
            } else {
                Language::C
            },
            target.clone(),
        );
        result.output = format!("{}\n{}", stdout, stderr);
        result.success = success;

        if !success {
            let errors: Vec<String> = stderr
                .lines()
                .filter(|line| line.contains("error"))
                .map(|s| s.to_string())
                .collect();
            result.errors = errors;
        }

        Ok(result)
    }

    fn check_availability(&self) -> Result<()> {
        if self.get_compiler(false).is_some() {
            Ok(())
        } else {
            Err(crate::error::Error::Config(
                "GCC or Clang not found in PATH".to_string(),
            ))
        }
    }

    fn get_version(&self) -> Result<String> {
        let compiler = self.get_compiler(false).ok_or_else(|| {
            crate::error::Error::Config("Compiler not found".to_string())
        })?;

        let output = Command::new(&compiler)
            .arg("--version")
            .output()
            .map_err(|e| crate::error::Error::Config(format!("Failed to get version: {}", e)))?;

        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("Unknown")
            .to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpp_compiler_creation() {
        let compiler = CppCompiler::new();
        assert!(compiler.gcc_path.is_some() || compiler.clang_path.is_some());
    }

    #[test]
    fn test_is_cpp_file() {
        assert!(CppCompiler::is_cpp_file(Path::new("main.cpp")));
        assert!(CppCompiler::is_cpp_file(Path::new("lib.cc")));
        assert!(!CppCompiler::is_cpp_file(Path::new("main.c")));
    }

    #[test]
    fn test_optimization_flags() {
        let flags = CppCompiler::get_optimization_flags(2);
        assert!(flags.contains(&"-O2".to_string()));
    }
}
