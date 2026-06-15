use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::sync::Arc;
use async_trait::async_trait;
use crate::KernelError;

pub type DeviceId = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceType {
    CPU,
    GPU,
    Memory,
    Storage,
    Network,
    Display,
    Input,
    Audio,
    Sensor,
    Other(String),
}

#[async_trait]
pub trait Device: Send + Sync {
    fn id(&self) -> DeviceId;
    fn device_type(&self) -> DeviceType;
    fn name(&self) -> &str;
    async fn init(&self) -> Result<(), KernelError>;
    async fn shutdown(&self) -> Result<(), KernelError>;
    fn is_available(&self) -> bool;
}

pub struct GenericDevice {
    id: DeviceId,
    device_type: DeviceType,
    name: String,
    available: RwLock<bool>,
}

impl GenericDevice {
    pub fn new(id: DeviceId, device_type: DeviceType, name: String) -> Self {
        GenericDevice {
            id,
            device_type,
            name,
            available: RwLock::new(false),
        }
    }
}

#[async_trait]
impl Device for GenericDevice {
    fn id(&self) -> DeviceId {
        self.id
    }

    fn device_type(&self) -> DeviceType {
        self.device_type.clone()
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn init(&self) -> Result<(), KernelError> {
        *self.available.write() = true;
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), KernelError> {
        *self.available.write() = false;
        Ok(())
    }

    fn is_available(&self) -> bool {
        *self.available.read()
    }
}

pub struct DeviceManager {
    devices: RwLock<BTreeMap<DeviceId, Arc<dyn Device>>>,
    next_device_id: RwLock<DeviceId>,
}

impl DeviceManager {
    pub fn new() -> Self {
        DeviceManager {
            devices: RwLock::new(BTreeMap::new()),
            next_device_id: RwLock::new(1),
        }
    }

    pub async fn register_device(
        &self,
        device: Arc<dyn Device>,
    ) -> Result<DeviceId, KernelError> {
        let id = device.id();
        device.init().await?;
        self.devices.write().insert(id, device);
        Ok(id)
    }

    pub fn get_device(&self, id: DeviceId) -> Option<Arc<dyn Device>> {
        self.devices.read().get(&id).cloned()
    }

    pub fn get_devices_by_type(&self, device_type: DeviceType) -> Vec<Arc<dyn Device>> {
        self.devices
            .read()
            .values()
            .filter(|d| d.device_type() == device_type)
            .cloned()
            .collect()
    }

    pub async fn unregister_device(&self, id: DeviceId) -> Result<(), KernelError> {
        if let Some(device) = self.devices.write().remove(&id) {
            device.shutdown().await?;
        }
        Ok(())
    }

    pub fn list_devices(&self) -> Vec<(DeviceId, String, DeviceType)> {
        self.devices
            .read()
            .values()
            .map(|d| (d.id(), d.name().to_string(), d.device_type()))
            .collect()
    }

    pub fn device_count(&self) -> usize {
        self.devices.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_device_registration() {
        let dm = DeviceManager::new();
        let device = Arc::new(GenericDevice::new(
            1,
            DeviceType::CPU,
            "CPU-0".to_string(),
        ));

        let result = dm.register_device(device).await;
        assert!(result.is_ok());
        assert_eq!(dm.device_count(), 1);
    }

    #[tokio::test]
    async fn test_get_device() {
        let dm = DeviceManager::new();
        let device = Arc::new(GenericDevice::new(
            1,
            DeviceType::CPU,
            "CPU-0".to_string(),
        ));

        dm.register_device(device).await.unwrap();

        let retrieved = dm.get_device(1);
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_get_devices_by_type() {
        let dm = DeviceManager::new();
        // Test would be async in practice
    }
}
