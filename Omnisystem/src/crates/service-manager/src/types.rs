//! Core types for Service Lifecycle Manager

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Service state in the lifecycle state machine
#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceState {
    Unstarted,   // Never spawned or after restore from archive
    Spawning,    // Creating vault, loading binary
    Running,     // Vault active, accepting requests
    Pausing,     // on_pause() called, state being serialized
    Paused,      // Snapshot in CAS, vault destroyed
    Archived,    // Snapshot moved to cold storage
    Restoring,   // Restoring from snapshot
    Failed,      // Unrecoverable failure
}

impl ServiceState {
    pub fn is_running(&self) -> bool {
        matches!(self, ServiceState::Running)
    }

    pub fn is_paused(&self) -> bool {
        matches!(self, ServiceState::Paused | ServiceState::Archived)
    }

    pub fn can_restore(&self) -> bool {
        matches!(self, ServiceState::Paused | ServiceState::Archived | ServiceState::Failed)
    }
}

/// Resource quota for a service
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ResourceQuota {
    /// Memory limit in MB
    pub memory_mb: u32,

    /// CPU cores allocated
    pub cpu_cores: f32,

    /// Max CPU percentage (0-100)
    pub cpu_percent_max: u32,

    /// I/O operations per second limit
    pub iops_limit: u32,

    /// Max number of snapshots to keep
    pub max_snapshots: u32,

    /// Max total snapshot size in MB
    pub max_snapshot_size_mb: u32,
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            memory_mb: 512,
            cpu_cores: 1.0,
            cpu_percent_max: 50,
            iops_limit: 1000,
            max_snapshots: 5,
            max_snapshot_size_mb: 256,
        }
    }
}

/// Service manifest (read from UMS)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceManifest {
    /// Service name (e.g., "fax", "scanner")
    pub name: String,

    /// Service version
    pub version: String,

    /// Binary hash in CAS
    pub binary_hash: String,

    /// Required capabilities
    pub capabilities_required: Vec<String>,

    /// Resource limits
    pub quota: ResourceQuota,

    /// Timeout before auto-pause (seconds)
    pub idle_timeout_secs: u32,

    /// Archive timeout (hours)
    pub archive_after_hours: u32,

    /// Heartbeat interval (seconds)
    pub heartbeat_interval_secs: u32,

    /// Heartbeat timeout (seconds)
    pub heartbeat_timeout_secs: u32,

    /// Council signature
    pub signature: String,
}

/// Snapshot metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    /// BLAKE3 hash of snapshot content
    pub hash: String,

    /// Size in bytes
    pub size_bytes: u64,

    /// Timestamp when snapshot was created
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Whether snapshot is archived (in cold storage)
    pub archived: bool,
}

/// Service instance being managed by SLM
#[derive(Clone, Debug)]
pub struct ServiceInstance {
    /// Unique service instance ID
    pub instance_id: Uuid,

    /// Service manifest
    pub manifest: ServiceManifest,

    /// Current state
    pub state: ServiceState,

    /// Vault ID if running (None if paused/archived)
    pub vault_id: Option<u64>,

    /// Latest snapshot
    pub latest_snapshot: Option<Snapshot>,

    /// All snapshots (for rotation)
    pub snapshots: Vec<Snapshot>,

    /// Last access timestamp
    pub last_access_timestamp: u64,

    /// Consecutive failures
    pub consecutive_failures: u32,

    /// Resource usage stats
    pub resource_usage: ResourceUsage,
}

/// Current resource usage for a service
#[derive(Clone, Debug, Default)]
pub struct ResourceUsage {
    /// Current memory usage in MB
    pub memory_used_mb: u32,

    /// Current CPU usage percentage
    pub cpu_percent: u32,

    /// IOPS in progress
    pub iops_current: u32,

    /// Timestamp of last measurement
    pub measured_at: u64,
}

/// Health status reported by service heartbeat
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Is service healthy
    pub healthy: bool,

    /// Memory used in MB
    pub memory_used_mb: u32,

    /// CPU usage percentage
    pub cpu_percent: u32,

    /// Optional message
    pub message: Option<String>,
}

/// Audit log event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Event ID
    pub id: Uuid,

    /// Event type
    pub event_type: String,

    /// Service name
    pub service_name: String,

    /// Service instance ID
    pub instance_id: Uuid,

    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Event details (JSON)
    pub details: serde_json::Value,
}

/// Configuration for SLM
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SLMConfig {
    /// Idle timeout before pausing (seconds)
    pub idle_timeout_secs: u32,

    /// Archive timeout after idle (hours)
    pub archive_after_hours: u32,

    /// Health check interval (seconds)
    pub health_check_interval_secs: u32,

    /// Max consecutive failures before restart
    pub max_consecutive_failures: u32,

    /// Max services per SLM
    pub max_services: u32,

    /// Total memory quota for all services (MB)
    pub total_memory_quota_mb: u32,
}

impl Default for SLMConfig {
    fn default() -> Self {
        Self {
            idle_timeout_secs: 300,
            archive_after_hours: 24,
            health_check_interval_secs: 10,
            max_consecutive_failures: 3,
            max_services: 10000,
            total_memory_quota_mb: 100000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_state_transitions() {
        assert!(!ServiceState::Unstarted.is_running());
        assert!(ServiceState::Running.is_running());
        assert!(!ServiceState::Paused.is_running());
        assert!(ServiceState::Paused.is_paused());
    }

    #[test]
    fn test_resource_quota_default() {
        let quota = ResourceQuota::default();
        assert_eq!(quota.memory_mb, 512);
        assert_eq!(quota.cpu_cores, 1.0);
    }

    #[test]
    fn test_snapshot_metadata() {
        let snap = Snapshot {
            hash: "abc123".to_string(),
            size_bytes: 1024,
            created_at: chrono::Utc::now(),
            archived: false,
        };
        assert_eq!(snap.size_bytes, 1024);
        assert!(!snap.archived);
    }
}
