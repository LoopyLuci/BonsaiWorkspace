//! App Manager API Server - Main entry point

use app_manager_api::*;
use app_manager_core::{AppRegistry, ModuleRegistry, SearchIndex, AppDiscoveryService};
use dashmap::DashMap;
use std::sync::Arc;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create registries
    let apps_map = Arc::new(DashMap::new());
    let app_registry = Arc::new(AppRegistry::new());
    let module_registry = Arc::new(ModuleRegistry::new());

    // Create search index
    let search_index = Arc::new(SearchIndex::new(apps_map.clone()));

    // Create discovery service
    let discovery_service = Arc::new(AppDiscoveryService::new(
        search_index.clone(),
        apps_map.clone(),
    ));

    // Create API state
    let state = ApiState {
        app_registry,
        module_registry,
        discovery_service,
    };

    // Start server
    let addr: SocketAddr = "127.0.0.1:8080".parse()?;
    println!("Starting App Manager API Server on {}", addr);

    start_server(addr, state).await?;

    Ok(())
}
