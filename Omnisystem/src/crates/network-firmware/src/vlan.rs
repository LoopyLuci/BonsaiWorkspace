use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct VlanConfig {
    pub vlan_id: u16,
    pub vlan_name: String,
    pub ports: Vec<u16>,
    pub mtu: u16,
}

pub struct VlanManager {
    vlans: Arc<DashMap<u16, VlanConfig>>,
}

impl VlanManager {
    pub fn new() -> Self {
        Self {
            vlans: Arc::new(DashMap::new()),
        }
    }

    pub fn create_vlan(&self, vlan_id: u16, name: String) -> bool {
        if self.vlans.contains_key(&vlan_id) {
            return false;
        }
        let config = VlanConfig {
            vlan_id,
            vlan_name: name,
            ports: Vec::new(),
            mtu: 1500,
        };
        self.vlans.insert(vlan_id, config);
        true
    }

    pub fn add_port(&self, vlan_id: u16, port: u16) -> bool {
        if let Some(mut vlan) = self.vlans.get_mut(&vlan_id) {
            vlan.ports.push(port);
            true
        } else {
            false
        }
    }

    pub fn get_vlan(&self, vlan_id: u16) -> Option<VlanConfig> {
        self.vlans.get(&vlan_id).map(|v| v.clone())
    }

    pub fn vlan_count(&self) -> usize {
        self.vlans.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vlan_creation() {
        let vm = VlanManager::new();
        assert!(vm.create_vlan(100, "VLAN100".to_string()));
        assert_eq!(vm.vlan_count(), 1);
    }

    #[test]
    fn test_vlan_port_addition() {
        let vm = VlanManager::new();
        vm.create_vlan(100, "VLAN100".to_string());
        assert!(vm.add_port(100, 1));
        assert!(vm.add_port(100, 2));
        let vlan = vm.get_vlan(100).unwrap();
        assert_eq!(vlan.ports.len(), 2);
    }
}
