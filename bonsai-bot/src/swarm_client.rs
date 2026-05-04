use std::sync::Arc;
use std::time::Duration;
use reqwest::Client;
use serde_json::{json, Value};

use crate::config::SwarmPeer;
use crate::platforms::InboundMessage;

// ── Swarm client ──────────────────────────────────────────────────────────────

pub struct SwarmClient {
    http:  Client,
    peers: Vec<SwarmPeer>,
}

impl SwarmClient {
    pub fn new(peers: Vec<SwarmPeer>) -> Arc<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("swarm reqwest client");
        Arc::new(Self { http, peers })
    }

    /// Returns true if the client has any configured peers.
    pub fn has_peers(&self) -> bool { !self.peers.is_empty() }

    /// Check a single peer's /health endpoint.
    pub async fn peer_healthy(&self, peer: &SwarmPeer) -> bool {
        let url = format!("{}/health", peer.admin_url);
        self.http.get(&url)
            .header("authorization", format!("Bearer {}", peer.token))
            .send().await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Find the best peer to handle `msg` based on keyword routing rules.
    /// Returns `None` if no live peer matches.
    pub async fn route(&self, msg: &InboundMessage) -> Option<&SwarmPeer> {
        let text_lower = msg.text.to_lowercase();

        // Keyword-matched peers first
        for peer in &self.peers {
            if peer.route_keywords.iter().any(|kw| text_lower.contains(&kw.to_lowercase())) {
                if self.peer_healthy(peer).await {
                    return Some(peer);
                }
                tracing::warn!("[swarm] peer '{}' matched keywords but is unhealthy", peer.name);
            }
        }
        None
    }

    /// Forward an inbound message to a specific peer's /broadcast endpoint.
    /// Returns the peer's response body, or an error string.
    pub async fn forward(&self, peer: &SwarmPeer, msg: &InboundMessage) -> Result<Value, String> {
        let url = format!("{}/broadcast", peer.admin_url);
        let body = json!({
            "message":   &msg.text,
            "platforms": [&msg.platform],
            "_forwarded_from": {
                "platform":    &msg.platform,
                "platform_id": &msg.platform_id,
                "user_id":     &msg.user_id,
            }
        });

        self.http.post(&url)
            .header("authorization", format!("Bearer {}", peer.token))
            .json(&body)
            .send().await
            .map_err(|e| e.to_string())?
            .json::<Value>().await
            .map_err(|e| e.to_string())
    }

    /// Health-check all peers and return a summary.
    pub async fn status_all(&self) -> Value {
        let mut results = Vec::new();
        for peer in &self.peers {
            let healthy = self.peer_healthy(peer).await;
            results.push(json!({
                "name":       &peer.name,
                "admin_url":  &peer.admin_url,
                "healthy":    healthy,
                "keywords":   &peer.route_keywords,
            }));
        }
        json!({ "peers": results })
    }
}
