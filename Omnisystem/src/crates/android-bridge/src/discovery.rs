use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use uuid::Uuid;

/// Device discovered via mDNS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    /// Unique device identifier
    pub device_id: String,
    /// Human-readable device name
    pub name: String,
    /// Device model (e.g., "Pixel 6", "Galaxy S21")
    pub model: String,
    /// Android API level
    pub api_level: u32,
    /// Device IP address
    pub ip: String,
    /// Bridge listening port
    pub port: u16,
    /// Public key for capability-based pairing
    pub public_key: String,
    /// Last seen timestamp
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

/// mDNS service type for Android devices
const MDNS_SERVICE: &str = "_bonsai-android._tcp.local.";

/// Discovery service for finding Android devices on local network
pub struct DiscoveryService {
    devices: Arc<parking_lot::RwLock<Vec<DiscoveredDevice>>>,
    discovery_interval: std::time::Duration,
}

impl DiscoveryService {
    /// Create new discovery service
    pub fn new(discovery_interval: std::time::Duration) -> Self {
        Self {
            devices: Arc::new(parking_lot::RwLock::new(Vec::new())),
            discovery_interval,
        }
    }

    /// Start discovery service (broadcasts mDNS queries)
    pub async fn start(&self) -> Result<()> {
        let devices = self.devices.clone();
        let interval = self.discovery_interval;

        tokio::spawn(async move {
            loop {
                // In production, use mdns-sd or similar library for proper mDNS
                // This is a simplified stub that demonstrates the pattern
                if let Ok(discovered) = Self::mdns_query().await {
                    let mut device_list = devices.write();
                    for new_device in discovered {
                        // Update or add device
                        if let Some(pos) = device_list
                            .iter()
                            .position(|d| d.device_id == new_device.device_id)
                        {
                            device_list[pos] = new_device;
                        } else {
                            device_list.push(new_device);
                        }
                    }
                }

                tokio::time::sleep(interval).await;
            }
        });

        Ok(())
    }

    /// Get all discovered devices
    pub fn get_devices(&self) -> Vec<DiscoveredDevice> {
        self.devices.read().clone()
    }

    /// Find device by ID
    pub fn find_device(&self, device_id: &str) -> Option<DiscoveredDevice> {
        self.devices
            .read()
            .iter()
            .find(|d| d.device_id == device_id)
            .cloned()
    }

    /// Find device by name
    pub fn find_device_by_name(&self, name: &str) -> Option<DiscoveredDevice> {
        self.devices
            .read()
            .iter()
            .find(|d| d.name == name)
            .cloned()
    }

    /// Remove stale devices (not seen for more than 5 minutes)
    pub fn cleanup_stale(&self) {
        let mut devices = self.devices.write();
        let now = chrono::Utc::now();
        devices.retain(|d| {
            (now - d.last_seen).num_seconds() < 300
        });
    }

    // Stub for mDNS query (would use mdns-sd crate in production)
    async fn mdns_query() -> Result<Vec<DiscoveredDevice>> {
        // Placeholder - actual implementation would:
        // 1. Send mDNS query for _bonsai-android._tcp.local.
        // 2. Parse responses
        // 3. Extract device info from TXT records
        Ok(Vec::new())
    }
}

/// Manual device registration (fallback when mDNS unavailable)
pub struct ManualDeviceRegistry {
    devices: Arc<parking_lot::RwLock<std::collections::HashMap<String, DiscoveredDevice>>>,
}

impl ManualDeviceRegistry {
    /// Create new manual device registry
    pub fn new() -> Self {
        Self {
            devices: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Register device manually (e.g., from config or user input)
    pub fn register(
        &self,
        device_id: String,
        name: String,
        model: String,
        api_level: u32,
        ip: String,
        port: u16,
        public_key: String,
    ) -> Result<()> {
        let device = DiscoveredDevice {
            device_id: device_id.clone(),
            name,
            model,
            api_level,
            ip,
            port,
            public_key,
            last_seen: chrono::Utc::now(),
        };

        self.devices.write().insert(device_id, device);
        Ok(())
    }

    /// Unregister device
    pub fn unregister(&self, device_id: &str) -> Result<()> {
        self.devices
            .write()
            .remove(device_id)
            .ok_or_else(|| crate::error::Error::DiscoveryError("Device not found".to_string()))?;
        Ok(())
    }

    /// Get all registered devices
    pub fn get_devices(&self) -> Vec<DiscoveredDevice> {
        self.devices.read().values().cloned().collect()
    }

    /// Get device by ID
    pub fn get_device(&self, device_id: &str) -> Option<DiscoveredDevice> {
        self.devices.read().get(device_id).cloned()
    }

    /// Update device last_seen
    pub fn update_last_seen(&self, device_id: &str) -> Result<()> {
        let mut devices = self.devices.write();
        if let Some(device) = devices.get_mut(device_id) {
            device.last_seen = chrono::Utc::now();
            Ok(())
        } else {
            Err(crate::error::Error::DiscoveryError(
                "Device not found".to_string(),
            ))
        }
    }
}

impl Default for ManualDeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manual_device_registry() {
        let registry = ManualDeviceRegistry::new();

        assert!(registry
            .register(
                "device1".to_string(),
                "Pixel 6".to_string(),
                "Pixel 6".to_string(),
                31,
                "192.168.1.100".to_string(),
                5037,
                "pk123".to_string(),
            )
            .is_ok());

        assert_eq!(registry.get_devices().len(), 1);
        assert!(registry.get_device("device1").is_some());
        assert!(registry.unregister("device1").is_ok());
        assert_eq!(registry.get_devices().len(), 0);
    }
}
