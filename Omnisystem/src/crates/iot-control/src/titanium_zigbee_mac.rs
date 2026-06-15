use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitaniumMacFrame {
    pub frame_control: u16,
    pub sequence_number: u8,
    pub destination_pan: u16,
    pub destination_address: u64,
    pub source_address: u64,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacFrameType {
    Beacon = 0,
    Data = 1,
    Acknowledgment = 2,
    Command = 3,
}

pub struct TitaniumMac {
    pan_id: u16,
    short_address: u16,
    extended_address: u64,
    sequence_number: u8,
    tx_queue: VecDeque<TitaniumMacFrame>,
    rx_queue: VecDeque<TitaniumMacFrame>,
    ack_enabled: bool,
    csma_enabled: bool,
    max_retries: u8,
    backoff_exponent: u8,
}

impl TitaniumMac {
    pub fn new(pan_id: u16, extended_address: u64) -> Self {
        TitaniumMac {
            pan_id,
            short_address: 0xFFFF,
            extended_address,
            sequence_number: 0,
            tx_queue: VecDeque::new(),
            rx_queue: VecDeque::new(),
            ack_enabled: true,
            csma_enabled: true,
            max_retries: 3,
            backoff_exponent: 3,
        }
    }

    pub fn set_short_address(&mut self, address: u16) {
        self.short_address = address;
    }

    pub fn send_data(&mut self, dest_address: u64, payload: Vec<u8>) -> std::result::Result<(), String> {
        let frame = TitaniumMacFrame {
            frame_control: self.create_frame_control(MacFrameType::Data),
            sequence_number: self.get_next_sequence(),
            destination_pan: self.pan_id,
            destination_address: dest_address,
            source_address: self.extended_address,
            payload,
        };

        self.tx_queue.push_back(frame);
        Ok(())
    }

    pub fn receive_data(&mut self) -> Option<(u64, Vec<u8>)> {
        self.rx_queue.pop_front().map(|frame| {
            (frame.source_address, frame.payload)
        })
    }

    pub fn process_frame(&mut self, frame: TitaniumMacFrame) {
        if frame.destination_pan == self.pan_id
            && (frame.destination_address == self.extended_address
                || frame.destination_address == 0xFFFFFFFFFFFFFFFF)
        {
            self.rx_queue.push_back(frame);
        }
    }

    pub fn get_next_tx_frame(&mut self) -> Option<TitaniumMacFrame> {
        self.tx_queue.pop_front()
    }

    pub fn enable_ack(&mut self, enable: bool) {
        self.ack_enabled = enable;
    }

    pub fn enable_csma(&mut self, enable: bool) {
        self.csma_enabled = enable;
    }

    pub fn set_max_retries(&mut self, retries: u8) {
        self.max_retries = retries;
    }

    pub fn set_backoff_exponent(&mut self, exponent: u8) {
        self.backoff_exponent = exponent.clamp(0, 5);
    }

    pub fn get_pan_id(&self) -> u16 {
        self.pan_id
    }

    pub fn get_short_address(&self) -> u16 {
        self.short_address
    }

    fn get_next_sequence(&mut self) -> u8 {
        let seq = self.sequence_number;
        self.sequence_number = self.sequence_number.wrapping_add(1);
        seq
    }

    fn create_frame_control(&self, frame_type: MacFrameType) -> u16 {
        let mut fc = (frame_type as u16) & 0x07;
        if self.ack_enabled {
            fc |= 0x20;
        }
        fc |= 0x800;
        fc
    }

    pub fn tx_queue_len(&self) -> usize {
        self.tx_queue.len()
    }

    pub fn rx_queue_len(&self) -> usize {
        self.rx_queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mac_creation() {
        let mac = TitaniumMac::new(0x1234, 0x123456789ABCDEF0);
        assert_eq!(mac.get_pan_id(), 0x1234);
    }

    #[test]
    fn test_send_data() {
        let mut mac = TitaniumMac::new(0x1234, 0x123456789ABCDEF0);
        assert!(mac.send_data(0xFEDCBA9876543210, vec![1, 2, 3]).is_ok());
        assert_eq!(mac.tx_queue_len(), 1);
    }

    #[test]
    fn test_sequence_numbering() {
        let mut mac = TitaniumMac::new(0x1234, 0x123456789ABCDEF0);
        let seq1 = mac.get_next_sequence();
        let seq2 = mac.get_next_sequence();
        assert_eq!(seq1 + 1, seq2);
    }

    #[test]
    fn test_frame_reception() {
        let mut mac = TitaniumMac::new(0x1234, 0x123456789ABCDEF0);
        let frame = TitaniumMacFrame {
            frame_control: 0x8801,
            sequence_number: 0,
            destination_pan: 0x1234,
            destination_address: 0x123456789ABCDEF0,
            source_address: 0xFEDCBA9876543210,
            payload: vec![1, 2, 3],
        };

        mac.process_frame(frame);
        assert_eq!(mac.rx_queue_len(), 1);

        let received = mac.receive_data();
        assert!(received.is_some());
        let (src, payload) = received.unwrap();
        assert_eq!(src, 0xFEDCBA9876543210);
        assert_eq!(payload, vec![1, 2, 3]);
    }

    #[test]
    fn test_ack_control() {
        let mut mac = TitaniumMac::new(0x1234, 0x123456789ABCDEF0);
        mac.enable_ack(true);
        assert!(mac.ack_enabled);
        mac.enable_ack(false);
        assert!(!mac.ack_enabled);
    }

    #[test]
    fn test_short_address() {
        let mut mac = TitaniumMac::new(0x1234, 0x123456789ABCDEF0);
        mac.set_short_address(0x0001);
        assert_eq!(mac.get_short_address(), 0x0001);
    }

    #[test]
    fn test_csma_control() {
        let mut mac = TitaniumMac::new(0x1234, 0x123456789ABCDEF0);
        mac.enable_csma(false);
        assert!(!mac.csma_enabled);
    }

    #[test]
    fn test_broadcast_frame() {
        let mut mac = TitaniumMac::new(0x1234, 0x123456789ABCDEF0);
        let frame = TitaniumMacFrame {
            frame_control: 0x8801,
            sequence_number: 0,
            destination_pan: 0x1234,
            destination_address: 0xFFFFFFFFFFFFFFFF,
            source_address: 0xFEDCBA9876543210,
            payload: vec![1, 2, 3],
        };

        mac.process_frame(frame);
        assert_eq!(mac.rx_queue_len(), 1);
    }
}
