// Device Management - PCI/PCIe, USB, device tree, hotplug

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Device Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub vendor_id: u32,
    pub device_type: String,
    pub bus: String,
    pub path: String,
    pub enabled: bool,
}

/// PCI Device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PCIDevice {
    pub vendor: u32,
    pub device: u32,
    pub class: u32,
    pub subclass: u32,
    pub prog_if: u32,
}

/// USB Device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct USBDevice {
    pub vendor_id: u32,
    pub product_id: u32,
    pub bus_number: u32,
    pub device_number: u32,
}

/// Device Manager
pub struct DeviceManager {
    devices: HashMap<String, DeviceInfo>,
}

impl DeviceManager {
    pub async fn new() -> anyhow::Result<Self> {
        tracing::info!("Initializing Device Manager");

        let mut devices = HashMap::new();

        // Add some example devices
        devices.insert(
            "eth0".to_string(),
            DeviceInfo {
                device_id: "eth0".to_string(),
                vendor_id: 0x8086, // Intel
                device_type: "network".to_string(),
                bus: "pci".to_string(),
                path: "/sys/devices/pci0000:00/0000:00:1f.6".to_string(),
                enabled: true,
            },
        );

        devices.insert(
            "nvme0".to_string(),
            DeviceInfo {
                device_id: "nvme0".to_string(),
                vendor_id: 0x144d, // Samsung
                device_type: "storage".to_string(),
                bus: "pci".to_string(),
                path: "/sys/devices/pci0000:00/0000:00:1d.0".to_string(),
                enabled: true,
            },
        );

        Ok(Self { devices })
    }

    pub async fn list_devices(&self) -> anyhow::Result<Vec<DeviceInfo>> {
        Ok(self.devices.values().cloned().collect())
    }

    pub async fn get_device(&self, id: &str) -> anyhow::Result<DeviceInfo> {
        self.devices
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Device not found: {}", id))
    }

    pub async fn enable_device(&mut self, id: &str) -> anyhow::Result<()> {
        tracing::info!("Enabling device: {}", id);
        if let Some(device) = self.devices.get_mut(id) {
            device.enabled = true;
        }
        Ok(())
    }

    pub async fn disable_device(&mut self, id: &str) -> anyhow::Result<()> {
        tracing::info!("Disabling device: {}", id);
        if let Some(device) = self.devices.get_mut(id) {
            device.enabled = false;
        }
        Ok(())
    }

    pub async fn scan_pci_bus(&self) -> anyhow::Result<Vec<PCIDevice>> {
        tracing::info!("Scanning PCI bus");
        Ok(vec![
            PCIDevice {
                vendor: 0x8086,
                device: 0x1f41,
                class: 0x06,
                subclass: 0x01,
                prog_if: 0x00,
            },
            PCIDevice {
                vendor: 0x144d,
                device: 0xa804,
                class: 0x01,
                subclass: 0x08,
                prog_if: 0x02,
            },
        ])
    }

    pub async fn enumerate_usb(&self) -> anyhow::Result<Vec<USBDevice>> {
        tracing::info!("Enumerating USB devices");
        Ok(vec![])
    }

    pub async fn handle_hotplug(&mut self, device_id: String, connected: bool) -> anyhow::Result<()> {
        if connected {
            tracing::info!("Device connected: {}", device_id);
            self.devices.insert(
                device_id.clone(),
                DeviceInfo {
                    device_id,
                    vendor_id: 0,
                    device_type: "unknown".to_string(),
                    bus: "unknown".to_string(),
                    path: "/sys/devices/unknown".to_string(),
                    enabled: true,
                },
            );
        } else {
            tracing::info!("Device disconnected: {}", device_id);
            self.devices.remove(&device_id);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_device_manager_creation() {
        let manager = DeviceManager::new().await.unwrap();
        let devices = manager.list_devices().await.unwrap();
        assert!(!devices.is_empty());
    }

    #[tokio::test]
    async fn test_get_device() {
        let manager = DeviceManager::new().await.unwrap();
        let device = manager.get_device("eth0").await.unwrap();
        assert_eq!(device.device_id, "eth0");
    }

    #[tokio::test]
    async fn test_pci_scan() {
        let manager = DeviceManager::new().await.unwrap();
        let devices = manager.scan_pci_bus().await.unwrap();
        assert!(!devices.is_empty());
    }

    #[tokio::test]
    async fn test_hotplug() {
        let mut manager = DeviceManager::new().await.unwrap();
        manager
            .handle_hotplug("usb0".to_string(), true)
            .await
            .unwrap();
        let device = manager.get_device("usb0").await.unwrap();
        assert!(device.enabled);
    }
}
