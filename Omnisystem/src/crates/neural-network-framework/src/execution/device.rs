//! Device discovery and management

use crate::error::Result;

/// Device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    CPU,
    CUDA,
    Metal,
    TPU,
}

/// Device information
#[derive(Debug, Clone)]
pub struct Device {
    pub device_type: DeviceType,
    pub device_id: usize,
    pub name: String,
    pub memory_total: u64,
    pub compute_capability: String,
}

/// Discover available devices
pub fn discover_devices() -> Result<Vec<Device>> {
    let mut devices = Vec::new();

    // CPU is always available
    devices.push(Device {
        device_type: DeviceType::CPU,
        device_id: 0,
        name: "CPU".to_string(),
        memory_total: 8 * 1024 * 1024 * 1024, // 8GB placeholder
        compute_capability: "generic".to_string(),
    });

    // TODO: Discover CUDA devices
    // TODO: Discover Metal devices
    // TODO: Discover TPU devices

    log::info!("Discovered {} device(s)", devices.len());

    Ok(devices)
}

/// Get default device
pub fn get_default_device() -> String {
    "cpu".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_devices() {
        let devices = discover_devices().unwrap();
        assert!(!devices.is_empty());
        assert_eq!(devices[0].device_type, DeviceType::CPU);
    }

    #[test]
    fn test_default_device() {
        let device = get_default_device();
        assert_eq!(device, "cpu");
    }
}
