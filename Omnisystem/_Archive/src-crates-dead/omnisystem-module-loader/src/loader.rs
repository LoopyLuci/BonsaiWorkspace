use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Module {
    pub id: String,
    pub name: String,
    pub loaded: bool,
}

pub struct ModuleLoader {
    modules: Arc<DashMap<String, Module>>,
}

impl ModuleLoader {
    pub fn new() -> Self {
        Self { modules: Arc::new(DashMap::new()) }
    }
    
    pub fn load_module(&self, id: String, name: String) -> String {
        let module = Module { id: id.clone(), name, loaded: true };
        self.modules.insert(id.clone(), module);
        id
    }
    
    pub fn get_module(&self, id: &str) -> Option<Module> {
        self.modules.get(id).map(|m| m.clone())
    }
    
    pub fn unload_module(&self, id: &str) -> bool {
        self.modules.remove(id).is_some()
    }
    
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_load() {
        let loader = ModuleLoader::new();
        loader.load_module("mod1".to_string(), "module1".to_string());
        assert_eq!(loader.module_count(), 1);
    }
    
    #[test]
    fn test_unload() {
        let loader = ModuleLoader::new();
        let id = loader.load_module("mod1".to_string(), "module1".to_string());
        assert!(loader.unload_module(&id));
    }
}
