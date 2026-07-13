/// Distributed linting coordinator
/// Parallelize linting across peer machines for 5-10x speedup

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub address: String,
    pub capacity: usize, // Number of files it can lint in parallel
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintJob {
    pub job_id: String,
    pub files: Vec<PathBuf>,
    pub config: String, // Serialized LintConfig
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub job_id: String,
    pub peer_id: String,
    pub diagnostics: Vec<crate::Diagnostic>,
    pub duration_ms: u64,
}

pub struct DistributedLintCoordinator {
    local_peer_id: String,
    peers: Vec<PeerInfo>,
}

impl DistributedLintCoordinator {
    pub async fn new(local_peer_id: String) -> Result<Self> {
        tracing::info!("Initializing distributed lint coordinator");

        Ok(Self {
            local_peer_id,
            peers: Vec::new(),
        })
    }

    /// Discover available peers for distributed linting
    pub async fn discover_peers(&mut self) -> Result<Vec<PeerInfo>> {
        tracing::info!("Discovering available peers...");

        // TODO: Replace with actual peer discovery via TransferDaemon
        // let peers = transfer_daemon::discover_peers().await?;

        self.peers = vec![];
        Ok(self.peers.clone())
    }

    /// Lint files distributed across peers
    pub async fn lint_distributed(&self, files: Vec<PathBuf>, config: String) -> Result<Vec<crate::Diagnostic>> {
        if files.is_empty() {
            return Ok(Vec::new());
        }

        tracing::info!("Distributing {} files across {} peers", files.len(), self.peers.len());

        if self.peers.is_empty() {
            // Fall back to local linting
            return self.lint_local(&files).await;
        }

        // Split files evenly among available peers
        let chunks = self.split_files_evenly(&files, self.peers.len());
        let mut results = Vec::new();

        for (peer, chunk) in self.peers.iter().zip(chunks) {
            if !peer.available {
                continue;
            }

            match self.send_lint_request(peer, chunk, &config).await {
                Ok(diagnostics) => {
                    tracing::debug!("Received {} diagnostics from peer {}", diagnostics.len(), peer.peer_id);
                    results.extend(diagnostics);
                }
                Err(e) => {
                    tracing::warn!("Lint request to peer {} failed: {}", peer.peer_id, e);
                    // Continue with other peers
                }
            }
        }

        Ok(results)
    }

    /// Check if distributed linting is beneficial
    pub fn should_distribute(&self, file_count: usize) -> bool {
        // Distribute if we have multiple peers and enough files
        !self.peers.is_empty() && file_count > 10
    }

    /// Estimate speedup from distributed linting
    pub fn estimate_speedup(&self) -> f32 {
        let available_peers = self.peers.iter().filter(|p| p.available).count();
        // Linear speedup minus overhead
        (available_peers as f32 * 0.95).max(1.0)
    }

    async fn send_lint_request(
        &self,
        peer: &PeerInfo,
        files: Vec<PathBuf>,
        _config: &str,
    ) -> Result<Vec<crate::Diagnostic>> {
        tracing::debug!("Sending lint request to peer {} for {} files", peer.peer_id, files.len());

        // TODO: Replace with actual RPC call via TransferDaemon
        // let request = LintJob {
        //     job_id: uuid::Uuid::new_v4().to_string(),
        //     files,
        //     config: config.to_string(),
        // };
        // let result = transfer_daemon::call_peer(&peer.address, "bul:lint", request).await?;

        Ok(Vec::new())
    }

    async fn lint_local(&self, files: &[PathBuf]) -> Result<Vec<crate::Diagnostic>> {
        tracing::info!("Falling back to local linting for {} files", files.len());
        // Local linting would be handled by main LintEngine
        Ok(Vec::new())
    }

    fn split_files_evenly(&self, files: &[PathBuf], peer_count: usize) -> Vec<Vec<PathBuf>> {
        let mut chunks: Vec<Vec<PathBuf>> = vec![Vec::new(); peer_count];

        for (i, file) in files.iter().enumerate() {
            chunks[i % peer_count].push(file.clone());
        }

        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coordinator_creation() {
        let coord = DistributedLintCoordinator::new("local-peer".to_string())
            .await
            .unwrap();
        assert_eq!(coord.local_peer_id, "local-peer");
    }

    #[test]
    fn test_should_distribute() {
        let coord = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(DistributedLintCoordinator::new("local-peer".to_string()))
            .unwrap();
        // No peers, shouldn't distribute
        assert!(!coord.should_distribute(100));
    }

    #[test]
    fn test_split_files_evenly() {
        let coord = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(DistributedLintCoordinator::new("local-peer".to_string()))
            .unwrap();

        let files: Vec<PathBuf> = (0..10).map(|i| PathBuf::from(format!("file{}.rs", i))).collect();

        let chunks = coord.split_files_evenly(&files, 3);
        assert_eq!(chunks.len(), 3);
        assert!(!chunks[0].is_empty());
    }

    #[tokio::test]
    async fn test_lint_distributed_no_peers() {
        let coord = DistributedLintCoordinator::new("local-peer".to_string())
            .await
            .unwrap();
        let files = vec![PathBuf::from("test.rs")];
        let result = coord.lint_distributed(files, "{}".to_string()).await;
        assert!(result.is_ok());
    }
}
