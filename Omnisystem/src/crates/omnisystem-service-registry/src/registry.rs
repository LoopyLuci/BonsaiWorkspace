use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Service {
    pub name: String,
    pub endpoint: String,
    pub active: bool,
}

pub struct ServiceRegistry {
    services: Arc<DashMap<String, Service>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self { services: Arc::new(DashMap::new()) }
    }
    
    pub fn register(&self, name: String, endpoint: String) {
        let service = Service { name: name.clone(), endpoint, active: true };
        self.services.insert(name, service);
    }
    
    pub fn get(&self, name: &str) -> Option<Service> {
        self.services.get(name).map(|s| s.clone())
    }
    
    pub fn deactivate(&self, name: &str) -> bool {
        if let Some(mut svc) = self.services.get_mut(name) {
            svc.active = false;
            true
        } else {
            false
        }
    }
    
    pub fn service_count(&self) -> usize {
        self.services.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_register() {
        let reg = ServiceRegistry::new();
        reg.register("svc1".to_string(), "localhost:8080".to_string());
        assert_eq!(reg.service_count(), 1);
    }
    
    #[test]
    fn test_deactivate() {
        let reg = ServiceRegistry::new();
        reg.register("svc1".to_string(), "localhost:8080".to_string());
        assert!(reg.deactivate("svc1"));
    }
}
