//! High-level client for connecting to peers and opening bidirectional streams.

use crate::error::TransferClientError;
use crate::session::PeerSession;
use crate::stream::PeerStream;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Configuration for the TransferDaemon client.
#[derive(Debug, Clone)]
pub struct TransferClientConfig {
    /// Relay server address (e.g., "127.0.0.1:9800")
    pub relay_addr: Option<String>,
    /// Relay authentication token
    pub relay_token: Option<String>,
    /// Fallback HTTP bridge base URL
    pub fallback_url: Option<String>,
    /// Stream timeout in milliseconds
    pub stream_timeout_ms: u64,
}

impl Default for TransferClientConfig {
    fn default() -> Self {
        Self {
            relay_addr: std::env::var("BONSAI_TRANSFER_RELAY_ADDR").ok(),
            relay_token: std::env::var("BONSAI_TRANSFER_RELAY_TOKEN").ok(),
            fallback_url: std::env::var("BONSAI_TRANSFER_BRIDGE_FALLBACK").ok(),
            stream_timeout_ms: std::env::var("BONSAI_TRANSFER_STREAM_TIMEOUT_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30_000),
        }
    }
}

/// High‑level client for connecting to peers and opening streams
/// via the TransferDaemon transport layer.
pub struct TransferDaemonClient {
    config: TransferClientConfig,
    active_sessions: Arc<Mutex<Vec<PeerSession>>>,
}

impl TransferDaemonClient {
    /// Create a new client with the given configuration.
    pub fn new(config: TransferClientConfig) -> Self {
        Self {
            config,
            active_sessions: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        Self::new(TransferClientConfig::default())
    }

    /// Connect to a remote peer and establish a session.
    pub async fn connect(&self, peer_id: &str) -> Result<PeerSession, TransferClientError> {
        // Check if we already have an active session
        let sessions = self.active_sessions.lock().await;
        if let Some(existing) = sessions.iter().find(|s| s.peer_id == peer_id && s.is_active()) {
            return Ok(existing.clone());
        }
        drop(sessions);

        // Determine which transport is available
        if self.config.relay_addr.is_some() && self.config.relay_token.is_some() {
            self.connect_via_relay(peer_id).await
        } else if self.config.fallback_url.is_some() {
            self.connect_via_http(peer_id).await
        } else {
            Err(TransferClientError::TransportUnavailable(
                "no relay or fallback URL configured".into(),
            ))
        }
    }

    /// Open a named stream to a connected peer.
    pub async fn open_stream(
        &self,
        _session: &PeerSession,
        stream_name: &str,
    ) -> Result<PeerStream, TransferClientError> {
        if self.config.relay_addr.is_some() && self.config.relay_token.is_some() {
            self.open_stream_via_relay(_session, stream_name).await
        } else if self.config.fallback_url.is_some() {
            Ok(self.open_stream_via_http(_session, stream_name))
        } else {
            Err(TransferClientError::TransportUnavailable(
                "no relay or fallback URL configured".into(),
            ))
        }
    }

    async fn connect_via_relay(
        &self,
        peer_id: &str,
    ) -> Result<PeerSession, TransferClientError> {
        let _relay_addr = self
            .config
            .relay_addr
            .as_ref()
            .ok_or(TransferClientError::ConfigError("relay_addr not set".into()))?;
        let _relay_token = self
            .config
            .relay_token
            .as_ref()
            .ok_or(TransferClientError::ConfigError("relay_token not set".into()))?;

        // TODO: When peer-side relay consumer is deployed, establish connection
        // For now, relay transport is not yet connected to peer-side consumer
        let session = PeerSession::new(peer_id);
        {
            let mut sessions = self.active_sessions.lock().await;
            sessions.push(session.clone());
        }
        Ok(session)
    }

    async fn connect_via_http(
        &self,
        peer_id: &str,
    ) -> Result<PeerSession, TransferClientError> {
        let fallback = self
            .config
            .fallback_url
            .as_ref()
            .ok_or(TransferClientError::ConfigError(
                "fallback_url not set".into(),
            ))?;

        let url = format!("{}/peers/{}/connect", fallback, peer_id);
        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .send()
            .await
            .map_err(|e| TransferClientError::ConnectionFailed {
                peer: peer_id.to_string(),
                reason: e.to_string(),
            })?;

        if !resp.status().is_success() {
            return Err(TransferClientError::ConnectionFailed {
                peer: peer_id.to_string(),
                reason: format!("HTTP {}", resp.status()),
            });
        }

        let session = PeerSession::new(peer_id);
        {
            let mut sessions = self.active_sessions.lock().await;
            sessions.push(session.clone());
        }
        Ok(session)
    }

    #[allow(dead_code)]
    async fn open_stream_via_relay(
        &self,
        _session: &PeerSession,
        _stream_name: &str,
    ) -> Result<PeerStream, TransferClientError> {
        // In production: use bonsai-relay raw frame sender/receiver
        Err(TransferClientError::TransportUnavailable(
            "relay stream API not yet connected to peer-side consumer".into(),
        ))
    }

    fn open_stream_via_http(
        &self,
        session: &PeerSession,
        stream_name: &str,
    ) -> PeerStream {
        let fallback = self
            .config
            .fallback_url
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:11429".into());
        PeerStream::new_http_fallback(stream_name, &session.peer_id, &fallback)
    }

    /// List all active sessions.
    pub async fn list_sessions(&self) -> Vec<PeerSession> {
        self.active_sessions.lock().await.clone()
    }

    /// Disconnect from a peer.
    pub async fn disconnect(&self, peer_id: &str) -> Result<(), TransferClientError> {
        let mut sessions = self.active_sessions.lock().await;
        sessions.retain(|s| s.peer_id != peer_id);
        Ok(())
    }
}
