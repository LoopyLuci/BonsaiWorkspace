use crate::{Result, IotError};
use dashmap::DashMap;
use std::sync::Arc;

pub struct AetherZWave {
    home_id: u32,
    node_id: u8,
    devices: Arc<DashMap<u8, ZWaveDevice>>,
    security_level: SecurityLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel {
    None,
    S0,
    S2Unauthenticated,
    S2Authenticated,
}

#[derive(Debug, Clone)]
pub struct ZWaveDevice {
    pub node_id: u8,
    pub device_type: u8,
    pub signature: String,
    pub security: SecurityLevel,
}

impl AetherZWave {
    pub fn new(home_id: u32, node_id: u8) -> Self {
        Self {
            home_id,
            node_id,
            devices: Arc::new(DashMap::new()),
            security_level: SecurityLevel::S2Authenticated,
        }
    }

    pub fn add_device(&self, node_id: u8, device_type: u8) -> Result<()> {
        let device = ZWaveDevice {
            node_id,
            device_type,
            signature: format!("zw_{:02x}_{:02x}", node_id, device_type),
            security: self.security_level,
        };
        self.devices.insert(node_id, device);
        tracing::info!("Z-Wave device added: node {}", node_id);
        Ok(())
    }

    pub fn send_message(&self, target: u8, command: u8, _data: &[u8]) -> Result<()> {
        if let Some(_device) = self.devices.get(&target) {
            tracing::info!("Sending Z-Wave message to node {}, command {}", target, command);
            Ok(())
        } else {
            Err(IotError::DeviceNotFound(format!("node_{}", target)))
        }
    }

    pub fn get_security_level(&self) -> SecurityLevel {
        self.security_level
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    pub fn get_device(&self, node_id: u8) -> Option<ZWaveDevice> {
        self.devices.get(&node_id).map(|d| d.clone())
    }

    pub fn remove_device(&self, node_id: u8) -> Option<ZWaveDevice> {
        self.devices.remove(&node_id).map(|(_, d)| d)
    }

    pub fn list_devices(&self) -> Vec<ZWaveDevice> {
        self.devices.iter().map(|ref_| ref_.value().clone()).collect()
    }

    pub fn heal_network(&self) {
        for device in self.devices.iter() {
            tracing::info!("Healing Z-Wave node {}", device.node_id);
        }
    }

    pub fn update_device_security(&self, node_id: u8, level: SecurityLevel) -> Result<()> {
        if let Some(mut device) = self.devices.get_mut(&node_id) {
            device.security = level;
            Ok(())
        } else {
            Err(IotError::DeviceNotFound(format!("node_{}", node_id)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aether_zwave_creation() {
        let zw = AetherZWave::new(0x12345678, 1);
        assert_eq!(zw.home_id, 0x12345678);
    }

    #[test]
    fn test_add_device() {
        let zw = AetherZWave::new(0x12345678, 1);
        assert!(zw.add_device(2, 0x10).is_ok());
        assert_eq!(zw.device_count(), 1);
    }

    #[test]
    fn test_send_message() {
        let zw = AetherZWave::new(0x12345678, 1);
        zw.add_device(2, 0x10).unwrap();
        assert!(zw.send_message(2, 0x20, &[1, 2, 3]).is_ok());
    }

    #[test]
    fn test_security_level() {
        let zw = AetherZWave::new(0x12345678, 1);
        assert_eq!(zw.get_security_level(), SecurityLevel::S2Authenticated);
    }
}
