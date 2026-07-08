use crate::{Device, DeviceType, Result, FabricationError};
use dashmap::DashMap;
use std::sync::Arc;

pub struct DeviceRegistry {
    devices: Arc<DashMap<String, Device>>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(DashMap::new()),
        }
    }

    pub fn register(&self, device: Device) -> Result<()> {
        self.devices.insert(device.id.clone(), device);
        tracing::info!("Device registered");
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<Device> {
        self.devices
            .get(id)
            .map(|ref_| ref_.value().clone())
            .ok_or_else(|| FabricationError::DeviceError(format!("Device not found: {}", id)))
    }

    pub fn update_status(&self, id: &str, online: bool) -> Result<()> {
        if let Some(mut device) = self.devices.get_mut(id) {
            device.online = online;
            Ok(())
        } else {
            Err(FabricationError::DeviceError(format!("Device not found: {}", id)))
        }
    }

    pub fn list_devices(&self) -> Vec<Device> {
        self.devices.iter().map(|ref_| ref_.value().clone()).collect()
    }

    pub fn list_by_type(&self, device_type: DeviceType) -> Vec<Device> {
        self.devices
            .iter()
            .filter(|ref_| ref_.value().device_type == device_type)
            .map(|ref_| ref_.value().clone())
            .collect()
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }
}

impl Default for DeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_device() {
        let registry = DeviceRegistry::new();
        let device = Device {
            id: "dev1".to_string(),
            name: "Printer".to_string(),
            device_type: DeviceType::FDMPrinter,
            model: "Prusa".to_string(),
            online: true,
            temperature: 200.0,
        };
        assert!(registry.register(device).is_ok());
        assert_eq!(registry.device_count(), 1);
    }

    #[test]
    fn test_get_device() {
        let registry = DeviceRegistry::new();
        let device = Device {
            id: "dev1".to_string(),
            name: "Printer".to_string(),
            device_type: DeviceType::FDMPrinter,
            model: "Prusa".to_string(),
            online: true,
            temperature: 200.0,
        };
        registry.register(device).unwrap();
        assert!(registry.get("dev1").is_ok());
    }

    #[test]
    fn test_list_by_type() {
        let registry = DeviceRegistry::new();
        let device = Device {
            id: "dev1".to_string(),
            name: "Printer".to_string(),
            device_type: DeviceType::FDMPrinter,
            model: "Prusa".to_string(),
            online: true,
            temperature: 200.0,
        };
        registry.register(device).unwrap();
        let devices = registry.list_by_type(DeviceType::FDMPrinter);
        assert_eq!(devices.len(), 1);
    }
}
