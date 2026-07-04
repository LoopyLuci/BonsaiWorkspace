use crate::{Protocol, Device, Message, Result, IotError};
use dashmap::DashMap;
use std::sync::Arc;

pub struct MultiProtocolRouter {
    device_registry: Arc<DashMap<String, (Protocol, String)>>,
    route_cache: Arc<DashMap<String, Protocol>>,
}

impl MultiProtocolRouter {
    pub fn new() -> Self {
        Self {
            device_registry: Arc::new(DashMap::new()),
            route_cache: Arc::new(DashMap::new()),
        }
    }

    pub fn register_device(&self, device_id: String, protocol: Protocol, handler_id: String) -> Result<()> {
        self.device_registry.insert(device_id, (protocol, handler_id));
        tracing::info!("Device registered for protocol: {:?}", protocol);
        Ok(())
    }

    pub fn route_message(&self, message: &Message) -> Result<Protocol> {
        if let Some(cached) = self.route_cache.get(&message.target) {
            return Ok(*cached.value());
        }

        if let Some(entry) = self.device_registry.get(&message.target) {
            let protocol = entry.0;
            self.route_cache.insert(message.target.clone(), protocol);
            Ok(protocol)
        } else {
            Err(IotError::DeviceNotFound(message.target.clone()))
        }
    }

    pub fn get_device_protocol(&self, device_id: &str) -> Result<Protocol> {
        self.device_registry
            .get(device_id)
            .map(|entry| entry.0)
            .ok_or_else(|| IotError::DeviceNotFound(device_id.to_string()))
    }

    pub fn device_count(&self) -> usize {
        self.device_registry.len()
    }

    pub fn clear_route_cache(&self) {
        self.route_cache.clear();
    }
}

impl Default for MultiProtocolRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let router = MultiProtocolRouter::new();
        assert_eq!(router.device_count(), 0);
    }

    #[test]
    fn test_register_device() {
        let router = MultiProtocolRouter::new();
        assert!(router.register_device(
            "dev1".to_string(),
            Protocol::Zigbee,
            "zigbee_handler".to_string()
        ).is_ok());
        assert_eq!(router.device_count(), 1);
    }

    #[test]
    fn test_get_device_protocol() {
        let router = MultiProtocolRouter::new();
        router.register_device(
            "dev1".to_string(),
            Protocol::ZWave,
            "zwave_handler".to_string()
        ).unwrap();
        let protocol = router.get_device_protocol("dev1").unwrap();
        assert_eq!(protocol, Protocol::ZWave);
    }

    #[test]
    fn test_multi_protocol_devices() {
        let router = MultiProtocolRouter::new();
        router.register_device("dev1".to_string(), Protocol::Zigbee, "h1".to_string()).unwrap();
        router.register_device("dev2".to_string(), Protocol::ZWave, "h2".to_string()).unwrap();
        router.register_device("dev3".to_string(), Protocol::BLE, "h3".to_string()).unwrap();
        assert_eq!(router.device_count(), 3);
    }
}
