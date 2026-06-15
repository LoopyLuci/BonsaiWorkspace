use crate::{Protocol, Result, IotError, Message, Device, DeviceState};
use dashmap::DashMap;
use std::sync::Arc;

pub struct TitaniumZigbee {
    channel: u8,
    pan_id: u16,
    extended_pan_id: u64,
    nodes: Arc<DashMap<String, ZigbeeNode>>,
    routing_table: Arc<DashMap<u16, Vec<u16>>>,
}

#[derive(Debug, Clone)]
pub struct ZigbeeNode {
    pub short_address: u16,
    pub ieee_address: u64,
    pub lqi: u8,
    pub join_time: u64,
}

impl TitaniumZigbee {
    pub fn new(channel: u8, pan_id: u16, extended_pan_id: u64) -> Self {
        Self {
            channel,
            pan_id,
            extended_pan_id,
            nodes: Arc::new(DashMap::new()),
            routing_table: Arc::new(DashMap::new()),
        }
    }

    pub fn join_device(&self, device_id: String, short_addr: u16, ieee_addr: u64, lqi: u8) -> Result<()> {
        let node = ZigbeeNode {
            short_address: short_addr,
            ieee_address: ieee_addr,
            lqi,
            join_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        self.nodes.insert(device_id, node);
        tracing::info!("Zigbee device joined: addr={:04x}", short_addr);
        Ok(())
    }

    pub fn get_node(&self, device_id: &str) -> Result<ZigbeeNode> {
        self.nodes
            .get(device_id)
            .map(|ref_| ref_.value().clone())
            .ok_or_else(|| IotError::DeviceNotFound(device_id.to_string()))
    }

    pub fn send_message(&self, target: u16, payload: &[u8]) -> Result<()> {
        if payload.len() > 127 {
            return Err(IotError::SendFailed("Payload exceeds 127 bytes".to_string()));
        }
        tracing::info!("Sending Zigbee message to {:04x}, {} bytes", target, payload.len());
        Ok(())
    }

    pub fn get_routing_path(&self, destination: u16) -> Option<Vec<u16>> {
        self.routing_table
            .get(&destination)
            .map(|ref_| ref_.value().clone())
    }

    pub fn update_routing_table(&self, dest: u16, path: Vec<u16>) {
        self.routing_table.insert(dest, path);
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_titanium_zigbee_creation() {
        let zb = TitaniumZigbee::new(15, 0x1234, 0x123456789abcdef0);
        assert_eq!(zb.channel, 15);
    }

    #[test]
    fn test_join_device() {
        let zb = TitaniumZigbee::new(15, 0x1234, 0x123456789abcdef0);
        assert!(zb.join_device("dev1".to_string(), 0x0001, 0x0000000000000001, 200).is_ok());
        assert_eq!(zb.node_count(), 1);
    }

    #[test]
    fn test_send_message() {
        let zb = TitaniumZigbee::new(15, 0x1234, 0x123456789abcdef0);
        let payload = vec![1, 2, 3, 4, 5];
        assert!(zb.send_message(0x0001, &payload).is_ok());
    }

    #[test]
    fn test_oversized_payload() {
        let zb = TitaniumZigbee::new(15, 0x1234, 0x123456789abcdef0);
        let payload = vec![0; 200];
        assert!(zb.send_message(0x0001, &payload).is_err());
    }
}
