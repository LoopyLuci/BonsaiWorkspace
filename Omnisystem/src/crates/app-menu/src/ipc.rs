use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

use crate::client::{
    LauncherClient, AppMetadata, AppInstance, LaunchRequest, LaunchResponse, SystemStatus,
};

/// IPC message types for launcher daemon communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IPCMessage {
    ListApps,
    GetApp { app_id: String },
    SearchApps { query: String },
    LaunchApp { request: LaunchRequest },
    ListInstances,
    TerminateApp { instance_id: Uuid },
    GetStatus,
}

/// IPC response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IPCResponse {
    AppsList(Vec<AppMetadata>),
    AppMetadata(Option<AppMetadata>),
    SearchResults(Vec<AppMetadata>),
    LaunchResult(LaunchResponse),
    InstancesList(Vec<AppInstance>),
    TerminateResult(bool),
    Status(SystemStatus),
    Error(String),
}

/// Real IPC client that connects to launcher daemon
pub struct IPCClient {
    addr: SocketAddr,
}

impl IPCClient {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }

    async fn send_message(&self, message: IPCMessage) -> Result<IPCResponse> {
        let mut stream = TcpStream::connect(self.addr).await?;

        // Serialize and send message
        let msg_json = serde_json::to_string(&message)?;
        let msg_bytes = msg_json.as_bytes();
        let len = msg_bytes.len() as u32;

        stream.write_all(&len.to_be_bytes()).await?;
        stream.write_all(msg_bytes).await?;
        stream.flush().await?;

        // Read response
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes) as usize;

        let mut buffer = vec![0u8; len];
        stream.read_exact(&mut buffer).await?;

        let response_json = String::from_utf8(buffer)?;
        let response = serde_json::from_str(&response_json)?;

        Ok(response)
    }
}

#[async_trait]
impl LauncherClient for IPCClient {
    async fn list_apps(&self) -> Result<Vec<AppMetadata>> {
        match self.send_message(IPCMessage::ListApps).await? {
            IPCResponse::AppsList(apps) => Ok(apps),
            IPCResponse::Error(e) => Err(anyhow::anyhow!(e)),
            _ => Err(anyhow::anyhow!("Invalid response")),
        }
    }

    async fn get_app(&self, app_id: &str) -> Result<Option<AppMetadata>> {
        match self
            .send_message(IPCMessage::GetApp {
                app_id: app_id.to_string(),
            })
            .await?
        {
            IPCResponse::AppMetadata(app) => Ok(app),
            IPCResponse::Error(e) => Err(anyhow::anyhow!(e)),
            _ => Err(anyhow::anyhow!("Invalid response")),
        }
    }

    async fn search_apps(&self, query: &str) -> Result<Vec<AppMetadata>> {
        match self
            .send_message(IPCMessage::SearchApps {
                query: query.to_string(),
            })
            .await?
        {
            IPCResponse::SearchResults(apps) => Ok(apps),
            IPCResponse::Error(e) => Err(anyhow::anyhow!(e)),
            _ => Err(anyhow::anyhow!("Invalid response")),
        }
    }

    async fn launch_app(&self, request: LaunchRequest) -> Result<LaunchResponse> {
        match self.send_message(IPCMessage::LaunchApp { request }).await? {
            IPCResponse::LaunchResult(response) => Ok(response),
            IPCResponse::Error(e) => Err(anyhow::anyhow!(e)),
            _ => Err(anyhow::anyhow!("Invalid response")),
        }
    }

    async fn list_instances(&self) -> Result<Vec<AppInstance>> {
        match self.send_message(IPCMessage::ListInstances).await? {
            IPCResponse::InstancesList(instances) => Ok(instances),
            IPCResponse::Error(e) => Err(anyhow::anyhow!(e)),
            _ => Err(anyhow::anyhow!("Invalid response")),
        }
    }

    async fn terminate_app(&self, instance_id: &Uuid) -> Result<()> {
        match self
            .send_message(IPCMessage::TerminateApp {
                instance_id: *instance_id,
            })
            .await?
        {
            IPCResponse::TerminateResult(_) => Ok(()),
            IPCResponse::Error(e) => Err(anyhow::anyhow!(e)),
            _ => Err(anyhow::anyhow!("Invalid response")),
        }
    }

    async fn get_system_status(&self) -> Result<SystemStatus> {
        match self.send_message(IPCMessage::GetStatus).await? {
            IPCResponse::Status(status) => Ok(status),
            IPCResponse::Error(e) => Err(anyhow::anyhow!(e)),
            _ => Err(anyhow::anyhow!("Invalid response")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_message_serialization() {
        let msg = IPCMessage::ListApps;
        let json = serde_json::to_string(&msg).unwrap();
        let _deserialized: IPCMessage = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_ipc_response_serialization() {
        let resp = IPCResponse::InstancesList(vec![]);
        let json = serde_json::to_string(&resp).unwrap();
        let _deserialized: IPCResponse = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_ipc_client_creation() {
        let addr = "127.0.0.1:9000".parse().unwrap();
        let _client = IPCClient::new(addr);
    }
}
