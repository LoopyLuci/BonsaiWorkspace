//! Reassembly Window — ordered delivery with NACK-based gap detection.
//!
//! The receiver buffers out-of-order chunks and delivers them in GSN order.
//! If a gap persists beyond the NACK timeout, a NACK is emitted so the
//! sender can retransmit on the fastest available lane.

use crate::error::TransferResult;
use p2p_crypto::cipher::ChunkCiphertext;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// Gap timeout before a NACK is emitted (milliseconds).
const GAP_NACK_TIMEOUT_MS: u64 = 500;

/// A complete message assembled from its chunks.
#[derive(Debug)]
pub struct AssembledMessage {
    /// All decrypted chunk payloads concatenated in GSN order.
    pub data: Vec<u8>,
    /// The GSN range [first, last].
    pub gsn_range: (u64, u64),
    /// Total chunks received.
    pub chunk_count: usize,
}

/// Pending chunk awaiting delivery.
struct PendingChunk {
    chunk: ChunkCiphertext,
    arrived_at: Instant,
}

/// The reassembly window for a single transfer.
pub struct ReassemblyWindow {
    /// Chunks buffered out of order: GSN → chunk.
    buffer: BTreeMap<u64, PendingChunk>,
    /// The next expected GSN.
    next_expected: u64,
    /// Total number of expected chunks (known after the transfer header arrives).
    expected_total: Option<u64>,
    /// GSNs for which NACKs have already been sent (to avoid duplicates).
    nacked: std::collections::HashSet<u64>,
}

impl ReassemblyWindow {
    pub fn new(first_gsn: u64) -> Self {
        Self {
            buffer: BTreeMap::new(),
            next_expected: first_gsn,
            expected_total: None,
            nacked: std::collections::HashSet::new(),
        }
    }

    /// Set the expected total chunk count (sent in the transfer header).
    pub fn set_expected_total(&mut self, total: u64) {
        self.expected_total = Some(total);
    }

    /// Deliver an incoming chunk into the reassembly buffer.
    pub fn receive(&mut self, chunk: ChunkCiphertext) {
        let gsn = chunk.gsn;
        if gsn < self.next_expected {
            return; // Duplicate — already delivered
        }
        self.buffer.entry(gsn).or_insert(PendingChunk {
            chunk,
            arrived_at: Instant::now(),
        });
    }

    /// Drain all contiguously-available chunks starting at `next_expected`.
    /// Returns decrypted payloads in order using the provided decrypt function.
    pub fn drain_ready<F>(&mut self, decrypt: F) -> TransferResult<Vec<(u64, Vec<u8>)>>
    where
        F: Fn(&ChunkCiphertext) -> TransferResult<Vec<u8>>,
    {
        let mut out = Vec::new();
        while let Some(pending) = self.buffer.remove(&self.next_expected) {
            let plain = decrypt(&pending.chunk)?;
            out.push((self.next_expected, plain));
            self.next_expected += 1;
        }
        Ok(out)
    }

    /// Check for stale gaps and return GSNs that need NACKing.
    pub fn pending_nacks(&mut self) -> Vec<u64> {
        let timeout = Duration::from_millis(GAP_NACK_TIMEOUT_MS);
        let mut nacks = Vec::new();
        let now = Instant::now();

        // Find the first gap: [next_expected .. first buffered GSN)
        if let Some((&first_buffered, _)) = self.buffer.iter().next() {
            for gsn in self.next_expected..first_buffered {
                if !self.nacked.contains(&gsn) {
                    // Only NACK if we've been waiting long enough
                    if let Some(first_pending) = self.buffer.values().next() {
                        if now.duration_since(first_pending.arrived_at) >= timeout {
                            nacks.push(gsn);
                            self.nacked.insert(gsn);
                        }
                    } else {
                        nacks.push(gsn);
                        self.nacked.insert(gsn);
                    }
                }
            }
        }
        nacks
    }

    /// Returns true if all expected chunks have been received and drained.
    pub fn is_complete(&self) -> bool {
        match self.expected_total {
            Some(total) => self.next_expected >= total,
            None => false,
        }
    }

    pub fn next_expected(&self) -> u64 {
        self.next_expected
    }
    pub fn buffered_count(&self) -> usize {
        self.buffer.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p2p_crypto::{
        cipher::{decrypt_chunk, encrypt_chunk},
        session::SessionKey,
    };

    fn make_chunk(key: &SessionKey, gsn: u64, data: &[u8]) -> ChunkCiphertext {
        encrypt_chunk(key, gsn, data).unwrap()
    }

    #[test]
    fn in_order_delivery() {
        let key = SessionKey([1u8; 32]);
        let mut win = ReassemblyWindow::new(0);
        win.set_expected_total(3);

        for i in 0..3u64 {
            win.receive(make_chunk(&key, i, &[i as u8; 16]));
        }

        let ready = win
            .drain_ready(|c| decrypt_chunk(&key, c).map_err(Into::into))
            .unwrap();
        assert_eq!(ready.len(), 3);
        assert_eq!(ready[0].0, 0);
        assert_eq!(ready[1].0, 1);
        assert_eq!(ready[2].0, 2);
    }

    #[test]
    fn out_of_order_buffered() {
        let key = SessionKey([2u8; 32]);
        let mut win = ReassemblyWindow::new(0);

        // Receive 2 then 1 then 0
        win.receive(make_chunk(&key, 2, b"chunk2"));
        win.receive(make_chunk(&key, 1, b"chunk1"));
        // GSN 0 not yet arrived → nothing drainable
        let ready = win
            .drain_ready(|c| decrypt_chunk(&key, c).map_err(Into::into))
            .unwrap();
        assert_eq!(ready.len(), 0);

        // Now GSN 0 arrives
        win.receive(make_chunk(&key, 0, b"chunk0"));
        let ready = win
            .drain_ready(|c| decrypt_chunk(&key, c).map_err(Into::into))
            .unwrap();
        assert_eq!(ready.len(), 3);
    }
}
