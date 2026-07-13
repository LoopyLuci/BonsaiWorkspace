use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub label: String,
    pub online: bool,
}

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
