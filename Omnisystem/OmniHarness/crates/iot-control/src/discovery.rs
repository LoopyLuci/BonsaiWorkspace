use crate::{Device, Result, IotError};
use dashmap::DashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryRequest {
    pub protocol: String,
    pub timeout_ms: u64,
    pub include_paired: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    pub device_id: String,
    pub name: String,
    pub protocol: String,
    pub mac_address: String,
    pub manufacturer: String,
    pub signal_strength: Option<i8>,
}

#[derive(Clone)]
pub struct DiscoveryService {
    discovered: Arc<DashMap<String, DiscoveryResult>>,
    paired_devices: Arc<DashMap<String, Device>>,
}

impl DiscoveryService {
    pub fn new() -> Self {
        DiscoveryService {
            discovered: Arc::new(DashMap::new()),
            paired_devices: Arc::new(DashMap::new()),
        }
    }

    pub async fn discover(&self, request: DiscoveryRequest) -> Result<Vec<DiscoveryResult>> {
        self.discovered.clear();

        // Simulate discovery based on protocol
        let results = match request.protocol.as_str() {
            "zigbee" => self.discover_zigbee().await?,
            "zwave" => self.discover_zwave().await?,
            "ble" => self.discover_ble().await?,
            "thread" => self.discover_thread().await?,
            "wifi" => self.discover_wifi().await?,
            _ => return Err(IotError::UnsupportedProtocol(request.protocol.clone()).into()),
        };

        for result in &results {
            self.discovered.insert(result.device_id.clone(), result.clone());
        }

        Ok(results)
    }

    async fn discover_zigbee(&self) -> Result<Vec<DiscoveryResult>> {
        // Simulated Zigbee discovery
        Ok(vec![
            DiscoveryResult {
                device_id: "zigbee_light_1".to_string(),
                name: "Bedroom Light".to_string(),
                protocol: "zigbee".to_string(),
                mac_address: "00:11:22:33:44:55".to_string(),
                manufacturer: "Philips".to_string(),
                signal_strength: Some(-35),
            },
            DiscoveryResult {
                device_id: "zigbee_sensor_1".to_string(),
                name: "Temperature Sensor".to_string(),
                protocol: "zigbee".to_string(),
                mac_address: "AA:BB:CC:DD:EE:FF".to_string(),
                manufacturer: "IKEA".to_string(),
                signal_strength: Some(-45),
            },
        ])
    }

    async fn discover_zwave(&self) -> Result<Vec<DiscoveryResult>> {
        // Simulated Z-Wave discovery
        Ok(vec![
            DiscoveryResult {
                device_id: "zwave_lock_1".to_string(),
                name: "Front Door Lock".to_string(),
                protocol: "zwave".to_string(),
                mac_address: "11:22:33:44:55:66".to_string(),
                manufacturer: "Yale".to_string(),
                signal_strength: Some(-40),
            },
        ])
    }

    async fn discover_ble(&self) -> Result<Vec<DiscoveryResult>> {
        Ok(vec![])
    }

    async fn discover_thread(&self) -> Result<Vec<DiscoveryResult>> {
        Ok(vec![])
    }

    async fn discover_wifi(&self) -> Result<Vec<DiscoveryResult>> {
        Ok(vec![])
    }

    pub fn get_discovered(&self, device_id: &str) -> Option<DiscoveryResult> {
        self.discovered.get(device_id).map(|ref_| ref_.value().clone())
    }

    pub fn list_discovered(&self) -> Vec<DiscoveryResult> {
        self.discovered
            .iter()
            .map(|ref_| ref_.value().clone())
            .collect()
    }

    pub fn pair_device(&self, device_id: &str, device: Device) -> Result<()> {
        self.paired_devices.insert(device_id.to_string(), device);
        Ok(())
    }

    pub fn unpair_device(&self, device_id: &str) -> Result<()> {
        self.paired_devices.remove(device_id);
        Ok(())
    }

    pub fn is_paired(&self, device_id: &str) -> bool {
        self.paired_devices.contains_key(device_id)
    }

    pub fn list_paired(&self) -> Vec<Device> {
        self.paired_devices
            .iter()
            .map(|ref_| ref_.value().clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discover_zigbee() {
        let service = DiscoveryService::new();
        let request = DiscoveryRequest {
            protocol: "zigbee".to_string(),
            timeout_ms: 5000,
            include_paired: false,
        };

        let results = service.discover(request).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].protocol, "zigbee");
    }

    #[tokio::test]
    async fn test_discover_zwave() {
        let service = DiscoveryService::new();
        let request = DiscoveryRequest {
            protocol: "zwave".to_string(),
            timeout_ms: 5000,
            include_paired: false,
        };

        let results = service.discover(request).await.unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_pair_device() {
        let service = DiscoveryService::new();
        let device = Device::new(
            "device_1".to_string(),
            "Test Device".to_string(),
            crate::DeviceType::Light,
            "TestMfg".to_string(),
            "TestModel".to_string(),
            "00:11:22:33:44:55".to_string(),
            "zigbee".to_string(),
        );

        assert!(service.pair_device("device_1", device).is_ok());
        assert!(service.is_paired("device_1"));
    }

    #[test]
    fn test_unpair_device() {
        let service = DiscoveryService::new();
        let device = Device::new(
            "device_1".to_string(),
            "Test Device".to_string(),
            crate::DeviceType::Light,
            "TestMfg".to_string(),
            "TestModel".to_string(),
            "00:11:22:33:44:55".to_string(),
            "zigbee".to_string(),
        );

        service.pair_device("device_1", device).unwrap();
        assert!(service.is_paired("device_1"));

        service.unpair_device("device_1").unwrap();
        assert!(!service.is_paired("device_1"));
    }
}
