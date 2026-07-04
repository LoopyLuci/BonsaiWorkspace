//! Network emulator with loss, latency, jitter, and partition simulation

use crate::errors::EmulationResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Network interface type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkInterface {
    /// Ethernet
    Ethernet,
    /// WiFi 5 (802.11ac)
    WiFi5,
    /// WiFi 6 (802.11ax)
    WiFi6,
    /// Cellular 4G LTE
    Cellular4G,
    /// Cellular 5G
    Cellular5G,
    /// Loopback (local)
    Loopback,
}

impl std::fmt::Display for NetworkInterface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ethernet => write!(f, "Ethernet"),
            Self::WiFi5 => write!(f, "WiFi 5"),
            Self::WiFi6 => write!(f, "WiFi 6"),
            Self::Cellular4G => write!(f, "4G LTE"),
            Self::Cellular5G => write!(f, "5G"),
            Self::Loopback => write!(f, "Loopback"),
        }
    }
}

/// Network loss configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NetworkLoss {
    /// Packet loss percentage (0-100)
    pub packet_loss_percent: f64,
    /// Burst loss probability
    pub burst_loss_prob: f64,
    /// Burst length
    pub burst_length: u32,
}

impl Default for NetworkLoss {
    fn default() -> Self {
        Self {
            packet_loss_percent: 0.0,
            burst_loss_prob: 0.0,
            burst_length: 0,
        }
    }
}

impl NetworkLoss {
    /// Create a profile with no loss
    pub fn none() -> Self {
        Self::default()
    }

    /// Create a profile simulating poor connectivity (3G-like)
    pub fn poor() -> Self {
        Self {
            packet_loss_percent: 2.0,
            burst_loss_prob: 0.1,
            burst_length: 5,
        }
    }

    /// Create a profile simulating moderate loss
    pub fn moderate() -> Self {
        Self {
            packet_loss_percent: 0.5,
            burst_loss_prob: 0.05,
            burst_length: 3,
        }
    }
}

/// Network profile (latency, bandwidth, loss, jitter)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkProfile {
    /// Interface type
    pub interface_type: NetworkInterface,
    /// Bandwidth in Mbps
    pub bandwidth_mbps: u32,
    /// Base latency in milliseconds
    pub latency_ms: f64,
    /// Jitter in milliseconds
    pub jitter_ms: f64,
    /// Packet loss configuration
    pub loss: NetworkLoss,
    /// MTU (maximum transmission unit) in bytes
    pub mtu: u16,
}

impl NetworkProfile {
    /// Create a profile for Ethernet (1 Gbps)
    pub fn ethernet() -> Self {
        Self {
            interface_type: NetworkInterface::Ethernet,
            bandwidth_mbps: 1000,
            latency_ms: 0.5,
            jitter_ms: 0.1,
            loss: NetworkLoss::none(),
            mtu: 1500,
        }
    }

    /// Create a profile for WiFi 6
    pub fn wifi6() -> Self {
        Self {
            interface_type: NetworkInterface::WiFi6,
            bandwidth_mbps: 1200,
            latency_ms: 5.0,
            jitter_ms: 2.0,
            loss: NetworkLoss::poor(),
            mtu: 1500,
        }
    }

    /// Create a profile for 5G cellular
    pub fn cellular_5g() -> Self {
        Self {
            interface_type: NetworkInterface::Cellular5G,
            bandwidth_mbps: 1000,
            latency_ms: 10.0,
            jitter_ms: 5.0,
            loss: NetworkLoss::moderate(),
            mtu: 1500,
        }
    }

    /// Create a profile for 4G LTE cellular
    pub fn cellular_4g() -> Self {
        Self {
            interface_type: NetworkInterface::Cellular4G,
            bandwidth_mbps: 100,
            latency_ms: 50.0,
            jitter_ms: 20.0,
            loss: NetworkLoss::poor(),
            mtu: 1500,
        }
    }

    /// Create a profile for loopback
    pub fn loopback() -> Self {
        Self {
            interface_type: NetworkInterface::Loopback,
            bandwidth_mbps: 100_000, // Effectively unlimited
            latency_ms: 0.01,
            jitter_ms: 0.0,
            loss: NetworkLoss::none(),
            mtu: 65535,
        }
    }
}

/// Network configuration with multiple interfaces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Network interfaces
    pub interfaces: Vec<NetworkProfile>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            interfaces: vec![
                NetworkProfile::ethernet(),
                NetworkProfile::loopback(),
            ],
        }
    }
}

impl NetworkConfig {
    /// Create a minimal configuration
    pub fn minimal() -> Self {
        Self {
            interfaces: vec![NetworkProfile::loopback()],
        }
    }

    /// Create a high-performance configuration
    pub fn high_performance() -> Self {
        Self {
            interfaces: vec![
                NetworkProfile::ethernet(),
                NetworkProfile::ethernet(),
                NetworkProfile::loopback(),
            ],
        }
    }

    /// Create a mobile/wireless configuration
    pub fn mobile() -> Self {
        Self {
            interfaces: vec![
                NetworkProfile::wifi6(),
                NetworkProfile::cellular_5g(),
                NetworkProfile::loopback(),
            ],
        }
    }
}

/// Network packet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Packet {
    /// Source address
    pub src_addr: String,
    /// Destination address
    pub dst_addr: String,
    /// Payload
    pub payload: Vec<u8>,
    /// Sequence number
    pub seq: u64,
    /// Timestamp when sent
    pub timestamp_us: u64,
}

/// Network statistics
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct NetworkStats {
    /// Packets sent
    pub packets_sent: u64,
    /// Packets received
    pub packets_received: u64,
    /// Packets dropped (due to loss)
    pub packets_dropped: u64,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Total latency accumulated (microseconds)
    pub total_latency_us: u64,
    /// Out-of-order packets
    pub out_of_order: u64,
}

impl NetworkStats {
    /// Average latency in milliseconds
    pub fn avg_latency_ms(&self) -> f64 {
        if self.packets_received == 0 {
            return 0.0;
        }
        self.total_latency_us as f64 / 1000.0 / self.packets_received as f64
    }

    /// Packet loss percentage
    pub fn packet_loss_percent(&self) -> f64 {
        let total = self.packets_sent + self.packets_dropped;
        if total == 0 {
            return 0.0;
        }
        (self.packets_dropped as f64 / total as f64) * 100.0
    }

    /// Throughput in Mbps
    pub fn throughput_mbps(&self) -> f64 {
        if self.total_latency_us == 0 {
            return 0.0;
        }
        let total_bits = (self.bytes_sent + self.bytes_received) * 8;
        (total_bits as f64 / 1_000_000.0) / (self.total_latency_us as f64 / 1_000_000.0)
    }
}

/// Network emulator trait
#[async_trait]
pub trait NetworkEmulator: Send + Sync {
    /// Send a packet on an interface
    async fn send_packet(&self, interface: usize, data: &[u8]) -> EmulationResult<()>;

    /// Receive a packet from an interface
    async fn receive_packet(&self, interface: usize) -> EmulationResult<Option<Vec<u8>>>;

    /// Reset network to initial state
    async fn reset(&self) -> EmulationResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_interface_display() {
        assert_eq!(NetworkInterface::Ethernet.to_string(), "Ethernet");
        assert_eq!(NetworkInterface::WiFi5.to_string(), "WiFi 5");
        assert_eq!(NetworkInterface::Cellular5G.to_string(), "5G");
    }

    #[test]
    fn test_network_loss_none() {
        let loss = NetworkLoss::none();
        assert_eq!(loss.packet_loss_percent, 0.0);
    }

    #[test]
    fn test_network_loss_poor() {
        let loss = NetworkLoss::poor();
        assert!(loss.packet_loss_percent > 0.0);
    }

    #[test]
    fn test_network_profile_ethernet() {
        let profile = NetworkProfile::ethernet();
        assert_eq!(profile.interface_type, NetworkInterface::Ethernet);
        assert_eq!(profile.bandwidth_mbps, 1000);
    }

    #[test]
    fn test_network_profile_wifi6() {
        let profile = NetworkProfile::wifi6();
        assert_eq!(profile.interface_type, NetworkInterface::WiFi6);
        assert!(profile.latency_ms > 0.0);
    }

    #[test]
    fn test_network_profile_5g() {
        let profile = NetworkProfile::cellular_5g();
        assert_eq!(profile.interface_type, NetworkInterface::Cellular5G);
    }

    #[test]
    fn test_network_profile_loopback() {
        let profile = NetworkProfile::loopback();
        assert_eq!(profile.interface_type, NetworkInterface::Loopback);
        assert_eq!(profile.latency_ms, 0.01);
    }

    #[test]
    fn test_network_config_default() {
        let config = NetworkConfig::default();
        assert_eq!(config.interfaces.len(), 2);
    }

    #[test]
    fn test_network_config_mobile() {
        let config = NetworkConfig::mobile();
        assert!(config.interfaces.len() >= 2);
    }

    #[test]
    fn test_network_stats_avg_latency() {
        let mut stats = NetworkStats::default();
        stats.packets_received = 100;
        stats.total_latency_us = 100_000; // 100 ms total

        assert_eq!(stats.avg_latency_ms(), 1.0);
    }

    #[test]
    fn test_network_stats_packet_loss() {
        let mut stats = NetworkStats::default();
        stats.packets_sent = 100;
        stats.packets_dropped = 5;
        // Total packets = sent + dropped = 105
        // Loss percentage = 5 / 105 * 100 = ~4.76%
        let loss_percent = stats.packet_loss_percent();
        assert!(loss_percent > 4.0 && loss_percent < 5.0);
    }
}
