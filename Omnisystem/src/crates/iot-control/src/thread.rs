use crate::{ProtocolHandler, Message, Device};

pub struct ThreadProtocol {
    pan_id: u16,
    channel: u8,
}

impl ThreadProtocol {
    pub fn new(pan_id: u16, channel: u8) -> Self {
        Self { pan_id, channel }
    }

    pub fn get_pan_id(&self) -> u16 {
        self.pan_id
    }
}

impl ProtocolHandler for ThreadProtocol {
    fn connect(&self, device: &Device) -> std::result::Result<(), String> {
        tracing::info!("Connecting Thread device: {}", device.id);
        Ok(())
    }

    fn send(&self, message: &Message) -> std::result::Result<(), String> {
        tracing::info!("Sending Thread message: {}", message.id);
        Ok(())
    }

    fn receive(&self) -> Option<Message> {
        None
    }

    fn disconnect(&self, device_id: &str) -> std::result::Result<(), String> {
        tracing::info!("Disconnecting Thread device: {}", device_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_protocol_creation() {
        let thread = ThreadProtocol::new(0x1234, 15);
        assert_eq!(thread.get_pan_id(), 0x1234);
    }
}
