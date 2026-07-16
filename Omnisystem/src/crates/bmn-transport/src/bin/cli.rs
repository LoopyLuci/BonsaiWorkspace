//! CLI that exercises multi-path bonding and the MoQ/WebRTC transports.

use bmn_common::transport::Transport;
use bmn_transport::{MoqTransport, MultiPathBonding, NetworkPath, PathHealth, WebRTCTransport};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bonding = MultiPathBonding::new();
    bonding.add_path(PathHealth {
        path: NetworkPath::Ethernet,
        latency_ms: 5.0,
        jitter_ms: 1.0,
        loss_percent: 0.0,
        bandwidth_mbps: 1000.0,
        is_healthy: true,
    });
    bonding.add_path(PathHealth {
        path: NetworkPath::WiFi,
        latency_ms: 50.0,
        jitter_ms: 10.0,
        loss_percent: 0.5,
        bandwidth_mbps: 100.0,
        is_healthy: true,
    });
    println!("Best path: {:?}", bonding.select_best_path());
    println!("Failover order: {:?}", bonding.failover_paths());

    let mut moq = MoqTransport::new();
    moq.connect("moq://stream.example.com").await?;
    moq.send(b"hello moq").await?;
    println!("MoQ stats: {:?}", moq.stats());

    let mut webrtc = WebRTCTransport::new();
    webrtc.connect("sdp-offer").await?;
    webrtc.send(b"hello webrtc").await?;
    println!("WebRTC stats: {:?}", webrtc.stats());

    Ok(())
}
