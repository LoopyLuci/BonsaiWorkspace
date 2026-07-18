//! Diagnostic demo binary for the `ApplicationManager` integration facade.
//!
//! This is a thin entry point that exercises `ApplicationManager` directly
//! for smoke-testing the orchestration layer. The full-featured Omnisystem
//! application-manager command-line tool lives in the separate
//! `app-manager-cli` crate.

use app_manager_omnisystem_integration::ApplicationManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut manager = ApplicationManager::new();
    manager.initialize().await?;

    println!("ApplicationManager initialized (apps_dir={:?})", manager.apps_dir());

    let health = manager.health_check().await?;
    println!("Initialized:    {}", health.initialized);
    println!("Operational:    {}", health.operational);
    println!("Modules loaded: {}", health.modules_loaded);

    Ok(())
}
