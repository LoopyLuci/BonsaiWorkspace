//! CLI demo: assemble a LauncherCore from its default subsystems and set some metadata.

use launcher_core::coordinator::DefaultLaunchCoordinator;
use launcher_core::lifecycle::DefaultLifecycleManager;
use launcher_core::registry::DefaultAppRegistry;
use launcher_core::session::DefaultSessionManager;
use launcher_core::LauncherCore;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let core = LauncherCore::new(
        Arc::new(DefaultSessionManager::default()),
        Arc::new(DefaultAppRegistry::default()),
        Arc::new(DefaultLaunchCoordinator::default()),
        Arc::new(DefaultLifecycleManager::default()),
    )
    .await?;

    core.set_metadata("environment".to_string(), "production".to_string());
    println!("environment = {:?}", core.get_metadata("environment"));

    Ok(())
}
