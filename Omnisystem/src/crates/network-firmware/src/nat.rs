use dashmap::DashMap;
use std::net::IpAddr;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct NatMapping {
    pub internal_ip: IpAddr,
    pub external_ip: IpAddr,
    pub internal_port: u16,
    pub external_port: u16,
}

pub struct NatManager {
    mappings: Arc<DashMap<String, NatMapping>>,
}

impl NatManager {
    pub fn new() -> Self {
        Self {
            mappings: Arc::new(DashMap::new()),
        }
    }

    pub fn add_mapping(&self, key: String, mapping: NatMapping) {
        self.mappings.insert(key, mapping);
    }

    pub fn get_mapping(&self, key: &str) -> Option<NatMapping> {
        self.mappings.get(key).map(|m| m.clone())
    }

    pub fn remove_mapping(&self, key: &str) -> bool {
        self.mappings.remove(key).is_some()
    }

    pub fn mapping_count(&self) -> usize {
        self.mappings.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nat_mapping() {
        let nm = NatManager::new();
        let mapping = NatMapping {
            internal_ip: "192.168.1.10".parse().unwrap(),
            external_ip: "203.0.113.1".parse().unwrap(),
            internal_port: 8080,
            external_port: 80,
        };
        nm.add_mapping("rule1".to_string(), mapping);
        assert_eq!(nm.mapping_count(), 1);
    }

    #[test]
    fn test_nat_removal() {
        let nm = NatManager::new();
        let mapping = NatMapping {
            internal_ip: "192.168.1.10".parse().unwrap(),
            external_ip: "203.0.113.1".parse().unwrap(),
            internal_port: 8080,
            external_port: 80,
        };
        nm.add_mapping("rule1".to_string(), mapping);
        assert!(nm.remove_mapping("rule1"));
        assert_eq!(nm.mapping_count(), 0);
    }
}
