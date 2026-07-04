use std::collections::VecDeque;

pub struct AetherMac {
    node_id: u32,
    tx_queue: VecDeque<Vec<u8>>,
    rx_queue: VecDeque<Vec<u8>>,
    multi_path_enabled: bool,
    max_retries: u8,
}

impl AetherMac {
    pub fn new(node_id: u32) -> Self {
        AetherMac {
            node_id,
            tx_queue: VecDeque::new(),
            rx_queue: VecDeque::new(),
            multi_path_enabled: true,
            max_retries: 5,
        }
    }

    pub fn send(&mut self, data: Vec<u8>) {
        self.tx_queue.push_back(data);
    }

    pub fn receive(&mut self) -> Option<Vec<u8>> {
        self.rx_queue.pop_front()
    }

    pub fn process_frame(&mut self, frame: Vec<u8>) {
        self.rx_queue.push_back(frame);
    }

    pub fn enable_multi_path(&mut self, enable: bool) {
        self.multi_path_enabled = enable;
    }

    pub fn set_max_retries(&mut self, retries: u8) {
        self.max_retries = retries;
    }

    pub fn get_next_tx(&mut self) -> Option<Vec<u8>> {
        self.tx_queue.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mac_send_receive() {
        let mut mac = AetherMac::new(1);
        mac.send(vec![1, 2, 3]);
        assert_eq!(mac.tx_queue.len(), 1);
    }

    #[test]
    fn test_multi_path() {
        let mut mac = AetherMac::new(1);
        mac.enable_multi_path(false);
        assert!(!mac.multi_path_enabled);
    }
}
