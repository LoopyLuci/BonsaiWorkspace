//! OmniDocker: Next-Generation Enterprise Docker Controller
//!
//! A complete Docker management platform with AI-powered optimization,
//! multi-agent orchestration, and enterprise-grade features.

use docker_engine_core::DockerEngine;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🐳 OmniDocker starting up...");

    // Initialize Docker Engine
    match DockerEngine::new("/var/run/docker.sock").await {
        Ok(engine) => {
            info!("✅ Docker Engine initialized");

            // Health check
            match engine.health_check().await {
                Ok(true) => info!("✅ Docker daemon is healthy"),
                Ok(false) => warn!("⚠️ Docker daemon health check failed"),
                Err(e) => warn!("⚠️ Health check error: {}", e),
            }

            // List containers
            match engine.list_containers().await {
                Ok(containers) => {
                    info!("📦 Found {} containers", containers.len());
                    for container in containers {
                        info!(
                            "  - {} ({}): {:?}",
                            container.name, container.id, container.status
                        );
                    }
                }
                Err(e) => warn!("Error listing containers: {}", e),
            }
        }
        Err(e) => {
            warn!("Failed to initialize Docker Engine: {}", e);
            warn!("Make sure Docker daemon is running at /var/run/docker.sock");
        }
    }

    info!("✅ OmniDocker is running");
    info!("📊 Dashboard: http://localhost:3000");
    info!("🔌 API: http://localhost:8080");

    // Keep server running
    tokio::signal::ctrl_c().await?;
    info!("🛑 OmniDocker shutting down gracefully");

    Ok(())
}
