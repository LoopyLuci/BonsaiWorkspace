use crate::{Capability, Result, IotError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceType {
    Light,
    Thermostat,
    Lock,
    Sensor,
    Blind,
    Switch,
    Relay,
    Outlet,
    Custom(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceState {
    Online,
    Offline,
    Error,
    Pairing,
    Updating,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub state: DeviceState,
    pub manufacturer: String,
    pub model: String,
    pub firmware_version: String,
    pub protocol: String,
    pub ip_address: Option<String>,
    pub mac_address: String,
    pub capabilities: Vec<Capability>,
    pub properties: HashMap<String, serde_json::Value>,
    pub last_seen: u64,
    pub battery_level: Option<u8>,
    pub signal_strength: Option<i8>,
    pub parent_device: Option<String>,
}

impl Device {
    pub fn new(
        id: String,
        name: String,
        device_type: DeviceType,
        manufacturer: String,
        model: String,
        mac_address: String,
        protocol: String,
    ) -> Self {
        Device {
            id,
            name,
            device_type,
            state: DeviceState::Offline,
            manufacturer,
            model,
            firmware_version: "0.0.0".to_string(),
            protocol,
            ip_address: None,
            mac_address,
            capabilities: Vec::new(),
            properties: HashMap::new(),
            last_seen: 0,
            battery_level: None,
            signal_strength: None,
            parent_device: None,
        }
    }

    pub fn add_capability(&mut self, capability: Capability) -> Result<()> {
        if !self.capabilities.iter().any(|c| c.name == capability.name) {
            self.capabilities.push(capability);
            Ok(())
        } else {
            Err(IotError::DuplicateCapability(capability.name).into())
        }
    }

    pub fn set_state(&mut self, state: DeviceState) {
        self.state = state;
    }

    pub fn set_property(&mut self, key: String, value: serde_json::Value) {
        self.properties.insert(key, value);
    }

    pub fn get_property(&self, key: &str) -> Option<&serde_json::Value> {
        self.properties.get(key)
    }

    pub fn is_online(&self) -> bool {
        self.state == DeviceState::Online
    }

    pub fn has_capability(&self, name: &str) -> bool {
        self.capabilities.iter().any(|c| c.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_creation() {
        let device = Device::new(
            "device_1".to_string(),
            "Living Room Light".to_string(),
            DeviceType::Light,
            "Philips".to_string(),
            "Hue".to_string(),
            "00:11:22:33:44:55".to_string(),
            "zigbee".to_string(),
        );

        assert_eq!(device.id, "device_1");
        assert_eq!(device.device_type, DeviceType::Light);
        assert_eq!(device.state, DeviceState::Offline);
    }

    #[test]
    fn test_add_capability() {
        let mut device = Device::new(
            "device_1".to_string(),
            "Light".to_string(),
            DeviceType::Light,
            "Philips".to_string(),
            "Hue".to_string(),
            "00:11:22:33:44:55".to_string(),
            "zigbee".to_string(),
        );

        let cap = Capability {
            name: "brightness".to_string(),
            description: "Control brightness".to_string(),
            readable: true,
            writable: true,
            min_value: Some(0.0),
            max_value: Some(100.0),
        };

        assert!(device.add_capability(cap).is_ok());
        assert_eq!(device.capabilities.len(), 1);
    }

    #[test]
    fn test_device_state_transition() {
        let mut device = Device::new(
            "device_1".to_string(),
            "Light".to_string(),
            DeviceType::Light,
            "Philips".to_string(),
            "Hue".to_string(),
            "00:11:22:33:44:55".to_string(),
            "zigbee".to_string(),
        );

        assert!(!device.is_online());
        device.set_state(DeviceState::Online);
        assert!(device.is_online());
    }

    #[test]
    fn test_device_properties() {
        let mut device = Device::new(
            "device_1".to_string(),
            "Thermostat".to_string(),
            DeviceType::Thermostat,
            "Honeywell".to_string(),
            "T9".to_string(),
            "AA:BB:CC:DD:EE:FF".to_string(),
            "zigbee".to_string(),
        );

        device.set_property(
            "temperature".to_string(),
            serde_json::json!(22.5),
        );

        assert_eq!(
            device.get_property("temperature"),
            Some(&serde_json::json!(22.5))
        );
    }
}
