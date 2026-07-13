use universal_module_loader::ModuleLoader;
use universal_module_registry::ModuleRegistry;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("Universal Module Loader CLI - v{}", env!("CARGO_PKG_VERSION"));
    println!("Dynamic Module Loading & Lifecycle Management");
    println!();
    println!("Features:");
    println!("  - Dynamic module loading on-demand");
    println!("  - Dependency resolution (parallel)");
    println!("  - State machine (8 states)");
    println!("  - Health monitoring");
    println!("  - Metrics collection");
    println!("  - Graceful shutdown");
    println!();
    println!("Creating test loader...");
    let registry = Arc::new(ModuleRegistry::new());
    let loader = ModuleLoader::new(registry);
    println!("Loader created with {} loaded modules", loader.count_loaded_modules());
    println!();
    println!("Status: Universal Module Loader ready for production use");
}
