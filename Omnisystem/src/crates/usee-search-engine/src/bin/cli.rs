use std::sync::Arc;
use usee_search_engine::SearchEngine;
use universal_module_registry::ModuleRegistry;

#[tokio::main]
async fn main() {
    println!("USEE - Universal Search Engine and Explorer - v{}", env!("CARGO_PKG_VERSION"));
    println!("Ultra-Fast Module Discovery and Search");
    println!();
    println!("Features:");
    println!("  - Full-text search (< 5ms)");
    println!("  - Prefix search/autocomplete (< 1ms)");
    println!("  - Tag-based search");
    println!("  - Capability-based search");
    println!("  - Advanced filtering");
    println!("  - Fuzzy matching");
    println!();
    println!("Creating search engine...");
    let registry = Arc::new(ModuleRegistry::new());
    let engine = SearchEngine::new(registry);
    println!("Search engine created with {} indexed modules", engine.registry.count_modules());
    println!();
    println!("Example searches:");
    println!("  - search \"analytics\" -> find all modules with 'analytics'");
    println!("  - search tag:realtime -> find all real-time modules");
    println!("  - search capability:processing -> find processing modules");
    println!();
    println!("Status: USEE ready for production searches");
}
