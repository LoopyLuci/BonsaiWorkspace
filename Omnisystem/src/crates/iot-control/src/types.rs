use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Protocol {
    Zigbee,
    ZWave,
    Thread,
    BLE,
    WiFi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceState {
    Offline,
    Online,
    Connecting,
    Connected,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub protocol: Protocol,
    pub state: DeviceState,
    pub rssi: i8,
    pub last_seen: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub source: String,
    pub target: String,
    pub protocol: Protocol,
    pub payload: Vec<u8>,
    pub sequence: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolConfig {
    pub protocol: Protocol,
    pub channel: u8,
    pub power: i8,
    pub timeout_ms: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_creation() {
        let device = Device {
            id: "dev1".to_string(),
            name: "Light".to_string(),
            protocol: Protocol::Zigbee,
            state: DeviceState::Online,
            rssi: -50,
            last_seen: 1000,
        };
        assert_eq!(device.protocol, Protocol::Zigbee);
    }

    #[test]
    fn test_protocol_config() {
        let config = ProtocolConfig {
            protocol: Protocol::ZWave,
            channel: 1,
            power: 0,
            timeout_ms: 5000,
        };
        assert_eq!(config.protocol, Protocol::ZWave);
    }

    #[test]
    fn test_message_creation() {
        let msg = Message {
            id: Uuid::new_v4().to_string(),
            source: "node1".to_string(),
            target: "node2".to_string(),
            protocol: Protocol::Thread,
            payload: vec![1, 2, 3],
            sequence: 1,
        };
        assert_eq!(msg.sequence, 1);
    }
}
