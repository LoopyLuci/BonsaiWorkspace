use anyhow::Result;
use std::sync::Arc;
use poe_core::{PoeCore, config::PoeConfig, HostBiometricTelemetry};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    println!("🧬 Booting Poe AI...");

    let config = PoeConfig::default();
    let core = Arc::new(PoeCore::new(config)?);
    println!("✅ Core initialized");

    // Create mesh
    let mesh = Arc::new(poe_mesh::PoeMesh::new(
        vec!["node-1".into(), "node-2".into(), "node-3".into()],
        2,
    )?);
    println!("✅ Mesh network joined");

    // Create bridge
    let bridge = poe_bridge::PoeBridge::new(core.clone(), mesh.clone());

    // Test conversation
    let response = bridge.converse("Hello, Poe.", 72).await?;
    println!("💬 {}", response);

    // Test AC Poe personality toggle
    bridge.set_narrative_mode(true).await?;
    let ac_response = bridge.converse("How are you feeling?", 68).await?;
    println!("🎭 {}", ac_response);

    // Return to production mode
    bridge.set_narrative_mode(false).await?;

    println!("🎉 Poe AI is online and ready.");
    Ok(())
}
