//! Omni-Bot - Next-Generation Orchestration Agent
//! 
//! Autonomous control system for Omnisystem, Bonsai Ecosystem, and UOSC
//! with full Bonsai Buddy integration

use omni_bot_core::init as core_init;
use omni_bot_api::init as api_init;
use omni_bot_actors::init as actors_init;
use log::info;

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()
        .ok();
    
    info!("=== Omni-Bot Starting ===");
    
    // Initialize core systems
    core_init();
    api_init();
    actors_init();
    
    info!("Omni-Bot initialized. Ready for commands.");
    info!("Phase 1: API Foundation - Service Management");
    info!("Listening on localhost:8080");
    
    // TODO: Start API server
    // TODO: Initialize actors
    // TODO: Connect to subsystems
    
    // Keep running
    tokio::signal::ctrl_c().await.ok();
    info!("Omni-Bot shutting down...");
}
