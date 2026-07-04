//! Ecosystem state tracking during bootstrap

use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// State of each ecosystem component
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ComponentState {
    /// Not yet initialized
    Uninitialized,
    /// Initialization in progress
    Initializing,
    /// Successfully initialized
    Initialized,
    /// Health check passed
    Healthy,
    /// Component is degraded
    Degraded,
    /// Component failed
    Failed,
    /// Cleanup in progress
    Cleaning,
    /// Cleanup complete
    Cleaned,
}

impl std::fmt::Display for ComponentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentState::Uninitialized => write!(f, "Uninitialized"),
            ComponentState::Initializing => write!(f, "Initializing"),
            ComponentState::Initialized => write!(f, "Initialized"),
            ComponentState::Healthy => write!(f, "Healthy"),
            ComponentState::Degraded => write!(f, "Degraded"),
            ComponentState::Failed => write!(f, "Failed"),
            ComponentState::Cleaning => write!(f, "Cleaning"),
            ComponentState::Cleaned => write!(f, "Cleaned"),
        }
    }
}

/// Information about a single ecosystem component
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComponentInfo {
    pub name: String,
    pub state: ComponentState,
    pub initialized_at: Option<DateTime<Utc>>,
    pub last_health_check: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub memory_mb: u64,
    pub uptime_secs: u64,
}

impl ComponentInfo {
    /// Create a new component info
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            state: ComponentState::Uninitialized,
            initialized_at: None,
            last_health_check: None,
            error_message: None,
            memory_mb: 0,
            uptime_secs: 0,
        }
    }

    /// Mark component as initializing
    pub fn mark_initializing(mut self) -> Self {
        self.state = ComponentState::Initializing;
        self
    }

    /// Mark component as initialized
    pub fn mark_initialized(mut self) -> Self {
        self.state = ComponentState::Initialized;
        self.initialized_at = Some(Utc::now());
        self
    }

    /// Mark component as healthy
    pub fn mark_healthy(mut self) -> Self {
        self.state = ComponentState::Healthy;
        self.last_health_check = Some(Utc::now());
        self.error_message = None;
        self
    }

    /// Mark component as failed
    pub fn mark_failed(mut self, error: impl Into<String>) -> Self {
        self.state = ComponentState::Failed;
        self.error_message = Some(error.into());
        self
    }

    /// Update memory usage
    pub fn set_memory_mb(mut self, memory: u64) -> Self {
        self.memory_mb = memory;
        self
    }

    /// Update uptime
    pub fn set_uptime_secs(mut self, uptime: u64) -> Self {
        self.uptime_secs = uptime;
        self
    }
}

/// Overall ecosystem state
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EcosystemState {
    /// Component states
    pub components: HashMap<String, ComponentInfo>,
    /// Bootstrap start time
    pub bootstrap_start: Option<DateTime<Utc>>,
    /// Bootstrap completion time
    pub bootstrap_end: Option<DateTime<Utc>>,
    /// Overall health status
    pub is_healthy: bool,
    /// Total memory used by ecosystem
    pub total_memory_mb: u64,
    /// Number of failed components
    pub failed_components: usize,
}

impl Default for EcosystemState {
    fn default() -> Self {
        Self {
            components: HashMap::new(),
            bootstrap_start: None,
            bootstrap_end: None,
            is_healthy: false,
            total_memory_mb: 0,
            failed_components: 0,
        }
    }
}

impl EcosystemState {
    /// Create a new ecosystem state
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a component
    pub fn add_component(&mut self, name: impl Into<String>) {
        let name_str = name.into();
        self.components.insert(name_str.clone(), ComponentInfo::new(name_str));
    }

    /// Get or create a component
    pub fn get_or_create_component(&mut self, name: impl Into<String>) -> &mut ComponentInfo {
        let name_str = name.into();
        self.components
            .entry(name_str.clone())
            .or_insert_with(|| ComponentInfo::new(name_str))
    }

    /// Mark all components as healthy
    pub fn mark_all_healthy(&mut self) {
        self.is_healthy = true;
        self.failed_components = 0;
        for component in self.components.values_mut() {
            if component.state != ComponentState::Failed {
                component.state = ComponentState::Healthy;
                component.last_health_check = Some(Utc::now());
            }
        }
    }

    /// Update failed component count
    pub fn recalculate_health(&mut self) {
        self.failed_components = self
            .components
            .values()
            .filter(|c| c.state == ComponentState::Failed)
            .count();
        self.is_healthy = self.failed_components == 0;
    }

    /// Update total memory
    pub fn update_total_memory(&mut self) {
        self.total_memory_mb = self
            .components
            .values()
            .map(|c| c.memory_mb)
            .sum();
    }

    /// Start bootstrap timer
    pub fn start_bootstrap(&mut self) {
        self.bootstrap_start = Some(Utc::now());
    }

    /// End bootstrap timer
    pub fn end_bootstrap(&mut self) {
        self.bootstrap_end = Some(Utc::now());
    }

    /// Get bootstrap duration
    pub fn bootstrap_duration(&self) -> Option<std::time::Duration> {
        match (self.bootstrap_start, self.bootstrap_end) {
            (Some(start), Some(end)) => Some((end - start).to_std().unwrap_or_default()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_state_display() {
        assert_eq!(ComponentState::Uninitialized.to_string(), "Uninitialized");
        assert_eq!(ComponentState::Initializing.to_string(), "Initializing");
        assert_eq!(ComponentState::Healthy.to_string(), "Healthy");
    }

    #[test]
    fn test_component_info_creation() {
        let info = ComponentInfo::new("workspace");
        assert_eq!(info.name, "workspace");
        assert_eq!(info.state, ComponentState::Uninitialized);
        assert!(info.error_message.is_none());
    }

    #[test]
    fn test_component_info_transitions() {
        let info = ComponentInfo::new("buddy")
            .mark_initializing()
            .mark_initialized()
            .mark_healthy();

        assert_eq!(info.state, ComponentState::Healthy);
        assert!(info.initialized_at.is_some());
        assert!(info.last_health_check.is_some());
    }

    #[test]
    fn test_component_error_marking() {
        let info = ComponentInfo::new("omnibot").mark_failed("startup timeout");
        assert_eq!(info.state, ComponentState::Failed);
        assert_eq!(info.error_message, Some("startup timeout".to_string()));
    }

    #[test]
    fn test_ecosystem_state_creation() {
        let state = EcosystemState::new();
        assert!(state.components.is_empty());
        assert!(!state.is_healthy);
    }

    #[test]
    fn test_ecosystem_add_components() {
        let mut state = EcosystemState::new();
        state.add_component("workspace");
        state.add_component("buddy");
        state.add_component("omnibot");

        assert_eq!(state.components.len(), 3);
        assert!(state.components.contains_key("workspace"));
    }

    #[test]
    fn test_ecosystem_health_calculation() {
        let mut state = EcosystemState::new();
        state.add_component("workspace");
        state.add_component("buddy");

        {
            let buddy = state.get_or_create_component("buddy");
            buddy.state = ComponentState::Failed;
        }

        state.recalculate_health();
        assert!(!state.is_healthy);
        assert_eq!(state.failed_components, 1);
    }

    #[test]
    fn test_ecosystem_bootstrap_timing() {
        let mut state = EcosystemState::new();
        state.start_bootstrap();
        state.end_bootstrap();

        assert!(state.bootstrap_start.is_some());
        assert!(state.bootstrap_end.is_some());
        assert!(state.bootstrap_duration().is_some());
    }

    #[test]
    fn test_ecosystem_memory_tracking() {
        let mut state = EcosystemState::new();
        state.add_component("workspace");
        state.add_component("buddy");

        {
            let workspace = state.get_or_create_component("workspace");
            workspace.memory_mb = 512;
        }

        {
            let buddy = state.get_or_create_component("buddy");
            buddy.memory_mb = 256;
        }

        state.update_total_memory();
        assert_eq!(state.total_memory_mb, 768);
    }
}
