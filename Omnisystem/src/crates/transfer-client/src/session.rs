use std::collections::HashMap;

/// Represents an established connection to a remote peer.
#[derive(Debug, Clone)]
pub struct PeerSession {
    /// The peer identifier (e.g., "alice-phone")
    pub peer_id: String,

    /// Whether the session is active
    pub active: bool,

    /// Transport hints (e.g., relay address, latency)
    pub transport_hints: HashMap<String, String>,

    /// Connected timestamp (Unix millis)
    pub connected_at: u64,
}

impl PeerSession {
    pub fn new(peer_id: &str) -> Self {
        Self {
            peer_id: peer_id.to_string(),
            active: true,
            transport_hints: HashMap::new(),
            connected_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_is_active_with_a_recent_timestamp() {
        let before = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let session = PeerSession::new("alice-phone");

        assert_eq!(session.peer_id, "alice-phone");
        assert!(session.is_active());
        assert!(session.transport_hints.is_empty());
        assert!(session.connected_at >= before);
    }
}
