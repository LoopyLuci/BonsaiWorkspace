use dashmap::DashMap;
use std::sync::Arc;

pub struct Core { data: Arc<DashMap<String, String>> }

impl Core {
    pub fn new() -> Self {
        Self { data: Arc::new(DashMap::new()) }
    }
    
    pub fn set(&self, k: String, v: String) {
        self.data.insert(k, v);
    }
    
    pub fn get(&self, k: &str) -> Option<String> {
        self.data.get(k).map(|v| v.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_core_new() {
        let _c = Core::new();
    }
    
    #[test]
    fn test_core_set_get() {
        let c = Core::new();
        c.set("key".into(), "value".into());
        assert_eq!(c.get("key"), Some("value".into()));
    }
}
