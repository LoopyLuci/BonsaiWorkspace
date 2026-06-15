use crate::{ProtocolHandler, Message, Device};

pub struct BLEProtocol {
    mtu: u16,
    timeout_ms: u32,
}

impl BLEProtocol {
    pub fn new(mtu: u16, timeout_ms: u32) -> Self {
        Self { mtu, timeout_ms }
    }

    pub fn get_mtu(&self) -> u16 {
        self.mtu
    }
}

impl ProtocolHandler for BLEProtocol {
    fn connect(&self, device: &Device) -> std::result::Result<(), String> {
        tracing::info!("Connecting BLE device: {}", device.id);
        Ok(())
    }

    fn send(&self, message: &Message) -> std::result::Result<(), String> {
        if message.payload.len() > self.mtu as usize {
            return Err("Payload exceeds MTU".to_string());
        }
        tracing::info!("Sending BLE message: {}", message.id);
        Ok(())
    }

    fn receive(&self) -> Option<Message> {
        None
    }

    fn disconnect(&self, device_id: &str) -> std::result::Result<(), String> {
        tracing::info!("Disconnecting BLE device: {}", device_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ble_protocol_creation() {
        let ble = BLEProtocol::new(247, 5000);
        assert_eq!(ble.get_mtu(), 247);
    }
}
