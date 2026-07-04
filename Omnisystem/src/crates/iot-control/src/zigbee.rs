use crate::{ProtocolHandler, Message, Device, Result};

pub struct TitaniumZigbee {
    channel: u8,
    power: i8,
}

impl TitaniumZigbee {
    pub fn new(channel: u8, power: i8) -> Self {
        Self { channel, power }
    }

    pub fn get_channel(&self) -> u8 {
        self.channel
    }

    pub fn set_channel(&mut self, channel: u8) {
        self.channel = channel;
    }
}

impl ProtocolHandler for TitaniumZigbee {
    fn connect(&self, device: &Device) -> std::result::Result<(), String> {
        tracing::info!("Connecting Zigbee device: {}", device.id);
        Ok(())
    }

    fn send(&self, message: &Message) -> std::result::Result<(), String> {
        tracing::info!("Sending Zigbee message: {}", message.id);
        Ok(())
    }

    fn receive(&self) -> Option<Message> {
        None
    }

    fn disconnect(&self, device_id: &str) -> std::result::Result<(), String> {
        tracing::info!("Disconnecting Zigbee device: {}", device_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_titanium_zigbee_creation() {
        let zigbee = TitaniumZigbee::new(15, 3);
        assert_eq!(zigbee.get_channel(), 15);
    }

    #[test]
    fn test_titanium_zigbee_set_channel() {
        let mut zigbee = TitaniumZigbee::new(15, 3);
        zigbee.set_channel(20);
        assert_eq!(zigbee.get_channel(), 20);
    }
}
