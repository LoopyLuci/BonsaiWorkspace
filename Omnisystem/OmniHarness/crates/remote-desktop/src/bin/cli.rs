//! Remote-desktop CLI — exercise the in-process session/rendezvous services.
//!
//! Usage:
//!   remote_desktop_cli register <name> <addr>
//!   remote_desktop_cli discover
//!   remote_desktop_cli session <addr>
//!   remote_desktop_cli status

use remote_desktop::rendezvous::{PeerInfo, RendezvousService};
use remote_desktop::session::SessionManager;
use remote_desktop::{PeerId, Result};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let rendezvous = RendezvousService::new();
    let sessions = SessionManager::new();

    match args.get(1).map(String::as_str) {
        Some("register") => {
            let name = args.get(2).cloned().unwrap_or_else(|| "peer".into());
            let addr = args.get(3).cloned().unwrap_or_else(|| "127.0.0.1:5000".into());
            let peer_id = PeerId::from_bytes(&[1u8; 32]);
            let socket_addr = addr.parse().map_err(|_| {
                remote_desktop::Error::Other(format!("invalid address: {addr}"))
            })?;
            let peer_info = PeerInfo::new(peer_id, name.clone()).with_address(socket_addr);
            rendezvous.register_peer(peer_info).await?;
            println!("registered peer {peer_id} ({name}) at {addr}");
        }
        Some("discover") => {
            match rendezvous.discover_peers().await {
                Ok(peers) => {
                    for p in peers {
                        println!("{} — {} ({} addr(s))", p.id, p.name, p.addresses.len());
                    }
                }
                Err(e) => println!("no peers: {e}"),
            }
        }
        Some("session") => {
            let addr = args.get(2).cloned().unwrap_or_else(|| "127.0.0.1:5000".into());
            let peer_id = PeerId::from_bytes(&[2u8; 32]);
            let socket_addr = addr.parse().map_err(|_| {
                remote_desktop::Error::Other(format!("invalid address: {addr}"))
            })?;
            rendezvous
                .register_peer(PeerInfo::new(peer_id, "session-peer".into()).with_address(socket_addr))
                .await?;
            let session_id = remote_desktop::SessionId::new();
            let state = sessions.create_session(session_id, peer_id, None).await?;
            println!("created session {} for peer {}", state.session_id, state.remote_peer);
        }
        Some("status") => {
            let state = sessions.health();
            println!("[{}] {}", state.timestamp, state.status);
        }
        _ => {
            eprintln!("usage: remote_desktop_cli <register <name> <addr>|discover|session <addr>|status>");
            std::process::exit(1);
        }
    }

    Ok(())
}
