//! app-manager-installer CLI
//!
//! Small utility for driving an install/uninstall cycle against a local
//! package repository cache, and for inspecting rollback snapshots.

use app_manager_core::module_lifecycle::ModuleLifecycleManager;
use app_manager_core::types::AppId;
use app_manager_installer::Installer;
use app_manager_repository::{Repository as PackageRepository, RepositoryConfig};
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("install") => {
            let app_id = args.get(2).ok_or("usage: cli install <app-id> <version> [install-path]")?;
            let version = args.get(3).ok_or("usage: cli install <app-id> <version> [install-path]")?;
            let install_path = args
                .get(4)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("./.apps").join(app_id));

            let app_id = AppId::new(app_id.clone())?;
            let version = app_manager_core::types::Version::parse(version)?;

            let repo = PackageRepository::new(RepositoryConfig::default());
            let lifecycle = ModuleLifecycleManager::new();
            let installer = Installer::new(repo, lifecycle);

            match installer.install(&app_id, &version, install_path).await {
                Ok(context) => println!("installed {} v{} ({}%)", app_id, version, context.progress_percent),
                Err(e) => {
                    eprintln!("install failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some("uninstall") => {
            let app_id = args.get(2).ok_or("usage: cli uninstall <app-id>")?;
            let app_id = AppId::new(app_id.clone())?;

            let repo = PackageRepository::new(RepositoryConfig::default());
            let lifecycle = ModuleLifecycleManager::new();
            lifecycle.register_module(app_id.clone())?;
            let installer = Installer::new(repo, lifecycle);

            match installer.uninstall(&app_id).await {
                Ok(()) => println!("uninstalled {}", app_id),
                Err(e) => {
                    eprintln!("uninstall failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            println!("app-manager-installer CLI");
            println!("usage:");
            println!("  cli install <app-id> <version> [install-path]");
            println!("  cli uninstall <app-id>");
        }
    }

    Ok(())
}
