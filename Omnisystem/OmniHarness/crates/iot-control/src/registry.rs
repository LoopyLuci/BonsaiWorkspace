use crate::{Device, DeviceType, Result, IotError};
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct DeviceRegistry {
    devices: Arc<DashMap<String, Device>>,
    devices_by_type: Arc<DashMap<String, Vec<String>>>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        DeviceRegistry {
            devices: Arc::new(DashMap::new()),
            devices_by_type: Arc::new(DashMap::new()),
        }
    }

    pub fn register(&self, device: Device) -> Result<()> {
        let device_id = device.id.clone();
        let device_type = format!("{:?}", device.device_type);

        self.devices.insert(device_id.clone(), device);

        self.devices_by_type
            .entry(device_type)
            .or_insert_with(Vec::new)
            .push(device_id);

        Ok(())
    }

    pub fn unregister(&self, device_id: &str) -> Result<()> {
        if let Some((_, device)) = self.devices.remove(device_id) {
            let device_type = format!("{:?}", device.device_type);
            if let Some(mut types) = self.devices_by_type.get_mut(&device_type) {
                types.retain(|id| id != device_id);
            }
            Ok(())
        } else {
            Err(IotError::DeviceNotFound(device_id.to_string()).into())
        }
    }

    pub fn get(&self, device_id: &str) -> Option<Device> {
        self.devices.get(device_id).map(|ref_| ref_.value().clone())
    }

    pub fn get_mut_device<F>(&self, device_id: &str, f: F) -> Option<()>
    where
        F: FnOnce(&mut Device),
    {
        if let Some(mut device) = self.devices.get_mut(device_id) {
            f(&mut device);
            Some(())
        } else {
            None
        }
    }

    pub fn list_all(&self) -> Vec<Device> {
        self.devices
            .iter()
            .map(|ref_| ref_.value().clone())
            .collect()
    }

    pub fn list_by_type(&self, device_type: DeviceType) -> Vec<Device> {
        let type_key = format!("{:?}", device_type);
        if let Some(ids) = self.devices_by_type.get(&type_key) {
            ids.iter()
                .filter_map(|id| {
                    self.devices.get(id).map(|ref_| ref_.value().clone())
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn list_by_protocol(&self, protocol: &str) -> Vec<Device> {
        self.devices
            .iter()
            .filter(|ref_| ref_.value().protocol == protocol)
            .map(|ref_| ref_.value().clone())
            .collect()
    }

    pub fn count(&self) -> usize {
        self.devices.len()
    }

    pub fn exists(&self, device_id: &str) -> bool {
        self.devices.contains_key(device_id)
    }

    pub fn update(&self, device_id: &str, update_fn: impl FnOnce(&mut Device)) -> Result<()> {
        if let Some(mut device) = self.devices.get_mut(device_id) {
            update_fn(&mut device);
            Ok(())
        } else {
            Err(IotError::DeviceNotFound(device_id.to_string()).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_device(id: &str) -> Device {
        Device::new(
            id.to_string(),
            format!("Device {}", id),
            DeviceType::Light,
            "TestManufacturer".to_string(),
            "TestModel".to_string(),
            "00:11:22:33:44:55".to_string(),
            "zigbee".to_string(),
        )
    }

    #[test]
    fn test_register_device() {
        let registry = DeviceRegistry::new();
        let device = create_test_device("device_1");

        assert!(registry.register(device).is_ok());
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_get_device() {
        let registry = DeviceRegistry::new();
        let device = create_test_device("device_1");
        let device_id = device.id.clone();

        registry.register(device).unwrap();

        let retrieved = registry.get(&device_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, device_id);
    }

    #[test]
    fn test_unregister_device() {
        let registry = DeviceRegistry::new();
        let device = create_test_device("device_1");

        registry.register(device).unwrap();
        assert_eq!(registry.count(), 1);

        registry.unregister("device_1").unwrap();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_list_by_type() {
        let registry = DeviceRegistry::new();

        registry.register(create_test_device("light_1")).unwrap();
        registry.register(create_test_device("light_2")).unwrap();

        let lights = registry.list_by_type(DeviceType::Light);
        assert_eq!(lights.len(), 2);
    }

    #[test]
    fn test_list_by_protocol() {
        let registry = DeviceRegistry::new();
        let device1 = create_test_device("device_1");
        let mut device2 = create_test_device("device_2");
        device2.protocol = "zwave".to_string();

        registry.register(device1).unwrap();
        registry.register(device2).unwrap();

        let zigbee_devices = registry.list_by_protocol("zigbee");
        assert_eq!(zigbee_devices.len(), 1);

        let zwave_devices = registry.list_by_protocol("zwave");
        assert_eq!(zwave_devices.len(), 1);
    }

    #[test]
    fn test_device_not_found() {
        let registry = DeviceRegistry::new();
        let result = registry.get("nonexistent");
        assert!(result.is_none());
    }
}
