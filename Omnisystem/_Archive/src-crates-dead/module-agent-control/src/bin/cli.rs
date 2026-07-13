use std::sync::Arc;
use module_agent_control::AgentModuleController;
use usee_search_engine::SearchEngine;
use universal_module_loader::ModuleLoader;
use universal_module_registry::ModuleRegistry;

#[tokio::main]
async fn main() {
    println!("Module Agent Control - v{}", env!("CARGO_PKG_VERSION"));
    println!("Agent Discovery and Module Management");
    println!();
    println!("Features:");
    println!("  - Agent module discovery");
    println!("  - Capability-based search");
    println!("  - Agent preferences");
    println!("  - Autonomous load/unload");
    println!("  - Performance monitoring");
    println!();
    println!("Creating controller...");
    let registry = Arc::new(ModuleRegistry::new());
    let loader = Arc::new(ModuleLoader::new(registry.clone()));
    let search_engine = Arc::new(SearchEngine::new(registry));
    let controller = AgentModuleController::new(registry, loader, search_engine);
    println!("Agent controller created");
    println!("Status: Agents can now discover and control modules");
}
