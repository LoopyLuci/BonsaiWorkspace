//! Time-travel debugger for the Sylva VM.
//!
//! Captures `Snapshot`s of VM state at call boundaries and supports
//! `:rewind N` (restore to N calls ago) and `:replay N` (re-execute from snapshot N).

use crate::vm::{SylvaValue, VmError};
use std::collections::HashMap;

// ── Snapshot ──────────────────────────────────────────────────────────────────

/// A frozen snapshot of the VM's global environment at a call boundary.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Sequence number (monotonically increasing).
    pub seq: u64,
    /// Human-readable label (function name + line, if available).
    pub label: String,
    /// Copy of all global bindings at the time the snapshot was taken.
    pub globals: HashMap<String, SylvaValue>,
    /// Source text that was being evaluated (if captured).
    pub source: Option<String>,
}

// ── RewindError ───────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum RewindError {
    #[error("no snapshots available")]
    NoSnapshots,
    #[error("snapshot index {0} out of range (have {1} snapshots)")]
    OutOfRange(usize, usize),
    #[error("VM error during replay: {0}")]
    Vm(#[from] VmError),
}

// ── Debugger ──────────────────────────────────────────────────────────────────

/// Ring-buffer time-travel debugger.
///
/// Keep the last `capacity` snapshots in a circular buffer.  The debugger is
/// decoupled from the VM: the VM calls `push_snapshot` at each call boundary,
/// and the REPL calls `rewind` / `replay` to restore or re-execute.
pub struct Debugger {
    capacity: usize,
    snapshots: Vec<Snapshot>,
    /// Index into `snapshots` of the next write slot.
    head: usize,
    /// Total number of snapshots ever pushed (for seq numbers).
    total: u64,
}

impl Debugger {
    /// Create a new debugger with the given ring-buffer capacity.
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            capacity,
            snapshots: Vec::with_capacity(capacity),
            head: 0,
            total: 0,
        }
    }

    /// Push a new snapshot into the ring buffer.
    pub fn push_snapshot(
        &mut self,
        label: impl Into<String>,
        globals: HashMap<String, SylvaValue>,
        source: Option<String>,
    ) {
        let snap = Snapshot {
            seq: self.total,
            label: label.into(),
            globals,
            source,
        };
        self.total += 1;
        if self.snapshots.len() < self.capacity {
            self.snapshots.push(snap);
            self.head = self.snapshots.len() % self.capacity;
        } else {
            self.snapshots[self.head] = snap;
            self.head = (self.head + 1) % self.capacity;
        }
    }

    /// Return the number of snapshots currently held.
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    /// Iterate snapshots from oldest to newest.
    pub fn iter_ordered(&self) -> impl Iterator<Item = &Snapshot> {
        let n = self.snapshots.len();
        if n < self.capacity {
            // Buffer not yet full — snapshots are in insertion order.
            let (a, b) = self.snapshots.split_at(0);
            b.iter().chain(a.iter())
        } else {
            // Buffer full — oldest is at `head`.
            let (older, newer) = self.snapshots.split_at(self.head);
            newer.iter().chain(older.iter())
        }
    }

    /// Return the snapshot `steps` calls ago (0 = most recent).
    pub fn get_ago(&self, steps: usize) -> Result<&Snapshot, RewindError> {
        let n = self.snapshots.len();
        if n == 0 {
            return Err(RewindError::NoSnapshots);
        }
        if steps >= n {
            return Err(RewindError::OutOfRange(steps, n));
        }
        // Most-recent snapshot is at index `(head - 1 + n) % n` in the ring.
        let most_recent = (self.head + n - 1) % n;
        let idx = (most_recent + n - steps) % n;
        Ok(&self.snapshots[idx])
    }

    /// List all stored snapshots as `(index_ago, seq, label)`.
    pub fn list(&self) -> Vec<(usize, u64, &str)> {
        self.iter_ordered()
            .enumerate()
            .map(|(i, s)| {
                let ago = self.snapshots.len() - 1 - i;
                (ago, s.seq, s.label.as_str())
            })
            .collect()
    }

    /// Restore the globals of a VM from a snapshot `steps` calls ago.
    ///
    /// Returns a clone of the snapshot's global map so the caller can
    /// re-populate `vm.env`.
    pub fn rewind(&self, steps: usize) -> Result<HashMap<String, SylvaValue>, RewindError> {
        let snap = self.get_ago(steps)?;
        Ok(snap.globals.clone())
    }

    /// Return the source text from a snapshot `steps` calls ago (for replay).
    pub fn replay_source(&self, steps: usize) -> Result<Option<String>, RewindError> {
        let snap = self.get_ago(steps)?;
        Ok(snap.source.clone())
    }
}

// ── Default ───────────────────────────────────────────────────────────────────

impl Default for Debugger {
    fn default() -> Self {
        Self::new(256)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn globals(n: i64) -> HashMap<String, SylvaValue> {
        let mut m = HashMap::new();
        m.insert("x".into(), SylvaValue::Int(n));
        m
    }

    #[test]
    fn push_and_rewind() {
        let mut dbg = Debugger::new(8);
        for i in 0..5i64 {
            dbg.push_snapshot(format!("call_{i}"), globals(i), None);
        }
        assert_eq!(dbg.len(), 5);
        // steps=0 → most recent (i=4)
        let g = dbg.rewind(0).unwrap();
        assert_eq!(g["x"], SylvaValue::Int(4));
        // steps=4 → oldest (i=0)
        let g = dbg.rewind(4).unwrap();
        assert_eq!(g["x"], SylvaValue::Int(0));
    }

    #[test]
    fn ring_wraps() {
        let mut dbg = Debugger::new(4);
        for i in 0..7i64 {
            dbg.push_snapshot(format!("c{i}"), globals(i), None);
        }
        assert_eq!(dbg.len(), 4); // capacity capped
                                  // Most recent should be i=6
        let g = dbg.rewind(0).unwrap();
        assert_eq!(g["x"], SylvaValue::Int(6));
        // Oldest in ring should be i=3
        let g = dbg.rewind(3).unwrap();
        assert_eq!(g["x"], SylvaValue::Int(3));
    }

    #[test]
    fn out_of_range_error() {
        let mut dbg = Debugger::new(8);
        dbg.push_snapshot("a", globals(1), None);
        assert!(matches!(dbg.rewind(1), Err(RewindError::OutOfRange(1, 1))));
    }

    #[test]
    fn empty_error() {
        let dbg = Debugger::new(8);
        assert!(matches!(dbg.rewind(0), Err(RewindError::NoSnapshots)));
    }
}
