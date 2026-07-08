use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub version: String,
}

pub struct ServiceRegistry {
    services: Arc<DashMap<String, ServiceInfo>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: Arc::new(DashMap::new()),
        }
    }

    pub fn register(&self, name: String, version: String) -> Result<()> {
        let service = ServiceInfo { name: name.clone(), version };
        self.services.insert(name, service);
        Ok(())
    }

    pub fn lookup(&self, name: &str) -> Option<ServiceInfo> {
        self.services.get(name).map(|s| s.clone())
    }

    pub fn list_services(&self) -> Vec<String> {
        self.services.iter().map(|r| r.key().clone()).collect()
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_register_service() {
        let reg = ServiceRegistry::new();
        reg.register("svc1".to_string(), "1.0.0".to_string()).unwrap();
        assert!(reg.lookup("svc1").is_some());
    }
}
