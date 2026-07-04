// Environment setup and configuration

use anyhow::Result;
use std::env;
use std::path::PathBuf;
use std::process::Command;

pub struct EnvironmentSetup;

impl EnvironmentSetup {
    /// Configure all necessary environment variables
    pub fn configure_all() -> Result<EnvironmentSetup> {
        println!("⚙️  Configuring environment...");

        // Setup Rust environment
        Self::setup_rust_env()?;

        // Setup Node environment
        Self::setup_node_env()?;

        // Setup cargo build cache
        Self::setup_build_cache()?;

        // Setup Tauri environment
        Self::setup_tauri_env()?;

        println!("✓ Environment configured successfully");

        Ok(EnvironmentSetup)
    }

    fn setup_rust_env() -> Result<()> {
        // Ensure Rust is in PATH
        if !Self::is_in_path("rustc") {
            println!("📍 Adding Rust to PATH...");

            #[cfg(target_os = "windows")]
            {
                let home = env::var("USERPROFILE")?;
                let cargo_home = format!("{}/.cargo/bin", home);
                env::set_var("PATH", format!("{}:{}", cargo_home, env::var("PATH")?));
            }

            #[cfg(not(target_os = "windows"))]
            {
                let home = env::var("HOME")?;
                let cargo_home = format!("{}/.cargo/bin", home);
                env::set_var("PATH", format!("{}:{}", cargo_home, env::var("PATH")?));
            }
        }

        // Set Rust backtrace for debugging
        env::set_var("RUST_BACKTRACE", "1");

        // Set optimization level for builds
        env::set_var("RUSTFLAGS", "-C target-cpu=native");

        // Install Rust toolchain components
        println!("📦 Installing Rust components...");
        Command::new("rustup")
            .args(&["component", "add", "rustfmt", "clippy", "rust-analyzer"])
            .output()?;

        // Update Rust
        Command::new("rustup")
            .args(&["update", "stable"])
            .output()?;

        Ok(())
    }

    fn setup_node_env() -> Result<()> {
        // Ensure Node is in PATH
        if !Self::is_in_path("node") {
            println!("📍 Adding Node.js to PATH...");
            // Node should be in system PATH after installation
        }

        // Set npm to use secure registry
        Command::new("npm")
            .args(&["config", "set", "registry", "https://registry.npmjs.org/"])
            .output()?;

        Ok(())
    }

    fn setup_build_cache() -> Result<()> {
        println!("🗂️  Setting up build cache...");

        // Setup sccache for faster builds
        env::set_var("RUSTC_WRAPPER", "sccache");

        // Create cache directory
        #[cfg(target_os = "windows")]
        {
            let cache_dir = PathBuf::from(env::var("APPDATA")?).join("sccache");
            std::fs::create_dir_all(&cache_dir)?;
            env::set_var("SCCACHE_DIR", cache_dir);
        }

        #[cfg(not(target_os = "windows"))]
        {
            let cache_dir = PathBuf::from(env::var("HOME")?).join(".cache/sccache");
            std::fs::create_dir_all(&cache_dir)?;
            env::set_var("SCCACHE_DIR", cache_dir);
        }

        // Set Cargo parallel jobs
        env::set_var("CARGO_BUILD_JOBS", num_cpus::get().to_string());

        Ok(())
    }

    fn setup_tauri_env() -> Result<()> {
        println!("⚡ Setting up Tauri environment...");

        // Set development mode
        env::set_var("TAURI_DEV", "true");

        // Set logging
        env::set_var("RUST_LOG", "debug");

        Ok(())
    }

    fn is_in_path(command: &str) -> bool {
        Command::new(if cfg!(target_os = "windows") {
            "where"
        } else {
            "which"
        })
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
    }
}

pub struct EnvironmentInfo {
    pub os: String,
    pub arch: String,
    pub rust_version: String,
    pub node_version: String,
    pub git_version: String,
    pub cargo_version: String,
    pub npm_version: String,
}

impl EnvironmentInfo {
    pub fn detect() -> Result<Self> {
        Ok(EnvironmentInfo {
            os: Self::detect_os(),
            arch: Self::detect_arch(),
            rust_version: Self::detect_version("rustc"),
            node_version: Self::detect_version("node"),
            git_version: Self::detect_version("git"),
            cargo_version: Self::detect_version("cargo"),
            npm_version: Self::detect_version("npm"),
        })
    }

    pub fn print_summary(&self) {
        println!("\n📋 System Information:\n");
        println!("  OS: {}", self.os);
        println!("  Architecture: {}", self.arch);
        println!("  Rust: {}", self.rust_version);
        println!("  Cargo: {}", self.cargo_version);
        println!("  Node.js: {}", self.node_version);
        println!("  npm: {}", self.npm_version);
        println!("  Git: {}\n", self.git_version);
    }

    fn detect_os() -> String {
        match std::env::consts::OS {
            "windows" => "Windows".to_string(),
            "macos" => "macOS".to_string(),
            "linux" => "Linux".to_string(),
            other => other.to_string(),
        }
    }

    fn detect_arch() -> String {
        match std::env::consts::ARCH {
            "x86_64" => "x64".to_string(),
            "aarch64" => "ARM64".to_string(),
            other => other.to_string(),
        }
    }

    fn detect_version(command: &str) -> String {
        match Command::new(command).arg("--version").output() {
            Ok(output) => {
                String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .to_string()
            }
            Err(_) => format!("{} not found", command),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_info_detection() {
        let info = EnvironmentInfo::detect().unwrap();
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
    }
}
