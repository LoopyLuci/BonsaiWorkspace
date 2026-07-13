//! Advanced fault types beyond basic fault injection.
//!
//! This module defines sophisticated fault models including latent faults,
//! cascading faults, transient faults, Byzantine faults, and silent faults.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Advanced fault type identifier.
pub type AdvancedFaultId = Uuid;

/// Latent fault: injected but manifests after delay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatentFault {
    /// Unique ID for this latent fault.
    pub id: AdvancedFaultId,
    /// Description of the underlying fault.
    pub description: String,
    /// When injected (epoch seconds).
    pub injected_at: u64,
    /// When the fault manifests (delay in seconds).
    pub manifest_delay_secs: u64,
    /// Duration of the fault once manifested.
    pub active_duration_secs: u64,
    /// Triggering conditions (e.g., "under_load", "specific_operation").
    pub trigger_conditions: Vec<String>,
    /// Metadata about this latent fault.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl LatentFault {
    /// Create a new latent fault.
    pub fn new(
        description: String,
        injected_at: u64,
        manifest_delay_secs: u64,
        active_duration_secs: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            description,
            injected_at,
            manifest_delay_secs,
            active_duration_secs,
            trigger_conditions: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Get when this fault will manifest.
    pub fn manifest_time(&self) -> u64 {
        self.injected_at + self.manifest_delay_secs
    }

    /// Get when this fault will recover.
    pub fn recovery_time(&self) -> u64 {
        self.manifest_time() + self.active_duration_secs
    }

    /// Check if fault is currently manifested at given time.
    pub fn is_manifested_at(&self, time: u64) -> bool {
        time >= self.manifest_time() && time < self.recovery_time()
    }

    /// Add a trigger condition.
    pub fn with_trigger_condition(mut self, condition: String) -> Self {
        self.trigger_conditions.push(condition);
        self
    }
}

/// Cascading fault: one fault triggers others.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadingFault {
    /// Unique ID for this cascading fault.
    pub id: AdvancedFaultId,
    /// Root cause fault description.
    pub root_cause: String,
    /// Time when root cause is injected.
    pub root_inject_time: u64,
    /// Cascading faults triggered by root cause.
    pub cascading_effects: Vec<CascadingEffect>,
    /// Maximum cascade depth to prevent infinite loops.
    pub max_depth: u32,
    /// Current propagation depth.
    pub current_depth: u32,
}

/// A single cascading effect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadingEffect {
    /// Description of the cascading fault.
    pub description: String,
    /// Delay before this cascade effect triggers.
    pub trigger_delay_secs: u64,
    /// How many other faults this can trigger.
    pub cascade_count: u32,
    /// Severity multiplier (1.0 = same as parent, > 1.0 = worse).
    pub severity_multiplier: f64,
}

impl CascadingFault {
    /// Create a new cascading fault.
    pub fn new(root_cause: String, root_inject_time: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            root_cause,
            root_inject_time,
            cascading_effects: Vec::new(),
            max_depth: 5,
            current_depth: 0,
        }
    }

    /// Add a cascading effect.
    pub fn with_effect(mut self, effect: CascadingEffect) -> Self {
        self.cascading_effects.push(effect);
        self
    }

    /// Check if more cascading is allowed.
    pub fn can_cascade(&self) -> bool {
        self.current_depth < self.max_depth && !self.cascading_effects.is_empty()
    }

    /// Propagate the cascade to next level.
    pub fn propagate(&mut self) -> Vec<CascadingEffect> {
        if self.can_cascade() {
            self.current_depth += 1;
            self.cascading_effects.clone()
        } else {
            Vec::new()
        }
    }
}

/// Transient fault: appears and disappears on schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransientFault {
    /// Unique ID.
    pub id: AdvancedFaultId,
    /// Description.
    pub description: String,
    /// Start time of first occurrence.
    pub start_time: u64,
    /// Duration of each occurrence.
    pub occurrence_duration_secs: u64,
    /// Time between occurrences.
    pub recurrence_interval_secs: u64,
    /// Total number of occurrences (0 = infinite).
    pub max_occurrences: u32,
    /// Current occurrence count.
    pub occurrence_count: u32,
}

impl TransientFault {
    /// Create a new transient fault.
    pub fn new(
        description: String,
        start_time: u64,
        occurrence_duration_secs: u64,
        recurrence_interval_secs: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            description,
            start_time,
            occurrence_duration_secs,
            recurrence_interval_secs,
            max_occurrences: 0, // infinite
            occurrence_count: 0,
        }
    }

    /// Check if fault is active at given time.
    pub fn is_active_at(&self, time: u64) -> bool {
        if time < self.start_time {
            return false;
        }

        let elapsed = time - self.start_time;
        let cycle_time = self.occurrence_duration_secs + self.recurrence_interval_secs;
        let position_in_cycle = elapsed % cycle_time;

        if self.max_occurrences > 0 {
            let occurrence_number = elapsed / cycle_time;
            if occurrence_number >= self.max_occurrences as u64 {
                return false;
            }
        }

        position_in_cycle < self.occurrence_duration_secs
    }

    /// Set maximum number of occurrences.
    pub fn with_max_occurrences(mut self, max: u32) -> Self {
        self.max_occurrences = max;
        self
    }
}

/// Byzantine fault: component gives inconsistent responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByzantineFault {
    /// Unique ID.
    pub id: AdvancedFaultId,
    /// Component identifier.
    pub component_id: String,
    /// Start time of Byzantine behavior.
    pub start_time: u64,
    /// Duration of Byzantine behavior.
    pub duration_secs: u64,
    /// Probability of byzantine response (0.0-1.0).
    pub byzantine_probability: f64,
    /// Types of inconsistencies this component exhibits.
    pub inconsistency_types: Vec<InconsistencyType>,
}

/// Type of Byzantine inconsistency.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InconsistencyType {
    /// Returns different results for identical queries.
    ResultMismatch,
    /// Returns stale/outdated data.
    StaleData,
    /// Partial failures on identical inputs.
    PartialFailure,
    /// Contradictory state responses.
    StateContradiction,
    /// Timing inconsistencies (sometimes fast, sometimes slow).
    TimingAnomaly,
}

impl ByzantineFault {
    /// Create a new Byzantine fault.
    pub fn new(
        component_id: String,
        start_time: u64,
        duration_secs: u64,
        byzantine_probability: f64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            component_id,
            start_time,
            duration_secs,
            byzantine_probability: byzantine_probability.clamp(0.0, 1.0),
            inconsistency_types: Vec::new(),
        }
    }

    /// Add an inconsistency type.
    pub fn with_inconsistency(mut self, inconsistency: InconsistencyType) -> Self {
        self.inconsistency_types.push(inconsistency);
        self
    }

    /// Check if Byzantine fault is active.
    pub fn is_active_at(&self, time: u64) -> bool {
        time >= self.start_time && time < self.start_time + self.duration_secs
    }
}

/// Silent fault: failures with no error indication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SilentFault {
    /// Unique ID.
    pub id: AdvancedFaultId,
    /// Description of silent failure.
    pub description: String,
    /// Operation that silently fails.
    pub target_operation: String,
    /// Start time.
    pub start_time: u64,
    /// Duration.
    pub duration_secs: u64,
    /// Failure rate (0.0-1.0).
    pub failure_rate: f64,
    /// Type of silent failure.
    pub failure_type: SilentFailureType,
}

/// Type of silent failure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SilentFailureType {
    /// Operation appears to succeed but has no effect.
    NoOp,
    /// Operation succeeds but returns wrong data.
    WrongResult,
    /// Operation silently drops data.
    DataLoss,
    /// Operation silently corrupts data.
    DataCorruption,
    /// Operation partially succeeds (some items fail silently).
    PartialSuccess,
}

impl SilentFault {
    /// Create a new silent fault.
    pub fn new(
        description: String,
        target_operation: String,
        start_time: u64,
        duration_secs: u64,
        failure_type: SilentFailureType,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            description,
            target_operation,
            start_time,
            duration_secs,
            failure_rate: 0.5,
            failure_type,
        }
    }

    /// Set failure rate.
    pub fn with_failure_rate(mut self, rate: f64) -> Self {
        self.failure_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Check if silent fault is active.
    pub fn is_active_at(&self, time: u64) -> bool {
        time >= self.start_time && time < self.start_time + self.duration_secs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latent_fault_timing() {
        let fault = LatentFault::new("db_connection_leak".to_string(), 100, 50, 30);
        assert_eq!(fault.manifest_time(), 150);
        assert_eq!(fault.recovery_time(), 180);
        assert!(!fault.is_manifested_at(145));
        assert!(fault.is_manifested_at(150));
        assert!(fault.is_manifested_at(179));
        assert!(!fault.is_manifested_at(180));
    }

    #[test]
    fn test_cascading_fault() {
        let effect = CascadingEffect {
            description: "connection timeout".to_string(),
            trigger_delay_secs: 10,
            cascade_count: 2,
            severity_multiplier: 1.5,
        };
        let mut fault = CascadingFault::new("database_crash".to_string(), 100);
        fault = fault.with_effect(effect);
        assert!(fault.can_cascade());
        assert_eq!(fault.propagate().len(), 1);
    }

    #[test]
    fn test_transient_fault() {
        let fault = TransientFault::new("network_jitter".to_string(), 100, 5, 10);
        // First cycle: 100-105 (active), 105-115 (inactive), 115-120 (active)
        assert!(!fault.is_active_at(99));
        assert!(fault.is_active_at(100));
        assert!(fault.is_active_at(104));
        assert!(!fault.is_active_at(105));
        assert!(!fault.is_active_at(114));
        assert!(fault.is_active_at(115));
    }

    #[test]
    fn test_byzantine_fault() {
        let fault = ByzantineFault::new(
            "consensus_node".to_string(),
            100,
            50,
            0.3,
        )
        .with_inconsistency(InconsistencyType::ResultMismatch);

        assert!(fault.is_active_at(125));
        assert!(!fault.is_active_at(150));
        assert_eq!(fault.inconsistency_types.len(), 1);
    }

    #[test]
    fn test_silent_fault() {
        let fault = SilentFault::new(
            "write operation silently fails".to_string(),
            "database_write".to_string(),
            100,
            30,
            SilentFailureType::NoOp,
        );

        assert!(fault.is_active_at(100));
        assert!(fault.is_active_at(129));
        assert!(!fault.is_active_at(130));
    }
}
