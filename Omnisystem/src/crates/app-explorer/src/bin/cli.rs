use std::sync::Arc;
use app_explorer::AppExplorer;
use app_marketplace::AppMarketplace;
use usee_search_engine::SearchEngine;
use universal_module_loader::ModuleLoader;
use universal_module_registry::ModuleRegistry;

#[tokio::main]
async fn main() {
    println!("App Explorer - v{}", env!("CARGO_PKG_VERSION"));
    println!("Interactive Module and Application Browser");
    println!();
    println!("Features:");
    println!("  - Browse applications by category");
    println!("  - Interactive search");
    println!("  - Dependency visualization");
    println!("  - Feature inspection");
    println!("  - Rating and reviews");
    println!("  - Recent items tracking");
    println!();
    println!("Creating explorer...");
    let registry = Arc::new(ModuleRegistry::new());
    let loader = Arc::new(ModuleLoader::new(registry.clone()));
    let search_engine = Arc::new(SearchEngine::new(registry.clone()));
    let marketplace = Arc::new(AppMarketplace::new(registry.clone(), loader, search_engine.clone()));
    let explorer = AppExplorer::new(registry, marketplace, search_engine);
    println!("Explorer created with {} categories", explorer.get_categories().len());
    println!("Status: App Explorer ready for interactive browsing");
}
