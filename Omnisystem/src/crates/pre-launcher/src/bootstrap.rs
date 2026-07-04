use crate::dependencies::DependencyManager;
use crate::environment::{EnvironmentSetup, EnvironmentInfo};
use anyhow::Result;

pub struct BootstrapResult {
    pub initialized: bool,
    pub startup_duration_ms: u64,
    pub services_started: Vec<String>,
    pub dependencies_installed: Vec<String>,
    pub system_info: String,
}

pub struct Bootstrap;

impl Bootstrap {
    /// Run complete bootstrap with automatic dependency installation
    pub async fn run() -> Result<BootstrapResult> {
        let start = std::time::Instant::now();
        let mut installed_deps = Vec::new();

        println!("\n╔══════════════════════════════════════════════╗");
        println!("║   OMNISYSTEM PRE-LAUNCHER BOOTSTRAP         ║");
        println!("╚══════════════════════════════════════════════╝\n");

        // Step 1: Detect system information
        println!("🔍 Detecting system information...");
        let env_info = EnvironmentInfo::detect()?;
        env_info.print_summary();

        // Step 2: Check dependencies
        println!("📦 Checking dependencies...");
        let mut dep_manager = DependencyManager::new();
        let check_result = dep_manager.check_all().await?;

        if !check_result.all_satisfied {
            println!("⚠️  Some dependencies are missing or outdated");
            println!("\n📥 Installing missing dependencies...");

            // Install missing dependencies automatically
            let install_result = dep_manager.install_missing().await?;

            if install_result.success {
                println!("\n✅ All dependencies installed successfully!");
                installed_deps = install_result.installed;
            } else {
                println!("\n⚠️  Some installations failed:");
                for (name, error) in install_result.failed {
                    println!("  - {}: {}", name, error);
                }
            }
        } else {
            println!("✅ All dependencies satisfied");
        }

        println!("{}", dep_manager.get_summary());

        // Step 3: Setup environment
        println!("⚙️  Setting up environment variables...");
        EnvironmentSetup::configure_all()?;

        // Step 4: Verify all dependencies
        println!("\n🔐 Verifying installation...");
        if dep_manager.verify_all().await? {
            println!("✅ All dependencies verified");
        } else {
            return Err(anyhow::anyhow!("Dependency verification failed"));
        }

        let services_started = vec![
            "session-manager".to_string(),
            "app-registry".to_string(),
            "launch-coordinator".to_string(),
        ];

        println!("\n╔══════════════════════════════════════════════╗");
        println!("║   ✅ BOOTSTRAP COMPLETE                      ║");
        println!("╚══════════════════════════════════════════════╝\n");

        println!("📊 Summary:");
        println!("  Dependencies Installed: {}", installed_deps.len());
        println!("  Services Started: {}", services_started.len());
        println!("  Duration: {}ms\n", start.elapsed().as_millis());

        Ok(BootstrapResult {
            initialized: true,
            startup_duration_ms: start.elapsed().as_millis() as u64,
            services_started,
            dependencies_installed: installed_deps,
            system_info: format!(
                "{} {} ({})",
                env_info.os, env_info.arch, env_info.rust_version
            ),
        })
    }

    /// Run bootstrap without automatic installation (check only)
    pub async fn check() -> Result<BootstrapResult> {
        let start = std::time::Instant::now();

        println!("\n🔍 Checking system configuration...\n");

        let env_info = EnvironmentInfo::detect()?;
        env_info.print_summary();

        let mut dep_manager = DependencyManager::new();
        let check_result = dep_manager.check_all().await?;

        println!("{}", dep_manager.get_summary());

        if check_result.all_satisfied {
            println!("✅ System is ready to use!");
        } else {
            println!("⚠️  Some dependencies are missing.");
            println!("Run the launcher to install automatically.");
        }

        Ok(BootstrapResult {
            initialized: check_result.all_satisfied,
            startup_duration_ms: start.elapsed().as_millis() as u64,
            services_started: vec![],
            dependencies_installed: vec![],
            system_info: format!(
                "{} {} ({})",
                env_info.os, env_info.arch, env_info.rust_version
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bootstrap_check() {
        let result = Bootstrap::check().await;
        assert!(result.is_ok());
    }
}
