/// RPC Client Module

use tracing::info;

/// RPC Client implementation
pub struct Client;

impl Client {
    pub fn new(addr: &str) -> Self {
        info!("Creating RPC Client for {}", addr);
        Self
    }

    pub async fn connect(&self) {
        info!("Connecting to RPC server");
    }
}
