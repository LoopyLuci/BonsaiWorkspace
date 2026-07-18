use crate::{Packet, MacAddress, Result, NetworkError};
use dashmap::DashMap;
use std::sync::Arc;

pub struct Layer2Switch {
    mac_table: Arc<DashMap<MacAddress, String>>,
    forwarding_enabled: bool,
}

impl Layer2Switch {
    pub fn new() -> Self {
        Self {
            mac_table: Arc::new(DashMap::new()),
            forwarding_enabled: true,
        }
    }

    pub fn learn_mac(&self, mac: MacAddress, port: String) -> Result<()> {
        self.mac_table.insert(mac, port);
        tracing::info!("MAC learned: {}", mac);
        Ok(())
    }

    pub fn lookup_mac(&self, mac: MacAddress) -> Result<String> {
        self.mac_table
            .get(&mac)
            .map(|ref_| ref_.value().clone())
            .ok_or_else(|| NetworkError::MACLookupFailed(mac.to_string()))
    }

    pub fn forward_frame(&self, packet: &Packet) -> Result<()> {
        if !self.forwarding_enabled {
            return Err(NetworkError::PacketError("Forwarding disabled".to_string()));
        }
        let _port = self.lookup_mac(packet.destination)?;
        tracing::info!("Frame forwarded to {}", packet.destination);
        Ok(())
    }

    pub fn mac_table_size(&self) -> usize {
        self.mac_table.len()
    }
}

impl Default for Layer2Switch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_learn_mac() {
        let switch = Layer2Switch::new();
        let mac = MacAddress::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
        assert!(switch.learn_mac(mac, "eth0".to_string()).is_ok());
        assert_eq!(switch.mac_table_size(), 1);
    }

    #[test]
    fn test_lookup_mac() {
        let switch = Layer2Switch::new();
        let mac = MacAddress::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
        switch.learn_mac(mac, "eth0".to_string()).unwrap();
        assert!(switch.lookup_mac(mac).is_ok());
    }

    #[test]
    fn test_forward_frame() {
        let switch = Layer2Switch::new();
        let mac = MacAddress::new([0xff; 6]);
        switch.learn_mac(mac, "eth1".to_string()).unwrap();
        
        let packet = Packet {
            id: "pkt1".to_string(),
            source: MacAddress::new([0xaa; 6]),
            destination: mac,
            payload: vec![],
            vlan_id: None,
        };
        assert!(switch.forward_frame(&packet).is_ok());
    }
}
