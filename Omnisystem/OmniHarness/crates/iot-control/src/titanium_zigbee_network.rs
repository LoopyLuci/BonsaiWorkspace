use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPacket {
    pub frame_type: u8,
    pub source: u64,
    pub destination: u64,
    pub next_hop: Option<u64>,
    pub hop_count: u8,
    pub sequence: u8,
    pub payload: Vec<u8>,
}

#[derive(Clone)]
pub struct RoutingTable {
    routes: HashMap<u64, RoutingEntry>,
}

#[derive(Clone, Debug)]
pub struct RoutingEntry {
    pub next_hop: u64,
    pub hop_count: u8,
    pub path_cost: u16,
    pub last_update: u64,
    pub stable: bool,
}

pub struct TitaniumNetwork {
    address: u64,
    parent: Option<u64>,
    children: Vec<u64>,
    routing_table: RoutingTable,
    tx_queue: VecDeque<NetworkPacket>,
    rx_queue: VecDeque<NetworkPacket>,
    sequence_number: u8,
    network_address: u16,
}

impl RoutingTable {
    pub fn new() -> Self {
        RoutingTable {
            routes: HashMap::new(),
        }
    }

    pub fn add_route(&mut self, dest: u64, entry: RoutingEntry) {
        self.routes.insert(dest, entry);
    }

    pub fn get_route(&self, dest: u64) -> Option<&RoutingEntry> {
        self.routes.get(&dest)
    }

    pub fn remove_route(&mut self, dest: u64) {
        self.routes.remove(&dest);
    }

    pub fn get_next_hop(&self, dest: u64) -> Option<u64> {
        self.get_route(dest).map(|entry| entry.next_hop)
    }

    pub fn update_route(&mut self, dest: u64, new_cost: u16) {
        if let Some(entry) = self.routes.get_mut(&dest) {
            entry.path_cost = new_cost;
        }
    }
}

impl TitaniumNetwork {
    pub fn new(address: u64, network_address: u16) -> Self {
        TitaniumNetwork {
            address,
            parent: None,
            children: Vec::new(),
            routing_table: RoutingTable::new(),
            tx_queue: VecDeque::new(),
            rx_queue: VecDeque::new(),
            sequence_number: 0,
            network_address,
        }
    }

    pub fn set_parent(&mut self, parent: u64) {
        self.parent = Some(parent);
    }

    pub fn add_child(&mut self, child: u64) {
        if !self.children.contains(&child) {
            self.children.push(child);
        }
    }

    pub fn remove_child(&mut self, child: u64) {
        self.children.retain(|&c| c != child);
    }

    pub fn send_packet(&mut self, dest: u64, payload: Vec<u8>) -> std::result::Result<(), String> {
        let packet = NetworkPacket {
            frame_type: 0x01,
            source: self.address,
            destination: dest,
            next_hop: self.routing_table.get_next_hop(dest),
            hop_count: 0,
            sequence: self.get_next_sequence(),
            payload,
        };

        self.tx_queue.push_back(packet);
        Ok(())
    }

    pub fn receive_packet(&mut self) -> Option<(u64, Vec<u8>)> {
        self.rx_queue.pop_front().map(|packet| {
            (packet.source, packet.payload)
        })
    }

    pub fn process_packet(&mut self, mut packet: NetworkPacket) {
        if packet.destination == self.address {
            self.rx_queue.push_back(packet);
        } else if packet.hop_count < 15 {
            packet.hop_count += 1;
            if let Some(next_hop) = self.routing_table.get_next_hop(packet.destination) {
                packet.next_hop = Some(next_hop);
                self.tx_queue.push_back(packet);
            }
        }
    }

    pub fn add_route(&mut self, dest: u64, next_hop: u64, hop_count: u8, cost: u16) {
        let entry = RoutingEntry {
            next_hop,
            hop_count,
            path_cost: cost,
            last_update: 0,
            stable: true,
        };
        self.routing_table.add_route(dest, entry);
    }

    pub fn broadcast_packet(&mut self, payload: Vec<u8>) -> std::result::Result<(), String> {
        self.send_packet(0xFFFFFFFFFFFFFFFF, payload)
    }

    pub fn get_address(&self) -> u64 {
        self.address
    }

    pub fn get_network_address(&self) -> u16 {
        self.network_address
    }

    pub fn get_parent(&self) -> Option<u64> {
        self.parent
    }

    pub fn get_children(&self) -> &[u64] {
        &self.children
    }

    fn get_next_sequence(&mut self) -> u8 {
        let seq = self.sequence_number;
        self.sequence_number = self.sequence_number.wrapping_add(1);
        seq
    }

    pub fn tx_queue_len(&self) -> usize {
        self.tx_queue.len()
    }

    pub fn rx_queue_len(&self) -> usize {
        self.rx_queue.len()
    }

    pub fn get_next_tx_packet(&mut self) -> Option<NetworkPacket> {
        self.tx_queue.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_creation() {
        let net = TitaniumNetwork::new(0x123456789ABCDEF0, 0x0001);
        assert_eq!(net.get_address(), 0x123456789ABCDEF0);
        assert_eq!(net.get_network_address(), 0x0001);
    }

    #[test]
    fn test_parent_child_relationships() {
        let mut net = TitaniumNetwork::new(0x0001, 0x0001);
        net.set_parent(0x0000);
        assert_eq!(net.get_parent(), Some(0x0000));

        net.add_child(0x0002);
        net.add_child(0x0003);
        assert_eq!(net.get_children().len(), 2);
    }

    #[test]
    fn test_send_packet() {
        let mut net = TitaniumNetwork::new(0x0001, 0x0001);
        assert!(net.send_packet(0x0002, vec![1, 2, 3]).is_ok());
        assert_eq!(net.tx_queue_len(), 1);
    }

    #[test]
    fn test_routing_table() {
        let mut rt = RoutingTable::new();
        rt.add_route(
            0x0002,
            RoutingEntry {
                next_hop: 0x0001,
                hop_count: 1,
                path_cost: 100,
                last_update: 0,
                stable: true,
            },
        );

        assert_eq!(rt.get_next_hop(0x0002), Some(0x0001));
    }

    #[test]
    fn test_add_route() {
        let mut net = TitaniumNetwork::new(0x0001, 0x0001);
        net.add_route(0x0003, 0x0002, 2, 200);
        assert_eq!(net.routing_table.get_next_hop(0x0003), Some(0x0002));
    }

    #[test]
    fn test_broadcast() {
        let mut net = TitaniumNetwork::new(0x0001, 0x0001);
        assert!(net.broadcast_packet(vec![1, 2, 3]).is_ok());
        assert_eq!(net.tx_queue_len(), 1);
    }

    #[test]
    fn test_packet_processing() {
        let mut net = TitaniumNetwork::new(0x0001, 0x0001);
        let packet = NetworkPacket {
            frame_type: 0x01,
            source: 0x0002,
            destination: 0x0001,
            next_hop: None,
            hop_count: 1,
            sequence: 0,
            payload: vec![1, 2, 3],
        };

        net.process_packet(packet);
        assert_eq!(net.rx_queue_len(), 1);
    }

    #[test]
    fn test_sequence_numbering() {
        let mut net = TitaniumNetwork::new(0x0001, 0x0001);
        net.send_packet(0x0002, vec![1]).unwrap();
        net.send_packet(0x0002, vec![2]).unwrap();

        let pkt1 = net.get_next_tx_packet();
        let pkt2 = net.get_next_tx_packet();

        assert!(pkt1.is_some());
        assert!(pkt2.is_some());
        assert_ne!(pkt1.unwrap().sequence, pkt2.unwrap().sequence);
    }
}
