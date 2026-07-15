//! p2p CLI
//!
//! Demonstrates constructing the transport lanes without requiring a live
//! peer, a running Tor daemon, or STUN/TURN reachability:
//! - `SwarmLane::connect` spins up a libp2p swarm and dials the given
//!   multiaddr in the background (construction succeeds even if the peer is
//!   unreachable).
//! - `OnionLane::connect_default` probes the local Tor SOCKS5 proxy but
//!   still returns a usable (marked-unavailable) lane if the proxy isn't
//!   running.

use p2p::{OnionLane, SwarmLane};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("swarm") => {
            let addr = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| "/ip4/127.0.0.1/tcp/7001".to_string());
            let lane = SwarmLane::connect("swarm:demo", &addr).await?;
            println!("constructed SwarmLane targeting {addr} (health available={})", {
                use p2p_core::lane::TransportLane;
                lane.health().available
            });
        }
        Some("onion") => {
            let target = args.get(2).cloned().unwrap_or_else(|| "example.com".to_string());
            let lane = OnionLane::connect_default("onion:demo", target.clone(), 80).await?;
            println!("constructed OnionLane targeting {target}:80 (health available={})", {
                use p2p_core::lane::TransportLane;
                lane.health().available
            });
        }
        _ => {
            println!("p2p CLI");
            println!("usage:");
            println!("  cli swarm [multiaddr]   construct a SwarmLane and dial (default: /ip4/127.0.0.1/tcp/7001)");
            println!("  cli onion [target-host] construct an OnionLane via the default Tor SOCKS5 proxy (default: example.com:80)");
        }
    }

    Ok(())
}
