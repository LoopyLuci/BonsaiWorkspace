use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ConfigValue {
    pub key: String,
    pub value: String,
}

pub struct ConfigManager {
    config: Arc<DashMap<String, String>>,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self { config: Arc::new(DashMap::new()) }
    }
    
    pub fn set(&self, key: String, value: String) {
        self.config.insert(key, value);
    }
    
    pub fn get(&self, key: &str) -> Option<String> {
        self.config.get(key).map(|v| v.clone())
    }
    
    pub fn remove(&self, key: &str) -> bool {
        self.config.remove(key).is_some()
    }
    
    pub fn config_count(&self) -> usize {
        self.config.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_set_and_get() {
        let cfg = ConfigManager::new();
        cfg.set("key1".to_string(), "value1".to_string());
        assert_eq!(cfg.get("key1"), Some("value1".to_string()));
    }
    
    #[test]
    fn test_remove() {
        let cfg = ConfigManager::new();
        cfg.set("key1".to_string(), "value1".to_string());
        assert!(cfg.remove("key1"));
        assert_eq!(cfg.get("key1"), None);
    }
}
