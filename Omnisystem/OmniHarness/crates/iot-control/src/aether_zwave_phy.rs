use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AetherChannel {
    US_908_4 = 908_400,
    US_909_6 = 909_600,
    EU_868_0 = 868_000,
    JP_922_0 = 922_000,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AetherFrame {
    pub sof: u8,
    pub length: u8,
    pub payload: Vec<u8>,
    pub checksum: u8,
}

impl AetherFrame {
    pub fn new(payload: Vec<u8>) -> Self {
        let length = payload.len() as u8;
        let checksum = Self::calculate_checksum(&payload);
        AetherFrame { sof: 0x01, length, payload, checksum }
    }

    pub fn calculate_checksum(payload: &[u8]) -> u8 {
        payload.iter().fold(0u8, |acc, b| acc.wrapping_add(*b))
    }

    pub fn verify_checksum(&self) -> bool {
        Self::calculate_checksum(&self.payload) == self.checksum
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut data = vec![self.sof, self.length];
        data.extend_from_slice(&self.payload);
        data.push(self.checksum);
        data
    }
}

pub struct AetherPhy {
    channels: Vec<AetherChannel>,
    current_channel: AetherChannel,
    tx_power: i8,
    turbo_mode: bool,
    fec_level: u8,
}

impl AetherPhy {
    pub fn new() -> Self {
        AetherPhy {
            channels: vec![AetherChannel::US_908_4, AetherChannel::EU_868_0],
            current_channel: AetherChannel::US_908_4,
            tx_power: 0,
            turbo_mode: true,
            fec_level: 2,
        }
    }

    pub fn transmit(&self, payload: &[u8]) -> std::result::Result<Vec<u8>, String> {
        let frame = AetherFrame::new(payload.to_vec());
        Ok(frame.serialize())
    }

    pub fn switch_channel(&mut self, channel: AetherChannel) {
        self.current_channel = channel;
    }

    pub fn enable_turbo_mode(&mut self, enable: bool) {
        self.turbo_mode = enable;
    }

    pub fn set_fec_level(&mut self, level: u8) {
        self.fec_level = level.min(3);
    }

    pub fn get_bitrate(&self) -> u32 {
        if self.turbo_mode { 256 } else { 100 }
    }

    pub fn set_tx_power(&mut self, power: i8) {
        self.tx_power = power.clamp(-20, 15);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame() {
        let frame = AetherFrame::new(vec![1, 2, 3]);
        assert!(frame.verify_checksum());
    }

    #[test]
    fn test_phy_turbo() {
        let mut phy = AetherPhy::new();
        phy.enable_turbo_mode(true);
        assert_eq!(phy.get_bitrate(), 256);
    }
}
