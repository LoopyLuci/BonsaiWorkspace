use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TitaniumChannel {
    Ch11 = 2405,
    Ch12 = 2410,
    Ch13 = 2415,
    Ch14 = 2420,
    Ch15 = 2425,
    Ch16 = 2430,
    Ch17 = 2435,
    Ch18 = 2440,
    Ch19 = 2445,
    Ch20 = 2450,
    Ch21 = 2455,
    Ch22 = 2460,
    Ch23 = 2465,
    Ch24 = 2470,
    Ch25 = 2475,
    Ch26 = 2480,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitaniumPhyFrame {
    pub preamble: [u8; 4],
    pub sfd: u8,
    pub frame_length: u8,
    pub payload: Vec<u8>,
    pub fcs: u16,
}

impl TitaniumPhyFrame {
    pub fn new(payload: Vec<u8>) -> Self {
        let frame_length = payload.len() as u8 + 3;
        let fcs = Self::calculate_fcs(&payload);

        TitaniumPhyFrame {
            preamble: [0xA7, 0xA7, 0xA7, 0xA7],
            sfd: 0x7E,
            frame_length,
            payload,
            fcs,
        }
    }

    pub fn calculate_fcs(payload: &[u8]) -> u16 {
        let mut crc = 0xFFFFu16;
        for byte in payload {
            crc ^= *byte as u16;
            for _ in 0..8 {
                if (crc & 1) != 0 {
                    crc = (crc >> 1) ^ 0xA001;
                } else {
                    crc >>= 1;
                }
            }
        }
        crc
    }

    pub fn verify_fcs(&self) -> bool {
        Self::calculate_fcs(&self.payload) == self.fcs
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.preamble);
        data.push(self.sfd);
        data.push(self.frame_length);
        data.extend_from_slice(&self.payload);
        data.extend_from_slice(&self.fcs.to_le_bytes());
        data
    }
}

pub struct TitaniumPhy {
    channels: Vec<TitaniumChannel>,
    current_channel: TitaniumChannel,
    transmit_power: i8,
    frame_queue: Vec<TitaniumPhyFrame>,
    adaptive_mode: bool,
    fec_enabled: bool,
    channel_metrics: HashMap<u8, ChannelMetrics>,
}

#[derive(Clone, Debug)]
pub struct ChannelMetrics {
    pub rssi: i8,
    pub lqi: u8,
    pub error_rate: f32,
    pub interference_level: u8,
}

impl TitaniumPhy {
    pub fn new() -> Self {
        let channels = vec![
            TitaniumChannel::Ch15, TitaniumChannel::Ch20, TitaniumChannel::Ch25,
        ];

        TitaniumPhy {
            channels,
            current_channel: TitaniumChannel::Ch15,
            transmit_power: 0,
            frame_queue: Vec::new(),
            adaptive_mode: true,
            fec_enabled: true,
            channel_metrics: HashMap::new(),
        }
    }

    pub fn transmit(&mut self, payload: Vec<u8>) -> std::result::Result<(), String> {
        let frame = TitaniumPhyFrame::new(payload);
        self.frame_queue.push(frame);
        Ok(())
    }

    pub fn receive(&mut self) -> Option<Vec<u8>> {
        if !self.frame_queue.is_empty() {
            self.frame_queue.remove(0).payload.into()
        } else {
            None
        }
    }

    pub fn switch_channel(&mut self, channel: TitaniumChannel) {
        self.current_channel = channel;
    }

    pub fn enable_adaptive_channel_switching(&mut self, enable: bool) {
        self.adaptive_mode = enable;
    }

    pub fn enable_fec(&mut self, enable: bool) {
        self.fec_enabled = enable;
    }

    pub fn update_channel_metrics(&mut self, channel: TitaniumChannel, metrics: ChannelMetrics) {
        self.channel_metrics.insert(channel as u8, metrics);
    }

    pub fn get_best_channel(&self) -> TitaniumChannel {
        self.channels
            .iter()
            .map(|&ch| {
                let metrics = self.channel_metrics.get(&(ch as u8));
                (ch, metrics.map(|m| m.error_rate).unwrap_or(1.0))
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(ch, _)| ch)
            .unwrap_or(TitaniumChannel::Ch15)
    }

    pub fn set_transmit_power(&mut self, power: i8) {
        self.transmit_power = power.clamp(-20, 10);
    }

    pub fn get_transmit_power(&self) -> i8 {
        self.transmit_power
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_creation() {
        let payload = vec![1, 2, 3, 4, 5];
        let frame = TitaniumPhyFrame::new(payload.clone());
        assert_eq!(frame.payload, payload);
        assert_eq!(frame.preamble, [0xA7, 0xA7, 0xA7, 0xA7]);
    }

    #[test]
    fn test_fcs_calculation() {
        let payload = vec![0x10, 0x20, 0x30];
        let fcs = TitaniumPhyFrame::calculate_fcs(&payload);
        assert!(fcs > 0);
    }

    #[test]
    fn test_frame_serialization() {
        let payload = vec![1, 2, 3];
        let payload_len = payload.len();
        let frame = TitaniumPhyFrame::new(payload);
        let serialized = frame.serialize();
        assert!(serialized.len() > payload_len);
    }

    #[test]
    fn test_phy_transmit() {
        let mut phy = TitaniumPhy::new();
        assert!(phy.transmit(vec![1, 2, 3]).is_ok());
        assert_eq!(phy.frame_queue.len(), 1);
    }

    #[test]
    fn test_phy_receive() {
        let mut phy = TitaniumPhy::new();
        phy.transmit(vec![1, 2, 3]).unwrap();
        let received = phy.receive();
        assert!(received.is_some());
    }

    #[test]
    fn test_adaptive_channel_switching() {
        let mut phy = TitaniumPhy::new();
        phy.enable_adaptive_channel_switching(true);
        assert!(phy.adaptive_mode);
    }

    #[test]
    fn test_best_channel_selection() {
        let mut phy = TitaniumPhy::new();
        phy.update_channel_metrics(
            TitaniumChannel::Ch15,
            ChannelMetrics {
                rssi: -60,
                lqi: 200,
                error_rate: 0.01,
                interference_level: 10,
            },
        );
        phy.update_channel_metrics(
            TitaniumChannel::Ch20,
            ChannelMetrics {
                rssi: -40,
                lqi: 240,
                error_rate: 0.001,
                interference_level: 5,
            },
        );

        let best = phy.get_best_channel();
        assert_eq!(best, TitaniumChannel::Ch20);
    }

    #[test]
    fn test_transmit_power_control() {
        let mut phy = TitaniumPhy::new();
        phy.set_transmit_power(10);
        assert_eq!(phy.get_transmit_power(), 10);
        phy.set_transmit_power(-30);
        assert_eq!(phy.get_transmit_power(), -20);
    }
}
