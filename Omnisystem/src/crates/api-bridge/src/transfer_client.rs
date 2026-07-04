//! Bridge adapter for the native TransferDaemon client.

use transfer_client::{TransferDaemonClient, PeerSession, PeerStream};
use std::sync::Arc;

/// Thin wrapper around the native `bonsai-transfer-client` crate.
pub struct TransferClientWrapper {
    inner: Arc<TransferDaemonClient>,
}

impl TransferClientWrapper {
    /// Create a new wrapper with a pre‑configured client.
    pub fn new(client: Arc<TransferDaemonClient>) -> Self {
        Self { inner: client }
    }

    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        Self {
            inner: Arc::new(TransferDaemonClient::from_env()),
        }
    }

    /// Connect to a peer.
    pub async fn connect(&self, peer_id: &str) -> Result<PeerSession, anyhow::Error> {
        self.inner
            .connect(peer_id)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Open a named stream on a connected peer.
    pub async fn open_stream(
        &self,
        session: &PeerSession,
        stream_name: &str,
    ) -> Result<PeerStream, anyhow::Error> {
        self.inner
            .open_stream(session, stream_name)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }
}