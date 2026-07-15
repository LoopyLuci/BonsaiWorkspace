use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub label: String,
    pub online: bool,
}

/// Peer discovery, currently backed by a static in-memory list (see
/// `routing::route_to_service`'s "discovery" -> `memory://discovery`
/// mapping). Production path: query the real Echo/mesh peer registry.
pub async fn list_peers() -> Vec<PeerInfo> {
    vec![
        PeerInfo {
            peer_id: "peer-localhost".to_string(),
            label: "Local Device".to_string(),
            online: true,
        },
        PeerInfo {
            peer_id: "peer-mobile".to_string(),
            label: "Phone".to_string(),
            online: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_peers_returns_nonempty() {
        let peers = list_peers().await;
        assert!(!peers.is_empty());
        assert!(peers.iter().all(|p| !p.peer_id.is_empty()));
    }
}
