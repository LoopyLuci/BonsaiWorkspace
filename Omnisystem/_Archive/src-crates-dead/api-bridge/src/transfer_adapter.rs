use anyhow::{anyhow, Result};
use p2p_core::streams::{TaskDefinition, TaskDistributeStream, ResourceRequirements, TaskResult};
use serde::{Deserialize, Serialize};

use crate::transfer_client::TransferClientWrapper;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerApiEnvelope {
    pub peer_id: String,
    pub service: String,
    pub trace_id: String,
    pub payload: serde_json::Value,
}

#[derive(Clone, Default)]
pub struct TransferDaemonAdapter;

impl TransferDaemonAdapter {
    pub async fn call_remote_backend(
        &self,
        peer_id: &str,
        service: &str,
        trace_id: &str,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let envelope = PeerApiEnvelope {
            peer_id: peer_id.to_string(),
            service: service.to_string(),
            trace_id: trace_id.to_string(),
            payload: payload.clone(),
        };

        let encoded = TaskDistributeStream::encode_task(&TaskDefinition {
            task_id: format!("api-{}", trace_id),
            wasm_bytes: Vec::new(),
            input_data: serde_json::to_vec(&envelope)?,
            resource_requirements: ResourceRequirements {
                cpu_cores: 0.1,
                memory_mb: 16,
                gpu_required: false,
                network_mbps: 1.0,
            },
            deadline_ms: 15_000,
        });

        let response_bytes = self.open_stream_and_exchange(peer_id, service, encoded).await?;
        let result = TaskDistributeStream::decode_result(&response_bytes)
            .ok_or_else(|| anyhow!("invalid TransferDaemon task response frame"))?;
        decode_task_result(result)
    }

    async fn open_stream_and_exchange(
        &self,
        peer_id: &str,
        service: &str,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>> {
        let client = TransferClientWrapper::from_env();
        let session = client.connect(peer_id).await?;
        let mut stream = client
            .open_stream(&session, service)
            .await?;
        stream
            .exchange(&payload)
            .await
            .map_err(|e| anyhow::anyhow!("Stream exchange failed: {}", e))
    }
}

fn decode_task_result(result: TaskResult) -> Result<serde_json::Value> {
    if let Some(error) = result.error {
        return Err(anyhow!(error));
    }
    Ok(serde_json::from_slice(&result.output_data)?)
}
