use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct VLANManager {
    vlans: Arc<DashMap<u16, VLANInfo>>,
}

#[derive(Debug, Clone)]
pub struct VLANInfo {
    pub vlan_id: u16,
    pub name: String,
    pub members: Vec<String>,
}

impl VLANManager {
    pub fn new() -> Self {
        Self {
            vlans: Arc::new(DashMap::new()),
        }
    }

    pub fn create_vlan(&self, vlan: VLANInfo) -> Result<()> {
        self.vlans.insert(vlan.vlan_id, vlan);
        tracing::info!("VLAN created");
        Ok(())
    }

    pub fn add_member(&self, vlan_id: u16, member: String) -> Result<()> {
        if let Some(mut vlan) = self.vlans.get_mut(&vlan_id) {
            vlan.members.push(member);
            Ok(())
        } else {
            Err(crate::NetworkError::RoutingError("VLAN not found".to_string()))
        }
    }

    pub fn vlan_count(&self) -> usize {
        self.vlans.len()
    }
}

impl Default for VLANManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_vlan() {
        let manager = VLANManager::new();
        let vlan = VLANInfo {
            vlan_id: 100,
            name: "Management".to_string(),
            members: vec![],
        };
        assert!(manager.create_vlan(vlan).is_ok());
        assert_eq!(manager.vlan_count(), 1);
    }
}
