//! CLI demo for omnisystem-ums: registers two modules with a dependency
//! between them, resolves load order, and runs one through the full
//! load/init/start/execute lifecycle.

use omnisystem_ums::{
    DataLayerManager, ModuleId, ModuleInfo, ModuleRegistry, ModuleRequest, ModuleResolver,
    ModuleRuntime,
};
use std::collections::HashMap;

fn make_module_info(name: &str, dependencies: Vec<String>) -> ModuleInfo {
    ModuleInfo {
        id: ModuleId::from_name(name),
        name: name.to_string(),
        version: "1.0.0".to_string(),
        description: format!("{} module", name),
        author: "demo".to_string(),
        dependencies,
        capabilities: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        interface_version: "1.0".to_string(),
        phase: 1,
        source_path: format!("/umd/{}", name),
        canonical_path: format!("/sylva/{}", name),
        spec_path: format!("/axiom/{}", name),
        metadata: HashMap::new(),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let registry = ModuleRegistry::new();
    registry.register(make_module_info("base", vec![]))?;
    registry.register(make_module_info("app", vec!["base".to_string()]))?;

    let resolver = ModuleResolver::new(registry.clone());
    let load_order = resolver.resolve_load_order(&["app"])?;
    println!("Resolved load order: {:?}", load_order);

    let demo_dir = std::env::temp_dir().join("omnisystem-ums-cli-demo");
    let data_manager = DataLayerManager::new(&demo_dir).await?;
    let runtime = ModuleRuntime::new(registry, resolver, data_manager).await?;

    let app_id = runtime.load_module("app").await?;
    runtime
        .initialize_module(app_id, serde_json::json!({}))
        .await?;
    runtime.start_module(app_id).await?;

    let result = runtime
        .execute(
            app_id,
            ModuleRequest {
                request_id: uuid::Uuid::new_v4().to_string(),
                operation: "ping".to_string(),
                args: serde_json::json!({}),
                metadata: HashMap::new(),
            },
        )
        .await?;
    println!("Execution result: {}", result);

    Ok(())
}
