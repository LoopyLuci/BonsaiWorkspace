//! Pre-defined chaos engineering scenarios.
//!
//! This module contains 40+ complete chaos scenarios covering real-world
//! failure modes: Black Friday, power grid failures, data center fires, etc.

pub mod scenarios_impl;
pub use scenarios_impl::*;

use crate::error::Result;
use crate::schedule_generator::FaultSchedule;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Scenario configuration and parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosScenario {
    /// Unique scenario name.
    pub name: String,
    /// Detailed description.
    pub description: String,
    /// Scenario category.
    pub category: ScenarioCategory,
    /// Expected impact on system (Low/Medium/High/Critical).
    pub impact_level: ImpactLevel,
    /// Estimated time to failure (seconds).
    pub time_to_failure_secs: u64,
    /// Estimated time to recovery (seconds).
    pub time_to_recovery_secs: u64,
    /// Pre-generated fault schedule.
    pub fault_schedule: FaultSchedule,
    /// Real-world incident this scenario is based on.
    pub real_world_incident: Option<String>,
    /// Keywords for searching/filtering scenarios.
    pub keywords: Vec<String>,
}

/// Scenario categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenarioCategory {
    /// Load and traffic scenarios.
    LoadTesting,
    /// Power and cooling failures.
    PowerCooling,
    /// Network failures.
    Network,
    /// Storage and data failures.
    Storage,
    /// Compute and CPU failures.
    Compute,
    /// Multi-component cascading failures.
    Cascading,
    /// Byzantine and consensus failures.
    Byzantine,
    /// Silent and subtle failures.
    Silent,
}

impl std::fmt::Display for ScenarioCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScenarioCategory::LoadTesting => write!(f, "LoadTesting"),
            ScenarioCategory::PowerCooling => write!(f, "PowerCooling"),
            ScenarioCategory::Network => write!(f, "Network"),
            ScenarioCategory::Storage => write!(f, "Storage"),
            ScenarioCategory::Compute => write!(f, "Compute"),
            ScenarioCategory::Cascading => write!(f, "Cascading"),
            ScenarioCategory::Byzantine => write!(f, "Byzantine"),
            ScenarioCategory::Silent => write!(f, "Silent"),
        }
    }
}

/// Impact level of scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ImpactLevel {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

impl std::fmt::Display for ImpactLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImpactLevel::Low => write!(f, "Low"),
            ImpactLevel::Medium => write!(f, "Medium"),
            ImpactLevel::High => write!(f, "High"),
            ImpactLevel::Critical => write!(f, "Critical"),
        }
    }
}

impl ChaosScenario {
    /// Create new scenario.
    pub fn new(
        name: String,
        description: String,
        category: ScenarioCategory,
        impact_level: ImpactLevel,
        time_to_failure_secs: u64,
        time_to_recovery_secs: u64,
        fault_schedule: FaultSchedule,
    ) -> Self {
        Self {
            name,
            description,
            category,
            impact_level,
            time_to_failure_secs,
            time_to_recovery_secs,
            fault_schedule,
            real_world_incident: None,
            keywords: Vec::new(),
        }
    }

    /// Add real-world incident reference.
    pub fn with_incident(mut self, incident: String) -> Self {
        self.real_world_incident = Some(incident);
        self
    }

    /// Add keywords.
    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = keywords;
        self
    }

    /// Get total scenario duration.
    pub fn total_duration(&self) -> u64 {
        self.time_to_failure_secs + self.time_to_recovery_secs
    }

    /// Validate scenario.
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(crate::error::ChaosError::ConfigurationError(
                "Scenario name cannot be empty".to_string(),
            ));
        }

        self.fault_schedule.validate()?;

        info!(
            "Scenario '{}' validated: {} faults, {} category, {} impact",
            self.name, self.fault_schedule.fault_count(), self.category, self.impact_level
        );

        Ok(())
    }
}

/// Scenario template for easy generation.
pub struct ScenarioTemplate {
    pub name: String,
    pub description: String,
    pub category: ScenarioCategory,
    pub impact_level: ImpactLevel,
    pub time_to_failure_secs: u64,
    pub time_to_recovery_secs: u64,
    pub fault_count: usize,
    pub real_world_incident: Option<String>,
    pub keywords: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_creation() {
        let schedule = FaultSchedule::new(1000, 3600, 42);
        let scenario = ChaosScenario::new(
            "Test".to_string(),
            "Test scenario".to_string(),
            ScenarioCategory::LoadTesting,
            ImpactLevel::High,
            300,
            600,
            schedule,
        );

        assert_eq!(scenario.total_duration(), 900);
    }

    #[test]
    fn test_impact_level_ordering() {
        assert!(ImpactLevel::Low < ImpactLevel::Medium);
        assert!(ImpactLevel::Medium < ImpactLevel::High);
        assert!(ImpactLevel::High < ImpactLevel::Critical);
    }
}
