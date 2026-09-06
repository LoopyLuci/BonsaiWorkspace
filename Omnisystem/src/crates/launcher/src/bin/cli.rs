//! CLI for launcher — exercises the crate's real daemon lifecycle, health
//! check and event bus, instead of the dead generic Component template.

use launcher::{EventBus, HealthMonitor, LauncherDaemon, LauncherEvent};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut daemon = LauncherDaemon::new();
    println!("daemon running: {}", daemon.is_running());
    daemon.start().await?;
    println!("daemon running: {}", daemon.is_running());

    let status = HealthMonitor::check().await?;
    println!("health status: {status:?}");

    let bus = EventBus::new();
    bus.publish(LauncherEvent::AppStarted("launcher_cli".to_string())).await?;
    println!("published LauncherEvent::AppStarted");

    daemon.stop().await?;
    println!("daemon running: {}", daemon.is_running());

    Ok(())
}
