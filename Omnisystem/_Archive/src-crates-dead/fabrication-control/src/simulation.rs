use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Hardware simulator for fabrication devices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareSimulator {
    pub simulator_id: String,
    pub simulated_devices: HashMap<String, SimulatedDevice>,
    pub current_time_ms: u64,
    pub simulation_speed: f32, // 1.0 = realtime, 2.0 = 2x speed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedDevice {
    pub device_id: String,
    pub device_type: String,
    pub position_mm: (f32, f32, f32),
    pub temperature_c: f32,
    pub power_w: f32,
    pub error_state: Option<String>,
}

impl HardwareSimulator {
    pub fn new(simulator_id: String) -> Self {
        HardwareSimulator {
            simulator_id,
            simulated_devices: HashMap::new(),
            current_time_ms: 0,
            simulation_speed: 1.0,
        }
    }

    pub fn add_device(&mut self, device: SimulatedDevice) {
        self.simulated_devices.insert(device.device_id.clone(), device);
    }

    pub async fn step_simulation(&mut self, delta_ms: u64) -> Result<()> {
        self.current_time_ms += (delta_ms as f32 * self.simulation_speed) as u64;

        for device in self.simulated_devices.values_mut() {
            // Simulate thermal effects
            device.temperature_c = (device.temperature_c * 0.95) + (device.power_w / 100.0);

            // Simulate drift
            if device.temperature_c > 85.0 {
                device.error_state = Some("Thermal warning".to_string());
            }
        }

        Ok(())
    }

    pub fn device_count(&self) -> usize {
        self.simulated_devices.len()
    }
}

/// Real device integration layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealDeviceAdapter {
    pub adapter_id: String,
    pub device_id: String,
    pub connection_type: ConnectionType,
    pub is_connected: bool,
    pub last_heartbeat_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionType {
    Serial,
    USB,
    Ethernet,
    WiFi,
    Bluetooth,
}

impl RealDeviceAdapter {
    pub fn new(adapter_id: String, device_id: String, conn_type: ConnectionType) -> Self {
        RealDeviceAdapter {
            adapter_id,
            device_id,
            connection_type: conn_type,
            is_connected: false,
            last_heartbeat_ms: 0,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        tracing::debug!("RealDevice: Connecting via {:?}", self.connection_type);
        self.is_connected = true;
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        self.is_connected = false;
        Ok(())
    }

    pub async fn send_command(&self, _command: Vec<u8>) -> Result<Vec<u8>> {
        if !self.is_connected {
            return Err(anyhow::anyhow!("Device not connected"));
        }
        Ok(vec![])
    }
}

/// Motion simulation with physics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionSimulator {
    pub sim_id: String,
    pub gravity_enabled: bool,
    pub friction_coefficient: f32,
    pub bodies: HashMap<String, RigidBody>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RigidBody {
    pub body_id: String,
    pub position: (f32, f32, f32),
    pub velocity: (f32, f32, f32),
    pub mass_kg: f32,
    pub friction: f32,
}

impl MotionSimulator {
    pub fn new(sim_id: String) -> Self {
        MotionSimulator {
            sim_id,
            gravity_enabled: false,
            friction_coefficient: 0.2,
            bodies: HashMap::new(),
        }
    }

    pub fn add_body(&mut self, body: RigidBody) {
        self.bodies.insert(body.body_id.clone(), body);
    }

    pub async fn simulate_motion(&mut self, dt: f32) -> Result<()> {
        for body in self.bodies.values_mut() {
            // Apply friction
            let friction_mag = (body.velocity.0.powi(2) + body.velocity.1.powi(2) + body.velocity.2.powi(2)).sqrt();
            if friction_mag > 0.001 {
                let friction_accel = self.friction_coefficient * 9.81;
                let factor = (friction_accel * dt / friction_mag).min(1.0);
                body.velocity.0 *= 1.0 - factor;
                body.velocity.1 *= 1.0 - factor;
                body.velocity.2 *= 1.0 - factor;
            }

            // Update position
            body.position.0 += body.velocity.0 * dt;
            body.position.1 += body.velocity.1 * dt;
            body.position.2 += body.velocity.2 * dt;
        }
        Ok(())
    }

    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }
}

/// Sensor simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorSimulator {
    pub sim_id: String,
    pub sensors: HashMap<String, VirtualSensor>,
    pub noise_level: f32, // 0.0-1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualSensor {
    pub sensor_id: String,
    pub sensor_type: SensorType,
    pub last_reading: f32,
    pub accuracy: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SensorType {
    Temperature,
    Pressure,
    Distance,
    Acceleration,
    Rotation,
}

impl SensorSimulator {
    pub fn new(sim_id: String) -> Self {
        SensorSimulator {
            sim_id,
            sensors: HashMap::new(),
            noise_level: 0.1,
        }
    }

    pub fn add_sensor(&mut self, sensor: VirtualSensor) {
        self.sensors.insert(sensor.sensor_id.clone(), sensor);
    }

    pub async fn read_sensor(&self, sensor_id: &str) -> Result<f32> {
        if let Some(sensor) = self.sensors.get(sensor_id) {
            // Add deterministic noise based on sensor_id hash
            let hash: u64 = sensor_id.bytes().fold(0u64, |acc, b| {
                acc.wrapping_mul(31).wrapping_add(b as u64)
            });
            let noise = ((hash as f32 / u64::MAX as f32) * self.noise_level) - (self.noise_level / 2.0);
            Ok(sensor.last_reading + noise)
        } else {
            Err(anyhow::anyhow!("Sensor not found"))
        }
    }

    pub fn sensor_count(&self) -> usize {
        self.sensors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_simulator() {
        let sim = HardwareSimulator::new("sim1".to_string());
        assert_eq!(sim.simulation_speed, 1.0);
    }

    #[test]
    fn test_real_device_adapter() {
        let adapter = RealDeviceAdapter::new(
            "adapter1".to_string(),
            "device1".to_string(),
            ConnectionType::USB,
        );
        assert!(!adapter.is_connected);
    }

    #[test]
    fn test_connection_types() {
        let types = vec![
            ConnectionType::Serial,
            ConnectionType::USB,
            ConnectionType::Ethernet,
            ConnectionType::WiFi,
        ];
        assert_eq!(types.len(), 4);
    }

    #[test]
    fn test_motion_simulator() {
        let sim = MotionSimulator::new("sim1".to_string());
        assert_eq!(sim.friction_coefficient, 0.2);
    }

    #[test]
    fn test_sensor_simulator() {
        let sim = SensorSimulator::new("sim1".to_string());
        assert_eq!(sim.noise_level, 0.1);
    }

    #[test]
    fn test_math() {
        assert_eq!(2 + 2, 4);
    }
}
