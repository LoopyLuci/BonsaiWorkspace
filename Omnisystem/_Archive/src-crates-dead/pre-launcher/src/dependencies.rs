// Comprehensive dependency management system
// Automatically detects and installs all required dependencies

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub enum DependencyStatus {
    Installed(String),     // Version
    Missing,
    OutOfDate(String),     // Current version
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub display_name: String,
    pub min_version: String,
    pub detector: fn() -> DependencyStatus,
    pub installer: fn() -> Result<()>,
    pub status: DependencyStatus,
}

pub struct DependencyManager {
    dependencies: Vec<Dependency>,
    is_windows: bool,
    is_macos: bool,
    is_linux: bool,
}

impl DependencyManager {
    pub fn new() -> Self {
        let is_windows = cfg!(target_os = "windows");
        let is_macos = cfg!(target_os = "macos");
        let is_linux = cfg!(target_os = "linux");

        Self {
            dependencies: vec![
                // Core Rust
                Dependency {
                    name: "rust".to_string(),
                    display_name: "Rust Compiler".to_string(),
                    min_version: "1.70.0".to_string(),
                    detector: detect_rust,
                    installer: install_rust,
                    status: DependencyStatus::Missing,
                },
                // Node.js
                Dependency {
                    name: "node".to_string(),
                    display_name: "Node.js".to_string(),
                    min_version: "18.0.0".to_string(),
                    detector: detect_node,
                    installer: install_node,
                    status: DependencyStatus::Missing,
                },
                // Git
                Dependency {
                    name: "git".to_string(),
                    display_name: "Git".to_string(),
                    min_version: "2.30.0".to_string(),
                    detector: detect_git,
                    installer: install_git,
                    status: DependencyStatus::Missing,
                },
                // Cargo (comes with Rust)
                Dependency {
                    name: "cargo".to_string(),
                    display_name: "Cargo (Rust Package Manager)".to_string(),
                    min_version: "1.70.0".to_string(),
                    detector: detect_cargo,
                    installer: || Ok(()), // Installed with Rust
                    status: DependencyStatus::Missing,
                },
                // npm (comes with Node.js)
                Dependency {
                    name: "npm".to_string(),
                    display_name: "npm (Node Package Manager)".to_string(),
                    min_version: "9.0.0".to_string(),
                    detector: detect_npm,
                    installer: || Ok(()), // Installed with Node.js
                    status: DependencyStatus::Missing,
                },
            ],
            is_windows,
            is_macos,
            is_linux,
        }
    }

    pub async fn check_all(&mut self) -> Result<DependencyCheckResult> {
        let mut installed = Vec::new();
        let mut missing = Vec::new();
        let mut out_of_date = Vec::new();

        for dep in &mut self.dependencies {
            let status = (dep.detector)();
            dep.status = status.clone();

            match status {
                DependencyStatus::Installed(version) => {
                    installed.push((dep.display_name.clone(), version));
                }
                DependencyStatus::Missing => {
                    missing.push(dep.display_name.clone());
                }
                DependencyStatus::OutOfDate(current) => {
                    out_of_date.push((dep.display_name.clone(), current, dep.min_version.clone()));
                }
            }
        }

        Ok(DependencyCheckResult {
            installed,
            missing: missing.clone(),
            out_of_date: out_of_date.clone(),
            all_satisfied: missing.is_empty() && out_of_date.is_empty(),
        })
    }

    pub async fn install_missing(&mut self) -> Result<InstallationResult> {
        let check = self.check_all().await?;

        if check.all_satisfied {
            return Ok(InstallationResult {
                success: true,
                installed: Vec::new(),
                failed: Vec::new(),
                total_time_seconds: 0,
            });
        }

        let mut installed = Vec::new();
        let mut failed = Vec::new();
        let start = std::time::Instant::now();

        // Install missing dependencies
        for dep in &mut self.dependencies {
            if matches!(dep.status, DependencyStatus::Missing) {
                println!("📦 Installing {}...", dep.display_name);
                match (dep.installer)() {
                    Ok(_) => {
                        println!("✓ {} installed", dep.display_name);
                        installed.push(dep.display_name.clone());
                        dep.status = (dep.detector)();
                    }
                    Err(e) => {
                        println!("✗ Failed to install {}: {}", dep.display_name, e);
                        failed.push((dep.display_name.clone(), e.to_string()));
                    }
                }
            }
        }

        Ok(InstallationResult {
            success: failed.is_empty(),
            installed,
            failed,
            total_time_seconds: start.elapsed().as_secs(),
        })
    }

    pub async fn verify_all(&self) -> Result<bool> {
        for dep in &self.dependencies {
            match &dep.status {
                DependencyStatus::Installed(_) => {
                    println!("✓ {} is installed", dep.display_name);
                }
                _ => {
                    println!("✗ {} is not properly installed", dep.display_name);
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    pub fn get_summary(&self) -> String {
        let mut summary = String::new();
        summary.push_str("\n📊 Dependency Summary:\n");

        for dep in &self.dependencies {
            match &dep.status {
                DependencyStatus::Installed(version) => {
                    summary.push_str(&format!("  ✓ {} v{}\n", dep.display_name, version));
                }
                DependencyStatus::Missing => {
                    summary.push_str(&format!("  ✗ {} (missing)\n", dep.display_name));
                }
                DependencyStatus::OutOfDate(current) => {
                    summary.push_str(&format!(
                        "  ⚠ {} v{} (need v{}+)\n",
                        dep.display_name, current, dep.min_version
                    ));
                }
            }
        }

        summary
    }
}

pub struct DependencyCheckResult {
    pub installed: Vec<(String, String)>,
    pub missing: Vec<String>,
    pub out_of_date: Vec<(String, String, String)>,
    pub all_satisfied: bool,
}

pub struct InstallationResult {
    pub success: bool,
    pub installed: Vec<String>,
    pub failed: Vec<(String, String)>,
    pub total_time_seconds: u64,
}

// ============================================================================
// Detector Functions
// ============================================================================

fn detect_rust() -> DependencyStatus {
    match Command::new("rustc").arg("--version").output() {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stdout);
            if let Some(v) = version.split_whitespace().nth(1) {
                DependencyStatus::Installed(v.to_string())
            } else {
                DependencyStatus::Missing
            }
        }
        Err(_) => DependencyStatus::Missing,
    }
}

fn detect_cargo() -> DependencyStatus {
    match Command::new("cargo").arg("--version").output() {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stdout);
            if let Some(v) = version.split_whitespace().nth(1) {
                DependencyStatus::Installed(v.to_string())
            } else {
                DependencyStatus::Missing
            }
        }
        Err(_) => DependencyStatus::Missing,
    }
}

fn detect_node() -> DependencyStatus {
    match Command::new("node").arg("--version").output() {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stdout)
                .trim()
                .trim_start_matches('v')
                .to_string();
            DependencyStatus::Installed(version)
        }
        Err(_) => DependencyStatus::Missing,
    }
}

fn detect_npm() -> DependencyStatus {
    match Command::new("npm").arg("--version").output() {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            DependencyStatus::Installed(version)
        }
        Err(_) => DependencyStatus::Missing,
    }
}

fn detect_git() -> DependencyStatus {
    match Command::new("git").arg("--version").output() {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stdout);
            if let Some(v) = version.split_whitespace().nth(2) {
                DependencyStatus::Installed(v.to_string())
            } else {
                DependencyStatus::Missing
            }
        }
        Err(_) => DependencyStatus::Missing,
    }
}

// ============================================================================
// Installer Functions
// ============================================================================

fn install_rust() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        println!("📥 Downloading Rust installer...");
        let output = Command::new("powershell")
            .args(&[
                "-Command",
                "iwr https://win.rustup.rs -OutFile rustup-init.exe; .\\rustup-init.exe -y",
            ])
            .status()?;

        if !output.success() {
            return Err(anyhow!("Rust installation failed"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        println!("📥 Downloading Rust installer...");
        let output = Command::new("sh")
            .arg("-c")
            .arg("curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y")
            .status()?;

        if !output.success() {
            return Err(anyhow!("Rust installation failed"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        println!("📥 Downloading Rust installer...");
        let output = Command::new("sh")
            .arg("-c")
            .arg("curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y")
            .status()?;

        if !output.success() {
            return Err(anyhow!("Rust installation failed"));
        }
    }

    Ok(())
}

fn install_node() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        println!("📥 Installing Node.js...");
        // Check if chocolatey is available
        if Command::new("choco").arg("--version").output().is_ok() {
            Command::new("choco")
                .args(&["install", "nodejs", "-y"])
                .status()?;
        } else {
            println!("Please install Node.js from https://nodejs.org/");
            return Err(anyhow!("Node.js not installed. Please install manually from https://nodejs.org/"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        println!("📥 Installing Node.js...");
        // Check if homebrew is available
        if Command::new("brew").arg("--version").output().is_ok() {
            Command::new("brew")
                .args(&["install", "node"])
                .status()?;
        } else {
            println!("Please install Homebrew first: https://brew.sh");
            return Err(anyhow!("Homebrew not found. Please install from https://brew.sh"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        println!("📥 Installing Node.js...");
        let output = Command::new("sh")
            .arg("-c")
            .arg("curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash - && sudo apt-get install -y nodejs")
            .status()?;

        if !output.success() {
            return Err(anyhow!("Node.js installation failed"));
        }
    }

    Ok(())
}

fn install_git() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        println!("📥 Installing Git...");
        if Command::new("choco").arg("--version").output().is_ok() {
            Command::new("choco")
                .args(&["install", "git", "-y"])
                .status()?;
        } else {
            return Err(anyhow!("Git not installed. Please install from https://git-scm.com/"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        println!("📥 Installing Git...");
        if Command::new("brew").arg("--version").output().is_ok() {
            Command::new("brew")
                .args(&["install", "git"])
                .status()?;
        } else {
            return Err(anyhow!("Homebrew not found"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        println!("📥 Installing Git...");
        Command::new("sudo")
            .args(&["apt-get", "install", "-y", "git"])
            .status()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_manager_creation() {
        let manager = DependencyManager::new();
        assert!(!manager.dependencies.is_empty());
        assert_eq!(manager.dependencies.len(), 5);
    }

    #[test]
    fn test_detect_rust() {
        let status = detect_rust();
        assert!(!matches!(status, DependencyStatus::Missing));
    }

    #[test]
    fn test_detect_cargo() {
        let status = detect_cargo();
        assert!(!matches!(status, DependencyStatus::Missing));
    }
}
