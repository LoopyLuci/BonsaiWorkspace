pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
}

pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: vec![],
        }
    }

    pub fn register_plugin(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    pub fn get_plugins(&self) -> usize {
        self.plugins.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin;
    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            "test-plugin"
        }
        fn version(&self) -> &str {
            "1.0.0"
        }
    }

    #[test]
    fn test_plugin_manager() {
        let mut manager = PluginManager::new();
        manager.register_plugin(Box::new(TestPlugin));
        assert_eq!(manager.get_plugins(), 1);
    }
}
