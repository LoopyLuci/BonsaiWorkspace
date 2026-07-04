use crate::{ProtocolHandler, Message, Device};

pub struct AetherZWave {
    home_id: u32,
    node_id: u8,
}

impl AetherZWave {
    pub fn new(home_id: u32, node_id: u8) -> Self {
        Self { home_id, node_id }
    }

    pub fn get_home_id(&self) -> u32 {
        self.home_id
    }

    pub fn get_node_id(&self) -> u8 {
        self.node_id
    }
}

impl ProtocolHandler for AetherZWave {
    fn connect(&self, device: &Device) -> std::result::Result<(), String> {
        tracing::info!("Connecting Z-Wave device: {}", device.id);
        Ok(())
    }

    fn send(&self, message: &Message) -> std::result::Result<(), String> {
        tracing::info!("Sending Z-Wave message: {}", message.id);
        Ok(())
    }

    fn receive(&self) -> Option<Message> {
        None
    }

    fn disconnect(&self, device_id: &str) -> std::result::Result<(), String> {
        tracing::info!("Disconnecting Z-Wave device: {}", device_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aether_zwave_creation() {
        let zwave = AetherZWave::new(0x12345678, 1);
        assert_eq!(zwave.get_home_id(), 0x12345678);
        assert_eq!(zwave.get_node_id(), 1);
    }
}
