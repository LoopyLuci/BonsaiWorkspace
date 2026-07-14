//! CLI demo: register a plugin and trigger a hot-reload.

use advanced_launcher::{HotReloadManager, Plugin, PluginManager};

struct DemoPlugin;
impl Plugin for DemoPlugin {
    fn name(&self) -> &str {
        "demo-plugin"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut plugins = PluginManager::new();
    plugins.register_plugin(Box::new(DemoPlugin));
    println!("Registered plugins: {}", plugins.get_plugins());

    let hotreload = HotReloadManager;
    hotreload.enable("demo-app").await?;
    hotreload.trigger_reload("demo-app").await?;
    println!("Triggered hot-reload for demo-app");

    Ok(())
}
