use crate::{Device, Message, Result, IotError, DeviceState, DeviceType};
use dashmap::DashMap;
use std::sync::Arc;

pub trait ProtocolHandler: Send + Sync {
    fn connect(&self, device: &Device) -> std::result::Result<(), String>;
    fn send(&self, message: &Message) -> std::result::Result<(), String>;
    fn receive(&self) -> Option<Message>;
    fn disconnect(&self, device_id: &str) -> std::result::Result<(), String>;
}

pub struct ProtocolManager {
    devices: Arc<DashMap<String, Device>>,
    messages: Arc<DashMap<String, Message>>,
}

impl ProtocolManager {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(DashMap::new()),
            messages: Arc::new(DashMap::new()),
        }
    }

    pub fn register_device(&self, device: Device) -> Result<()> {
        self.devices.insert(device.id.clone(), device);
        tracing::info!("Device registered");
        Ok(())
    }

    pub fn get_device(&self, id: &str) -> Result<Device> {
        self.devices
            .get(id)
            .map(|ref_| ref_.value().clone())
            .ok_or_else(|| IotError::DeviceNotFound(id.to_string()))
    }

    pub fn update_device_state(&self, id: &str, state: DeviceState) -> Result<()> {
        if let Some(mut device) = self.devices.get_mut(id) {
            device.state = state;
            Ok(())
        } else {
            Err(IotError::DeviceNotFound(id.to_string()))
        }
    }

    pub fn list_devices(&self) -> Vec<Device> {
        self.devices
            .iter()
            .map(|ref_| ref_.value().clone())
            .collect()
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    pub fn enqueue_message(&self, message: Message) -> Result<()> {
        self.messages.insert(message.id.clone(), message);
        Ok(())
    }

    pub fn dequeue_message(&self) -> Option<Message> {
        self.messages.iter().next().map(|ref_| {
            let msg = ref_.value().clone();
            drop(ref_);
            self.messages.remove(&msg.id);
            msg
        })
    }
}

impl Default for ProtocolManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_device(id: &str) -> Device {
        Device::new(
            id.to_string(),
            "Light".to_string(),
            DeviceType::Light,
            "TestMfg".to_string(),
            "TestModel".to_string(),
            "00:11:22:33:44:55".to_string(),
            "zigbee".to_string(),
        )
    }

    #[test]
    fn test_register_device() {
        let manager = ProtocolManager::new();
        let device = create_test_device("dev1");
        assert!(manager.register_device(device).is_ok());
    }

    #[test]
    fn test_get_device() {
        let manager = ProtocolManager::new();
        let device = create_test_device("dev1");
        manager.register_device(device).unwrap();
        assert!(manager.get_device("dev1").is_ok());
    }

    #[test]
    fn test_update_device_state() {
        let manager = ProtocolManager::new();
        let device = create_test_device("dev1");
        manager.register_device(device).unwrap();
        assert!(manager.update_device_state("dev1", DeviceState::Offline).is_ok());
    }

    #[test]
    fn test_device_count() {
        let manager = ProtocolManager::new();
        let device = create_test_device("dev1");
        manager.register_device(device).unwrap();
        assert_eq!(manager.device_count(), 1);
    }
}
