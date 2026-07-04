//! Peripheral emulation (GPU, USB, sensors)

use serde::{Deserialize, Serialize};

/// GPU emulator trait and implementations
pub trait GPUEmulator: Send + Sync {
    /// Submit a compute kernel
    fn submit_kernel(&self, kernel_id: u32, grid_size: (u32, u32, u32)) -> crate::errors::EmulationResult<()>;

    /// Wait for kernel completion
    fn wait_kernel(&self) -> crate::errors::EmulationResult<()>;

    /// Get GPU memory info
    fn get_memory_info(&self) -> (u64, u64); // (used, available)
}

/// USB device type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum USBDeviceType {
    /// Storage device
    Storage,
    /// Input device (keyboard, mouse)
    Input,
    /// Audio device
    Audio,
    /// Video device (camera, display)
    Video,
    /// Other
    Other,
}

/// USB device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct USBDevice {
    /// Device ID
    pub id: u16,
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
    /// Device type
    pub device_type: USBDeviceType,
    /// Device name
    pub name: String,
}

impl USBDevice {
    /// Create a new USB device
    pub fn new(
        id: u16,
        vendor_id: u16,
        product_id: u16,
        device_type: USBDeviceType,
        name: String,
    ) -> Self {
        Self {
            id,
            vendor_id,
            product_id,
            device_type,
            name,
        }
    }
}

/// USB emulator
pub struct USBEmulator {
    /// Connected devices
    devices: Vec<USBDevice>,
}

impl USBEmulator {
    /// Create a new USB emulator
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    /// Connect a device
    pub fn connect_device(&mut self, device: USBDevice) {
        self.devices.push(device);
    }

    /// Disconnect a device
    pub fn disconnect_device(&mut self, device_id: u16) -> Option<USBDevice> {
        self.devices
            .iter()
            .position(|d| d.id == device_id)
            .map(|idx| self.devices.remove(idx))
    }

    /// Get list of connected devices
    pub fn list_devices(&self) -> Vec<&USBDevice> {
        self.devices.iter().collect()
    }

    /// Get device by ID
    pub fn get_device(&self, device_id: u16) -> Option<&USBDevice> {
        self.devices.iter().find(|d| d.id == device_id)
    }
}

impl Default for USBEmulator {
    fn default() -> Self {
        Self::new()
    }
}

/// Sensor type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensorType {
    /// Temperature sensor
    Temperature,
    /// Accelerometer
    Accelerometer,
    /// Gyroscope
    Gyroscope,
    /// Magnetometer
    Magnetometer,
    /// Proximity sensor
    Proximity,
    /// Light sensor
    Light,
    /// Pressure sensor
    Pressure,
}

impl std::fmt::Display for SensorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Temperature => write!(f, "Temperature"),
            Self::Accelerometer => write!(f, "Accelerometer"),
            Self::Gyroscope => write!(f, "Gyroscope"),
            Self::Magnetometer => write!(f, "Magnetometer"),
            Self::Proximity => write!(f, "Proximity"),
            Self::Light => write!(f, "Light"),
            Self::Pressure => write!(f, "Pressure"),
        }
    }
}

/// Sensor reading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    /// Sensor type
    pub sensor_type: SensorType,
    /// Reading values
    pub values: Vec<f64>,
    /// Timestamp in microseconds
    pub timestamp_us: u64,
    /// Accuracy
    pub accuracy: f64,
}

impl SensorReading {
    /// Create a new sensor reading
    pub fn new(
        sensor_type: SensorType,
        values: Vec<f64>,
        timestamp_us: u64,
        accuracy: f64,
    ) -> Self {
        Self {
            sensor_type,
            values,
            timestamp_us,
            accuracy,
        }
    }
}

/// Sensor emulator
pub struct SensorEmulator {
    /// Sensor readings
    readings: Vec<SensorReading>,
    /// Sensor enabled flags (using u8 as bit flags)
    enabled_flags: u8,
}

impl SensorEmulator {
    /// Create a new sensor emulator
    pub fn new() -> Self {
        Self {
            readings: Vec::new(),
            enabled_flags: 0,
        }
    }

    /// Enable a sensor
    pub fn enable_sensor(&mut self, sensor_type: SensorType) {
        let bit = sensor_type as u8;
        self.enabled_flags |= 1 << bit;
    }

    /// Disable a sensor
    pub fn disable_sensor(&mut self, sensor_type: SensorType) {
        let bit = sensor_type as u8;
        self.enabled_flags &= !(1 << bit);
    }

    /// Check if sensor is enabled
    pub fn is_enabled(&self, sensor_type: SensorType) -> bool {
        let bit = sensor_type as u8;
        (self.enabled_flags & (1 << bit)) != 0
    }

    /// Add a sensor reading
    pub fn add_reading(&mut self, reading: SensorReading) {
        self.readings.push(reading);
    }

    /// Get latest reading for a sensor type
    pub fn get_latest_reading(&self, sensor_type: SensorType) -> Option<&SensorReading> {
        self.readings
            .iter()
            .rev()
            .find(|r| r.sensor_type == sensor_type)
    }

    /// Get all readings
    pub fn get_readings(&self) -> &[SensorReading] {
        &self.readings
    }
}

impl Default for SensorEmulator {
    fn default() -> Self {
        Self::new()
    }
}

/// General peripheral emulator
pub struct PeripheralEmulator {
    /// USB emulator
    pub usb: USBEmulator,
    /// Sensor emulator
    pub sensor: SensorEmulator,
}

impl PeripheralEmulator {
    /// Create a new peripheral emulator
    pub fn new() -> Self {
        Self {
            usb: USBEmulator::new(),
            sensor: SensorEmulator::new(),
        }
    }

    /// Reset all peripherals
    pub fn reset(&mut self) {
        self.usb = USBEmulator::new();
        self.sensor = SensorEmulator::new();
    }
}

impl Default for PeripheralEmulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usb_device_creation() {
        let device = USBDevice::new(1, 0x1234, 0x5678, USBDeviceType::Storage, "Test Device".to_string());
        assert_eq!(device.id, 1);
        assert_eq!(device.vendor_id, 0x1234);
    }

    #[test]
    fn test_usb_emulator_connect_disconnect() {
        let mut usb = USBEmulator::new();
        let device =
            USBDevice::new(1, 0x1234, 0x5678, USBDeviceType::Storage, "Test".to_string());
        usb.connect_device(device);

        assert_eq!(usb.list_devices().len(), 1);

        let disconnected = usb.disconnect_device(1);
        assert!(disconnected.is_some());
        assert_eq!(usb.list_devices().len(), 0);
    }

    #[test]
    fn test_usb_get_device() {
        let mut usb = USBEmulator::new();
        let device =
            USBDevice::new(1, 0x1234, 0x5678, USBDeviceType::Storage, "Test".to_string());
        usb.connect_device(device);

        let found = usb.get_device(1);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, 1);
    }

    #[test]
    fn test_sensor_type_display() {
        assert_eq!(SensorType::Temperature.to_string(), "Temperature");
        assert_eq!(SensorType::Accelerometer.to_string(), "Accelerometer");
    }

    #[test]
    fn test_sensor_reading_creation() {
        let reading = SensorReading::new(
            SensorType::Temperature,
            vec![25.5],
            1000,
            0.95,
        );
        assert_eq!(reading.sensor_type, SensorType::Temperature);
        assert_eq!(reading.values[0], 25.5);
    }

    #[test]
    fn test_sensor_emulator_enable_disable() {
        let mut emulator = SensorEmulator::new();
        emulator.enable_sensor(SensorType::Temperature);
        assert!(emulator.is_enabled(SensorType::Temperature));

        emulator.disable_sensor(SensorType::Temperature);
        assert!(!emulator.is_enabled(SensorType::Temperature));
    }

    #[test]
    fn test_sensor_emulator_readings() {
        let mut emulator = SensorEmulator::new();
        let reading = SensorReading::new(
            SensorType::Temperature,
            vec![25.5],
            1000,
            0.95,
        );
        emulator.add_reading(reading);

        assert_eq!(emulator.get_readings().len(), 1);
        let latest = emulator.get_latest_reading(SensorType::Temperature);
        assert!(latest.is_some());
    }

    #[test]
    fn test_peripheral_emulator_creation() {
        let peripheral = PeripheralEmulator::new();
        assert_eq!(peripheral.usb.list_devices().len(), 0);
    }
}
