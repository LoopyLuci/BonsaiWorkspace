//! ECF-RG Adaptive Transfer Engine.
//!
//! **Earliest Completion First with Reorder Guard** (ECF-RG):
//!
//! For each chunk, the scheduler:
//! 1. Evaluates all available lanes and estimates completion time as:
//!    ETA = rtt_ms/2 + (chunk_size / bandwidth_bps)
//! 2. Selects the lane with the lowest ETA (Earliest Completion First).
//! 3. Enforces a *Reorder Guard*: the selected lane must not push the
//!    in-flight GSN gap beyond MAX_REORDER_GAP, preventing reassembly
//!    deadlock at the receiver.
//! 4. For critical chunks (is_critical=true), sends redundantly on a
//!    second lane (QoS mirroring); the first arrival wins.

use crate::lane::{LaneHealth, LaneKind, TransportLane};
use std::collections::HashMap;
use std::sync::Arc;

/// Maximum GSN gap before the reorder guard blocks a lane.
const MAX_REORDER_GAP: u32 = 256;

/// The result of a scheduling decision for one chunk.
#[derive(Debug, Clone)]
pub struct ChunkAssignment {
    /// Primary lane to use.
    pub primary: String,
    /// Optional mirroring lane (for critical chunks).
    pub mirror: Option<String>,
    /// Estimated delivery time in seconds.
    pub eta_secs: f64,
}

/// Per-lane runtime state tracked by the scheduler.
#[derive(Debug, Default)]
struct LaneState {
    in_flight: u32,
    highest_gsn_in_flight: u64,
}

/// The ECF-RG adaptive scheduler.
pub struct EcfRgScheduler {
    lanes: HashMap<String, Arc<dyn TransportLane>>,
    state: HashMap<String, LaneState>,
    /// GSN of the highest chunk sent so far across all lanes.
    global_max_gsn: u64,
}

impl EcfRgScheduler {
    pub fn new() -> Self {
        Self {
            lanes: HashMap::new(),
            state: HashMap::new(),
            global_max_gsn: 0,
        }
    }

    /// Register a lane. Returns the lane name key.
    pub fn add_lane(&mut self, lane: Arc<dyn TransportLane>) -> String {
        let name = lane.name().to_string();
        self.state.entry(name.clone()).or_default();
        self.lanes.insert(name.clone(), lane);
        name
    }

    /// Remove a lane (e.g., after failure).
    pub fn remove_lane(&mut self, name: &str) {
        self.lanes.remove(name);
        self.state.remove(name);
    }

    /// Select the best lane for a chunk of `chunk_size` bytes with the given `gsn`.
    ///
    /// Returns `None` if no lanes are available.
    pub fn assign(
        &mut self,
        gsn: u64,
        chunk_size: usize,
        is_critical: bool,
    ) -> Option<ChunkAssignment> {
        let available: Vec<(&String, &Arc<dyn TransportLane>, LaneHealth)> = self
            .lanes
            .iter()
            .filter(|(name, lane)| {
                let h = lane.health();
                if !h.available {
                    return false;
                }
                // Reorder guard: reject lanes where accepting this chunk would
                // push the gap beyond MAX_REORDER_GAP.
                let state = self.state.get(*name).unwrap();
                if state.in_flight > 0 {
                    let gap = gsn.saturating_sub(state.highest_gsn_in_flight);
                    if gap > MAX_REORDER_GAP as u64 {
                        return false;
                    }
                }
                true
            })
            .map(|(name, lane)| {
                let health = lane.health();
                (name, lane, health)
            })
            .collect();

        if available.is_empty() {
            return None;
        }

        // Sort by estimated completion time
        let mut ranked: Vec<(&str, f64)> = available
            .iter()
            .map(|(name, _, health)| {
                let eta = health.estimated_completion_secs(chunk_size as u64);
                (name.as_str(), eta)
            })
            .collect();
        ranked.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let (primary_name, eta) = ranked[0];

        // Update scheduler state
        if let Some(st) = self.state.get_mut(primary_name) {
            st.in_flight += 1;
            st.highest_gsn_in_flight = st.highest_gsn_in_flight.max(gsn);
        }
        if gsn > self.global_max_gsn {
            self.global_max_gsn = gsn;
        }

        // QoS mirroring for critical chunks: pick the second-best lane
        let mirror = if is_critical && ranked.len() > 1 {
            let mirror_name = ranked[1].0.to_string();
            if let Some(st) = self.state.get_mut(mirror_name.as_str()) {
                st.in_flight += 1;
            }
            Some(mirror_name)
        } else {
            None
        };

        Some(ChunkAssignment {
            primary: primary_name.to_string(),
            mirror,
            eta_secs: eta,
        })
    }

    /// Called when a chunk ACK arrives — decrements in-flight count.
    pub fn on_ack(&mut self, lane_name: &str, _gsn: u64) {
        if let Some(st) = self.state.get_mut(lane_name) {
            st.in_flight = st.in_flight.saturating_sub(1);
        }
    }

    /// Called when a lane reports a failure — marks it unavailable.
    pub fn on_lane_failure(&mut self, lane_name: &str) {
        self.remove_lane(lane_name);
    }

    /// List all registered lane names and their current health.
    pub fn lane_summary(&self) -> Vec<(String, LaneKind, LaneHealth)> {
        self.lanes
            .iter()
            .map(|(name, lane)| (name.clone(), lane.kind(), lane.health()))
            .collect()
    }
}

impl Default for EcfRgScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lane::InProcessLane;

    #[test]
    fn assigns_to_best_lane() {
        let mut sched = EcfRgScheduler::new();
        let (lane1, _rx1) = InProcessLane::new_pair("fast");
        let (lane2, _rx2) = InProcessLane::new_pair("slow");
        sched.add_lane(Arc::new(lane1));
        sched.add_lane(Arc::new(lane2));

        let assignment = sched.assign(0, 1024, false).unwrap();
        // Both in-process lanes are identical health, but it should pick one
        assert!(!assignment.primary.is_empty());
    }

    #[test]
    fn critical_chunk_gets_mirror() {
        let mut sched = EcfRgScheduler::new();
        let (lane1, _rx1) = InProcessLane::new_pair("a");
        let (lane2, _rx2) = InProcessLane::new_pair("b");
        sched.add_lane(Arc::new(lane1));
        sched.add_lane(Arc::new(lane2));

        let assignment = sched.assign(0, 512, true).unwrap();
        assert!(
            assignment.mirror.is_some(),
            "critical chunks must be mirrored"
        );
    }

    #[test]
    fn no_lanes_returns_none() {
        let mut sched = EcfRgScheduler::new();
        assert!(sched.assign(0, 1024, false).is_none());
    }
}
