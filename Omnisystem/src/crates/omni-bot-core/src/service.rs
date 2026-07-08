//! Service management types

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Service state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceState {
    #[serde(rename = "unstarted")]
    Unstarted,
    #[serde(rename = "booting")]
    Booting,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "pausing")]
    Pausing,
    #[serde(rename = "paused")]
    Paused,
    #[serde(rename = "stopping")]
    Stopping,
    #[serde(rename = "stopped")]
    Stopped,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "archived")]
    Archived,
}

impl ServiceState {
    pub fn is_running(&self) -> bool {
        matches!(self, ServiceState::Running)
    }
    
    pub fn is_active(&self) -> bool {
        matches!(self, ServiceState::Running | ServiceState::Booting)
    }
}

/// Service status indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceStatus {
    #[serde(rename = "healthy")]
    Healthy,
    #[serde(rename = "degraded")]
    Degraded,
    #[serde(rename = "unhealthy")]
    Unhealthy,
    #[serde(rename = "unknown")]
    Unknown,
}

/// Resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_percent: f32,
    pub memory_mb: u32,
    pub disk_mb: u32,
    pub bandwidth_mbps: f32,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            cpu_percent: 0.0,
            memory_mb: 0,
            disk_mb: 0,
            bandwidth_mbps: 0.0,
        }
    }
}

/// Service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub version: String,
    pub state: ServiceState,
    pub status: ServiceStatus,
    pub uptime_seconds: u64,
    pub resource_usage: ResourceUsage,
    pub last_health_check: DateTime<Utc>,
    pub pid: Option<u32>,
    pub error: Option<String>,
}

impl ServiceInfo {
    pub fn new(name: String, version: String) -> Self {
        Self {
            name,
            version,
            state: ServiceState::Unstarted,
            status: ServiceStatus::Unknown,
            uptime_seconds: 0,
            resource_usage: ResourceUsage::default(),
            last_health_check: Utc::now(),
            pid: None,
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_service_state() {
        assert!(ServiceState::Running.is_running());
        assert!(!ServiceState::Stopped.is_running());
        assert!(ServiceState::Running.is_active());
        assert!(ServiceState::Booting.is_active());
    }
    
    #[test]
    fn test_service_info_creation() {
        let info = ServiceInfo::new("p2p".to_string(), "1.0.0".to_string());
        assert_eq!(info.state, ServiceState::Unstarted);
        assert_eq!(info.status, ServiceStatus::Unknown);
    }
}
