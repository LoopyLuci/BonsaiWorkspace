use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPCMessage {
    pub id: String,
    pub command: String,
    pub payload: String,
}

pub struct IPCServer {
    listening: bool,
}

impl IPCServer {
    pub fn new() -> Self {
        Self { listening: false }
    }

    pub async fn start_listening(&mut self) -> anyhow::Result<()> {
        self.listening = true;
        Ok(())
    }

    pub fn is_listening(&self) -> bool {
        self.listening
    }
}

impl Default for IPCServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ipc_server() {
        let mut server = IPCServer::new();
        assert!(!server.is_listening());
        server.start_listening().await.unwrap();
        assert!(server.is_listening());
    }
}
