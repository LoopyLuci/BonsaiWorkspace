//! Language Server Protocol support for all languages
//!
//! This module provides a unified LSP server that can serve any registered language.

/// Unified LSP server for the Bonsai Ecosystem
///
/// Supports all registered languages through a common protocol.
pub struct BonsaiLspServer;

impl BonsaiLspServer {
    pub fn new() -> Self {
        Self
    }

    /// Start the LSP server on stdio
    pub async fn run(&self) -> anyhow::Result<()> {
        tracing::info!("Bonsai LSP server starting on stdio");
        Ok(())
    }
}

impl Default for BonsaiLspServer {
    fn default() -> Self {
        Self::new()
    }
}
