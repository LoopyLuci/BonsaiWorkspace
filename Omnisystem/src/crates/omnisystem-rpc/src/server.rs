/// RPC Server Module

use tracing::info;

/// RPC Server implementation
pub struct Server;

impl Server {
    pub fn new() -> Self {
        info!("Creating RPC Server");
        Self
    }

    pub async fn start(&self, port: u16) {
        info!("Starting RPC Server on port {}", port);
    }
}
