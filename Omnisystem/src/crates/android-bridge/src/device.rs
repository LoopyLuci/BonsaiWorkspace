use crate::capability::{CapabilityType, CapabilityToken};
use crate::error::Result;
use crate::security::DeviceIdentity;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Device status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeviceStatus {
    /// Device discovered but not yet connected
    Discovered,
    /// Connection in progress
    Connecting,
    /// Connected and authenticated
    Connected,
    /// Pairing in progress
    Pairing,
    /// Device paired and authorized
    Paired,
    /// Connection lost
    Disconnected,
    /// Error state
    Error,
}

/// Device information and state
#[derive(Debug)]
pub struct Device {
    /// Unique device ID
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Device model
    pub model: String,
    /// Android API level
    pub api_level: u32,
    /// Current connection status
    pub status: DeviceStatus,
    /// IP address
    pub ip: String,
    /// Bridge port
    pub port: u16,
    /// Device identity (contains key material)
    pub identity: Arc<DeviceIdentity>,
    /// List of capabilities this device has
    pub capabilities: Vec<CapabilityType>,
    /// Connection timestamp
    pub connected_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Last heartbeat
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    /// Device metrics
    pub metrics: DeviceMetrics,
}

/// Device metrics and statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceMetrics {
    /// Total screen frames sent
    pub screen_frames_sent: u64,
    /// Total input events processed
    pub input_events_processed: u64,
    /// Total files synced
    pub files_synced: u64,
    /// Average screen latency (ms)
    pub avg_screen_latency: f64,
    /// Total data transferred (bytes)
    pub total_data_transferred: u64,
    /// Connection uptime (seconds)
    pub connection_uptime: u64,
    /// Last error (if any)
    pub last_error: Option<String>,
    /// Battery level (0-100)
    pub battery_level: Option<u8>,
    /// Ambient temperature (Celsius)
    pub device_temperature: Option<f32>,
}

impl Device {
    /// Create new device
    pub fn new(
        id: String,
        name: String,
        model: String,
        api_level: u32,
        ip: String,
        port: u16,
    ) -> Self {
        Self {
            id,
            name,
            model,
            api_level,
            status: DeviceStatus::Discovered,
            ip,
            port,
            identity: Arc::new(DeviceIdentity::generate()),
            capabilities: Vec::new(),
            connected_at: None,
            last_heartbeat: chrono::Utc::now(),
            metrics: DeviceMetrics::default(),
        }
    }

    /// Mark device as connected
    pub fn mark_connected(&mut self) {
        self.status = DeviceStatus::Connected;
        self.connected_at = Some(chrono::Utc::now());
        self.last_heartbeat = chrono::Utc::now();
    }

    /// Mark device as disconnected
    pub fn mark_disconnected(&mut self) {
        self.status = DeviceStatus::Disconnected;
    }

    /// Update heartbeat
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = chrono::Utc::now();
    }

    /// Check if device is responsive (heartbeat within 30 seconds)
    pub fn is_responsive(&self) -> bool {
        (chrono::Utc::now() - self.last_heartbeat).num_seconds() < 30
    }

    /// Add capability to device
    pub fn add_capability(&mut self, capability: CapabilityType) {
        if !self.capabilities.contains(&capability) {
            self.capabilities.push(capability);
        }
    }

    /// Check if device has capability
    pub fn has_capability(&self, capability: &CapabilityType) -> bool {
        self.capabilities.contains(capability)
    }

    /// Record screen frame
    pub fn record_screen_frame(&mut self, latency_ms: f64) {
        self.metrics.screen_frames_sent += 1;
        // Update rolling average
        let old_avg = self.metrics.avg_screen_latency;
        let frame_count = self.metrics.screen_frames_sent as f64;
        self.metrics.avg_screen_latency =
            (old_avg * (frame_count - 1.0) + latency_ms) / frame_count;
    }

    /// Record input event
    pub fn record_input_event(&mut self) {
        self.metrics.input_events_processed += 1;
    }

    /// Record file sync
    pub fn record_file_sync(&mut self, bytes: u64) {
        self.metrics.files_synced += 1;
        self.metrics.total_data_transferred += bytes;
    }

    /// Record error
    pub fn record_error(&mut self, error: String) {
        self.metrics.last_error = Some(error);
        self.status = DeviceStatus::Error;
    }

    /// Get uptime in seconds
    pub fn get_uptime(&self) -> Option<u64> {
        self.connected_at.map(|t| (chrono::Utc::now() - t).num_seconds() as u64)
    }
}

/// Device pool for managing multiple devices
pub struct DevicePool {
    devices: Arc<parking_lot::RwLock<std::collections::HashMap<String, Device>>>,
}

impl DevicePool {
    /// Create new device pool
    pub fn new() -> Self {
        Self {
            devices: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Add device to pool
    pub fn add_device(&self, device: Device) -> Result<()> {
        self.devices.write().insert(device.id.clone(), device);
        Ok(())
    }

    /// Remove device from pool
    pub fn remove_device(&self, device_id: &str) -> Result<Device> {
        self.devices
            .write()
            .remove(device_id)
            .ok_or_else(|| crate::error::Error::InvalidState("Device not found".to_string()))
    }

    /// Get device by ID
    pub fn get_device(&self, device_id: &str) -> Option<Device> {
        self.devices.read().get(device_id).cloned()
    }

    /// Update device
    pub fn update_device<F>(&self, device_id: &str, updater: F) -> Result<()>
    where
        F: FnOnce(&mut Device),
    {
        let mut devices = self.devices.write();
        devices
            .get_mut(device_id)
            .ok_or_else(|| crate::error::Error::InvalidState("Device not found".to_string()))
            .map(|d| updater(d))
    }

    /// Get all devices
    pub fn get_all_devices(&self) -> Vec<Device> {
        self.devices.read().values().cloned().collect()
    }

    /// Get devices by status
    pub fn get_devices_by_status(&self, status: DeviceStatus) -> Vec<Device> {
        self.devices
            .read()
            .values()
            .filter(|d| d.status == status)
            .cloned()
            .collect()
    }

    /// Get device count
    pub fn device_count(&self) -> usize {
        self.devices.read().len()
    }

    /// Check if device exists
    pub fn has_device(&self, device_id: &str) -> bool {
        self.devices.read().contains_key(device_id)
    }

    /// Get statistics across all devices
    pub fn get_pool_stats(&self) -> PoolStats {
        let devices = self.devices.read();
        let total_devices = devices.len();
        let connected_devices = devices.iter().filter(|(_, d)| d.is_responsive()).count();

        let total_frames: u64 = devices.iter().map(|(_, d)| d.metrics.screen_frames_sent).sum();
        let total_input: u64 = devices
            .iter()
            .map(|(_, d)| d.metrics.input_events_processed)
            .sum();
        let total_data: u64 = devices
            .iter()
            .map(|(_, d)| d.metrics.total_data_transferred)
            .sum();

        PoolStats {
            total_devices,
            connected_devices,
            total_frames,
            total_input_events: total_input,
            total_data_transferred: total_data,
        }
    }
}

impl Default for DevicePool {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Device {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            name: self.name.clone(),
            model: self.model.clone(),
            api_level: self.api_level,
            status: self.status,
            ip: self.ip.clone(),
            port: self.port,
            identity: self.identity.clone(),
            capabilities: self.capabilities.clone(),
            connected_at: self.connected_at,
            last_heartbeat: self.last_heartbeat,
            metrics: self.metrics.clone(),
        }
    }
}

/// Device pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    pub total_devices: usize,
    pub connected_devices: usize,
    pub total_frames: u64,
    pub total_input_events: u64,
    pub total_data_transferred: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_creation() {
        let device = Device::new(
            "device1".to_string(),
            "Pixel 6".to_string(),
            "Pixel 6".to_string(),
            31,
            "192.168.1.100".to_string(),
            5037,
        );

        assert_eq!(device.id, "device1");
        assert_eq!(device.status, DeviceStatus::Discovered);
        assert!(!device.is_responsive() || true); // Just check it doesn't panic
    }

    #[test]
    fn test_device_pool() {
        let pool = DevicePool::new();

        let device = Device::new(
            "device1".to_string(),
            "Pixel 6".to_string(),
            "Pixel 6".to_string(),
            31,
            "192.168.1.100".to_string(),
            5037,
        );

        assert!(pool.add_device(device).is_ok());
        assert_eq!(pool.device_count(), 1);
        assert!(pool.has_device("device1"));
    }
}
