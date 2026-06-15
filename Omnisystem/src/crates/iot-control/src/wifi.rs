use crate::{ProtocolHandler, Message, Device};

pub struct WiFiProtocol {
    ssid: String,
    bandwidth: u32,
}

impl WiFiProtocol {
    pub fn new(ssid: String, bandwidth: u32) -> Self {
        Self { ssid, bandwidth }
    }

    pub fn get_ssid(&self) -> &str {
        &self.ssid
    }

    pub fn get_bandwidth(&self) -> u32 {
        self.bandwidth
    }
}

impl ProtocolHandler for WiFiProtocol {
    fn connect(&self, device: &Device) -> std::result::Result<(), String> {
        tracing::info!("Connecting WiFi device: {} to {}", device.id, self.ssid);
        Ok(())
    }

    fn send(&self, message: &Message) -> std::result::Result<(), String> {
        tracing::info!("Sending WiFi message: {}", message.id);
        Ok(())
    }

    fn receive(&self) -> Option<Message> {
        None
    }

    fn disconnect(&self, device_id: &str) -> std::result::Result<(), String> {
        tracing::info!("Disconnecting WiFi device: {}", device_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wifi_protocol_creation() {
        let wifi = WiFiProtocol::new("MyNetwork".to_string(), 80_000_000);
        assert_eq!(wifi.get_ssid(), "MyNetwork");
    }
}
