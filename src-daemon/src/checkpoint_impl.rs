use std::sync::Arc;
use bonsai_cas::{CasStore, CasKey};
use tracing::info;

use crate::state::DaemonState;

/// Snapshot the daemon's durable state to CAS and return the key.
/// Restoring from the key re-populates `state.transfers`.
pub async fn checkpoint(state: &Arc<DaemonState>, cas: &CasStore) -> Result<CasKey, String> {
    // Collect current transfer statuses
    let transfers = state.transfers.lock().await;
    let snap = serde_json::to_vec(&*transfers).map_err(|e| e.to_string())?;
    drop(transfers);

    let key = cas.put(&snap, "application/json")
        .await
        .map_err(|e| e.to_string())?;
    info!(cas_key = %key, "daemon state checkpointed");
    Ok(key)
}

/// Restore transfer statuses from a previous checkpoint.
pub async fn restore(state: &Arc<DaemonState>, cas: &CasStore, key: &CasKey) -> Result<(), String> {
    let data = cas.get(key).await.map_err(|e| e.to_string())?
        .ok_or_else(|| format!("checkpoint key not found: {key}"))?;
    let map: std::collections::HashMap<String, bonsai_transfer_core::transfer::TransferStatus> =
        serde_json::from_slice(&data).map_err(|e| e.to_string())?;
    *state.transfers.lock().await = map;
    info!(cas_key = %key, "daemon state restored from checkpoint");
    Ok(())
}
