use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Resource {
    pub resource_id: String,
    pub resource_type: ResourceType,
    pub owner_system: String,
    pub consumers: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    SearchIndex,
    FileStore,
    CommandQueue,
    SessionData,
}

pub struct ResourceCoordinator {
    resources: Arc<DashMap<String, Resource>>,
}

impl ResourceCoordinator {
    pub fn new() -> Self {
        Self {
            resources: Arc::new(DashMap::new()),
        }
    }

    pub fn register_resource(&self, resource_id: String, resource_type: ResourceType, owner: String) {
        let resource = Resource {
            resource_id,
            resource_type,
            owner_system: owner,
            consumers: Vec::new(),
        };
        self.resources.insert(resource.resource_id.clone(), resource);
    }

    pub fn add_consumer(&self, resource_id: &str, consumer: String) -> bool {
        if let Some(mut resource) = self.resources.get_mut(resource_id) {
            resource.consumers.push(consumer);
            true
        } else {
            false
        }
    }

    pub fn get_resource(&self, resource_id: &str) -> Option<Resource> {
        self.resources.get(resource_id).map(|r| r.clone())
    }

    pub fn remove_consumer(&self, resource_id: &str, consumer: &str) -> bool {
        if let Some(mut resource) = self.resources.get_mut(resource_id) {
            resource.consumers.retain(|c| c != consumer);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_registration() {
        let rc = ResourceCoordinator::new();
        rc.register_resource("idx_1".to_string(), ResourceType::SearchIndex, "usee".to_string());
        assert!(rc.get_resource("idx_1").is_some());
    }

    #[test]
    fn test_add_consumer() {
        let rc = ResourceCoordinator::new();
        rc.register_resource("idx_1".to_string(), ResourceType::SearchIndex, "usee".to_string());
        assert!(rc.add_consumer("idx_1", "buddy".to_string()));
        let resource = rc.get_resource("idx_1").unwrap();
        assert_eq!(resource.consumers.len(), 1);
    }
}
