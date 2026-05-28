//! High-level Transfer — orchestrates chunking, encrypting, scheduling, and sending.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use bonsai_transfer_crypto::{session::SessionKey, cipher::encrypt_chunk};
use crate::gsn::GsnAllocator;
use crate::scheduler::EcfRgScheduler;
use crate::lane::TransportLane;
use crate::error::{TransferError, TransferResult};

/// Default chunk size: 256 KiB. Balances per-chunk overhead vs. memory use.
pub const DEFAULT_CHUNK_SIZE: usize = 256 * 1024;

/// Maximum chunk size: 16 MiB.
pub const MAX_CHUNK_SIZE: usize = 16 * 1024 * 1024;

// ── Transfer direction ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferDirection {
    Send,
    Receive,
}

// ── Transfer status ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferStatus {
    pub id: Uuid,
    pub direction: TransferDirection,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub chunk_count: u64,
    pub chunks_done: u64,
    pub active_lanes: Vec<String>,
    pub state: TransferState,
    pub bytes_per_sec: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferState {
    Pending,
    Active,
    Paused,
    Complete,
    Failed(String),
    Cancelled,
}

impl TransferStatus {
    pub fn progress(&self) -> f32 {
        if self.total_bytes == 0 { return 0.0; }
        (self.transferred_bytes as f64 / self.total_bytes as f64) as f32
    }
}

// ── TransferHandle — cancellation + progress ──────────────────────────────────

/// A lightweight handle to a running transfer.
#[derive(Clone)]
pub struct TransferHandle {
    pub id: Uuid,
    cancel_flag: Arc<AtomicBool>,
    bytes_sent: Arc<AtomicU64>,
    total_bytes: u64,
}

impl TransferHandle {
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }

    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent.load(Ordering::Relaxed)
    }

    pub fn progress(&self) -> f32 {
        if self.total_bytes == 0 { return 0.0; }
        self.bytes_sent() as f32 / self.total_bytes as f32
    }
}

// ── Transfer — the main send engine ──────────────────────────────────────────

pub struct Transfer {
    pub id: Uuid,
    gsn: GsnAllocator,
}

impl Transfer {
    pub fn new() -> Self {
        Self { id: Uuid::new_v4(), gsn: GsnAllocator::new() }
    }

    /// Send `data` in chunks across the given lanes using the session key.
    /// Returns a `TransferHandle` for monitoring and cancellation.
    pub async fn send_data(
        &self,
        data: Vec<u8>,
        session_key: Arc<SessionKey>,
        scheduler: Arc<tokio::sync::Mutex<EcfRgScheduler>>,
        lanes: Arc<std::collections::HashMap<String, Arc<dyn TransportLane>>>,
        chunk_size: usize,
        progress_tx: Option<mpsc::UnboundedSender<TransferStatus>>,
    ) -> TransferResult<TransferHandle> {
        let total_bytes = data.len() as u64;
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let bytes_sent = Arc::new(AtomicU64::new(0));
        let handle = TransferHandle {
            id: self.id,
            cancel_flag: cancel_flag.clone(),
            bytes_sent: bytes_sent.clone(),
            total_bytes,
        };

        let id = self.id;
        let gsn = self.gsn.clone();
        let chunk_size = chunk_size.min(MAX_CHUNK_SIZE).max(1);

        tokio::spawn(async move {
            let chunks: Vec<&[u8]> = data.chunks(chunk_size).collect();
            let total_chunks = chunks.len() as u64;
            let mut done_chunks = 0u64;

            for chunk_data in chunks {
                if cancel_flag.load(Ordering::Relaxed) {
                    break;
                }

                let chunk_gsn = gsn.next();
                let is_last = done_chunks == total_chunks - 1;
                let is_critical = is_last; // Last chunk is critical (signals completion)

                let ct = match encrypt_chunk(&session_key, chunk_gsn, chunk_data) {
                    Ok(ct) => ct,
                    Err(e) => {
                        tracing::error!("encrypt failed for GSN {chunk_gsn}: {e}");
                        break;
                    }
                };

                // Schedule
                let assignment = {
                    let mut sched = scheduler.lock().await;
                    sched.assign(chunk_gsn, chunk_data.len(), is_critical)
                };

                let Some(assignment) = assignment else {
                    tracing::error!("no lanes available for GSN {chunk_gsn}");
                    break;
                };

                // Send on primary lane
                if let Some(lane) = lanes.get(&assignment.primary) {
                    if let Err(e) = lane.send_chunk(&ct).await {
                        tracing::warn!("primary lane {} failed: {e}", assignment.primary);
                    }
                }

                // Mirror for critical chunks
                if let Some(mirror_name) = &assignment.mirror {
                    if let Some(lane) = lanes.get(mirror_name) {
                        let _ = lane.send_chunk(&ct).await;
                    }
                }

                let sent = chunk_data.len() as u64;
                bytes_sent.fetch_add(sent, Ordering::Relaxed);
                done_chunks += 1;

                // Emit progress
                if let Some(ref tx) = progress_tx {
                    let status = TransferStatus {
                        id,
                        direction: TransferDirection::Send,
                        total_bytes,
                        transferred_bytes: bytes_sent.load(Ordering::Relaxed),
                        chunk_count: total_chunks,
                        chunks_done: done_chunks,
                        active_lanes: vec![assignment.primary.clone()],
                        state: if done_chunks == total_chunks {
                            TransferState::Complete
                        } else {
                            TransferState::Active
                        },
                        bytes_per_sec: 0.0, // TODO: measure window
                    };
                    let _ = tx.send(status);
                }
            }
        });

        Ok(handle)
    }
}

impl Default for Transfer {
    fn default() -> Self { Self::new() }
}
