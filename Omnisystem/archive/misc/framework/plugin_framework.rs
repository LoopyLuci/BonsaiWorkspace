// OMNISYSTEM PLUGIN FRAMEWORK
// Dynamic plugin loading, management, and hot-reloading

use std::collections::HashMap;
use std::sync::{Arc, RwLock, Mutex};
use std::path::PathBuf;

// ============================================================================
// PLUGIN METADATA
// ============================================================================

#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub api_version: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PluginStatus {
    Loaded,
    Initialized,
    Running,
    Paused,
    Disabled,
    Error,
}

// ============================================================================
// PLUGIN TRAIT
// ============================================================================

pub trait Plugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    fn initialize(&mut self) -> Result<(), String>;
    fn execute(&self, command: &str, args: Vec<String>) -> Result<String, String>;
    fn on_load(&self) -> Result<(), String> {
        Ok(())
    }
    fn on_unload(&self) -> Result<(), String> {
        Ok(())
    }
    fn get_capabilities(&self) -> Vec<String>;
    fn health_check(&self) -> Result<(), String>;
}

// ============================================================================
// PLUGIN INSTANCE
// ============================================================================

pub struct PluginInstance {
    pub id: String,
    pub metadata: PluginMetadata,
    pub status: PluginStatus,
    pub loaded_at: std::time::Instant,
    pub execution_count: u64,
    pub error_count: u64,
}

impl PluginInstance {
    pub fn new(id: &str, metadata: PluginMetadata) -> Self {
        PluginInstance {
            id: id.to_string(),
            metadata,
            status: PluginStatus::Loaded,
            loaded_at: std::time::Instant::now(),
            execution_count: 0,
            error_count: 0,
        }
    }
}

// ============================================================================
// PLUGIN MANAGER
// ============================================================================

pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<String, Arc<Mutex<Box<dyn Plugin>>>>>>,
    instances: Arc<RwLock<HashMap<String, PluginInstance>>>,
    plugin_dir: PathBuf,
    config: PluginConfig,
}

pub struct PluginConfig {
    pub auto_load: bool,
    pub auto_reload: bool,
    pub sandboxed: bool,
    pub max_plugins: usize,
    pub timeout_ms: u64,
}

impl Default for PluginConfig {
    fn default() -> Self {
        PluginConfig {
            auto_load: true,
            auto_reload: true,
            sandboxed: true,
            max_plugins: 100,
            timeout_ms: 5000,
        }
    }
}

impl PluginManager {
    pub fn new(plugin_dir: &str) -> Self {
        let config = PluginConfig::default();
        PluginManager {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            instances: Arc::new(RwLock::new(HashMap::new())),
            plugin_dir: PathBuf::from(plugin_dir),
            config,
        }
    }

    pub fn with_config(plugin_dir: &str, config: PluginConfig) -> Self {
        PluginManager {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            instances: Arc::new(RwLock::new(HashMap::new())),
            plugin_dir: PathBuf::from(plugin_dir),
            config,
        }
    }

    pub fn register_plugin(&self, id: &str, plugin: Box<dyn Plugin>) -> Result<(), String> {
        let mut plugins = self.plugins.write().unwrap();
        let mut instances = self.instances.write().unwrap();

        if plugins.len() >= self.config.max_plugins {
            return Err("Plugin limit exceeded".to_string());
        }

        let metadata = plugin.metadata();
        let instance = PluginInstance::new(id, metadata.clone());

        plugins.insert(id.to_string(), Arc::new(Mutex::new(plugin)));
        instances.insert(id.to_string(), instance);

        println!("✅ Plugin registered: {} ({})", id, metadata.name);
        Ok(())
    }

    pub fn load_plugin(&self, id: &str) -> Result<(), String> {
        let plugins = self.plugins.read().unwrap();
        let mut instances = self.instances.write().unwrap();

        if let Some(plugin_arc) = plugins.get(id) {
            if let Ok(mut plugin) = plugin_arc.lock() {
                plugin.on_load()?;
                plugin.initialize()?;

                if let Some(instance) = instances.get_mut(id) {
                    instance.status = PluginStatus::Initialized;
                    println!("🚀 Plugin loaded: {}", id);
                }
                Ok(())
            } else {
                Err("Failed to acquire plugin lock".to_string())
            }
        } else {
            Err(format!("Plugin not found: {}", id))
        }
    }

    pub fn unload_plugin(&self, id: &str) -> Result<(), String> {
        let plugins = self.plugins.read().unwrap();
        let mut instances = self.instances.write().unwrap();

        if let Some(plugin_arc) = plugins.get(id) {
            if let Ok(plugin) = plugin_arc.lock() {
                plugin.on_unload()?;
                if let Some(instance) = instances.get_mut(id) {
                    instance.status = PluginStatus::Disabled;
                    println!("🛑 Plugin unloaded: {}", id);
                }
                Ok(())
            } else {
                Err("Failed to acquire plugin lock".to_string())
            }
        } else {
            Err(format!("Plugin not found: {}", id))
        }
    }

    pub fn execute_plugin(&self, id: &str, command: &str, args: Vec<String>) -> Result<String, String> {
        let plugins = self.plugins.read().unwrap();
        let mut instances = self.instances.write().unwrap();

        if let Some(plugin_arc) = plugins.get(id) {
            if let Ok(plugin) = plugin_arc.lock() {
                match plugin.execute(command, args) {
                    Ok(result) => {
                        if let Some(instance) = instances.get_mut(id) {
                            instance.execution_count += 1;
                            instance.status = PluginStatus::Running;
                        }
                        Ok(result)
                    }
                    Err(e) => {
                        if let Some(instance) = instances.get_mut(id) {
                            instance.error_count += 1;
                            instance.status = PluginStatus::Error;
                        }
                        Err(e)
                    }
                }
            } else {
                Err("Failed to acquire plugin lock".to_string())
            }
        } else {
            Err(format!("Plugin not found: {}", id))
        }
    }

    pub fn reload_plugin(&self, id: &str) -> Result<(), String> {
        self.unload_plugin(id)?;
        self.load_plugin(id)?;
        println!("🔄 Plugin reloaded: {}", id);
        Ok(())
    }

    pub fn list_plugins(&self) -> Vec<PluginInstance> {
        let instances = self.instances.read().unwrap();
        instances.values().cloned().collect()
    }

    pub fn get_plugin_status(&self, id: &str) -> Option<PluginStatus> {
        let instances = self.instances.read().unwrap();
        instances.get(id).map(|i| i.status.clone())
    }

    pub fn health_check_all(&self) -> HashMap<String, Result<(), String>> {
        let plugins = self.plugins.read().unwrap();
        let mut results = HashMap::new();

        for (id, plugin_arc) in plugins.iter() {
            if let Ok(plugin) = plugin_arc.lock() {
                results.insert(id.clone(), plugin.health_check());
            }
        }

        results
    }

    pub fn get_plugin_by_capability(&self, capability: &str) -> Vec<String> {
        let plugins = self.plugins.read().unwrap();
        let mut matching = Vec::new();

        for (id, plugin_arc) in plugins.iter() {
            if let Ok(plugin) = plugin_arc.lock() {
                if plugin.get_capabilities().contains(&capability.to_string()) {
                    matching.push(id.clone());
                }
            }
        }

        matching
    }
}

// ============================================================================
// EXAMPLE PLUGIN IMPLEMENTATION
// ============================================================================

pub struct ExamplePlugin {
    initialized: bool,
    execution_count: u64,
}

impl ExamplePlugin {
    pub fn new() -> Self {
        ExamplePlugin {
            initialized: false,
            execution_count: 0,
        }
    }
}

impl Plugin for ExamplePlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "Example Plugin".to_string(),
            version: "1.0.0".to_string(),
            author: "Omnisystem".to_string(),
            description: "An example plugin for demonstration".to_string(),
            api_version: "1.0".to_string(),
            capabilities: vec!["process".to_string(), "analyze".to_string()],
        }
    }

    fn initialize(&mut self) -> Result<(), String> {
        self.initialized = true;
        println!("📦 Example plugin initialized");
        Ok(())
    }

    fn execute(&self, command: &str, args: Vec<String>) -> Result<String, String> {
        match command {
            "process" => Ok(format!("Processed with args: {:?}", args)),
            "analyze" => Ok(format!("Analyzed: {:?}", args)),
            _ => Err(format!("Unknown command: {}", command)),
        }
    }

    fn on_load(&self) -> Result<(), String> {
        println!("📝 Example plugin loading");
        Ok(())
    }

    fn on_unload(&self) -> Result<(), String> {
        println!("🚪 Example plugin unloading");
        Ok(())
    }

    fn get_capabilities(&self) -> Vec<String> {
        vec!["process".to_string(), "analyze".to_string()]
    }

    fn health_check(&self) -> Result<(), String> {
        if self.initialized {
            Ok(())
        } else {
            Err("Not initialized".to_string())
        }
    }
}

// ============================================================================
// PLUGIN MARKETPLACE
// ============================================================================

pub struct PluginMarketplace {
    registry: Arc<RwLock<HashMap<String, PluginMetadata>>>,
}

impl PluginMarketplace {
    pub fn new() -> Self {
        PluginMarketplace {
            registry: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register_plugin(&self, id: &str, metadata: PluginMetadata) {
        let mut registry = self.registry.write().unwrap();
        registry.insert(id.to_string(), metadata);
        println!("🏪 Plugin registered in marketplace: {}", id);
    }

    pub fn search(&self, query: &str) -> Vec<(String, PluginMetadata)> {
        let registry = self.registry.read().unwrap();
        registry.iter()
            .filter(|(_, meta)| {
                meta.name.to_lowercase().contains(&query.to_lowercase())
                    || meta.description.to_lowercase().contains(&query.to_lowercase())
            })
            .map(|(id, meta)| (id.clone(), meta.clone()))
            .collect()
    }

    pub fn list_all(&self) -> Vec<PluginMetadata> {
        let registry = self.registry.read().unwrap();
        registry.values().cloned().collect()
    }

    pub fn get_by_capability(&self, capability: &str) -> Vec<PluginMetadata> {
        let registry = self.registry.read().unwrap();
        registry.values()
            .filter(|meta| meta.capabilities.contains(&capability.to_string()))
            .cloned()
            .collect()
    }
}

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

pub fn example_plugin_system() -> Result<(), Box<dyn std::error::Error>> {
    let manager = PluginManager::new("./plugins");

    // Register example plugin
    let plugin = Box::new(ExamplePlugin::new());
    manager.register_plugin("example-1", plugin)?;

    // Load plugin
    manager.load_plugin("example-1")?;

    // Execute plugin
    let result = manager.execute_plugin("example-1", "process", vec!["data".to_string()])?;
    println!("Result: {}", result);

    // List plugins
    for plugin in manager.list_plugins() {
        println!("Plugin: {} ({})", plugin.metadata.name, plugin.status);
    }

    // Marketplace
    let marketplace = PluginMarketplace::new();
    marketplace.register_plugin("example-1", PluginMetadata {
        name: "Example Plugin".to_string(),
        version: "1.0.0".to_string(),
        author: "Omnisystem".to_string(),
        description: "An example plugin".to_string(),
        api_version: "1.0".to_string(),
        capabilities: vec!["process".to_string()],
    });

    let results = marketplace.search("example");
    println!("Found {} plugins", results.len());

    Ok(())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_metadata() {
        let metadata = PluginMetadata {
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            author: "Author".to_string(),
            description: "Desc".to_string(),
            api_version: "1.0".to_string(),
            capabilities: vec![],
        };
        assert_eq!(metadata.name, "Test");
    }

    #[test]
    fn test_plugin_instance() {
        let metadata = PluginMetadata {
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            author: "Author".to_string(),
            description: "Desc".to_string(),
            api_version: "1.0".to_string(),
            capabilities: vec![],
        };
        let instance = PluginInstance::new("test-1", metadata);
        assert_eq!(instance.status, PluginStatus::Loaded);
    }

    #[test]
    fn test_plugin_manager() {
        let manager = PluginManager::new("./plugins");
        let plugin = Box::new(ExamplePlugin::new());
        assert!(manager.register_plugin("test", plugin).is_ok());
    }

    #[test]
    fn test_example_plugin() {
        let mut plugin = ExamplePlugin::new();
        assert!(plugin.initialize().is_ok());
        let result = plugin.execute("process", vec!["test".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_plugin_marketplace() {
        let marketplace = PluginMarketplace::new();
        let metadata = PluginMetadata {
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            author: "Author".to_string(),
            description: "Test description".to_string(),
            api_version: "1.0".to_string(),
            capabilities: vec!["process".to_string()],
        };
        marketplace.register_plugin("test", metadata);
        assert_eq!(marketplace.list_all().len(), 1);
    }
}
