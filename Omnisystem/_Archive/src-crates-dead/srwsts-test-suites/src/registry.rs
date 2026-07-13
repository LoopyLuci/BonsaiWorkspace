//! Test Suite Registry
//!
//! Centralized registry for discovering and loading test suites.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use dashmap::DashMap;
use tracing::debug;

/// Unique identifier for a test suite
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SuiteId {
    Kernel,
    Service,
    Language,
    Application,
    HardwareEquivalence,
    FullStack,
}

impl std::fmt::Display for SuiteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Kernel => write!(f, "kernel"),
            Self::Service => write!(f, "service"),
            Self::Language => write!(f, "language"),
            Self::Application => write!(f, "application"),
            Self::HardwareEquivalence => write!(f, "hardware-equivalence"),
            Self::FullStack => write!(f, "fullstack"),
        }
    }
}

/// Metadata about a test suite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteMetadata {
    pub id: SuiteId,
    pub name: String,
    pub description: String,
    pub version: String,
    pub test_count: usize,
    pub estimated_duration_minutes: u32,
}

impl SuiteMetadata {
    /// Create metadata for the Kernel suite
    pub fn kernel() -> Self {
        Self {
            id: SuiteId::Kernel,
            name: "Kernel Test Suite".to_string(),
            description: "Comprehensive tests for scheduler, memory, IPC, and drivers".to_string(),
            version: "0.1.0".to_string(),
            test_count: 12,
            estimated_duration_minutes: 15,
        }
    }

    /// Create metadata for the Service suite
    pub fn service() -> Self {
        Self {
            id: SuiteId::Service,
            name: "Service Test Suite".to_string(),
            description: "Tests for P2P, storage, network, and compositor services".to_string(),
            version: "0.1.0".to_string(),
            test_count: 12,
            estimated_duration_minutes: 20,
        }
    }

    /// Create metadata for the Language suite
    pub fn language() -> Self {
        Self {
            id: SuiteId::Language,
            name: "Language Test Suite".to_string(),
            description: "Tests for Titan, Sylva, Aether, and Axiom languages".to_string(),
            version: "0.1.0".to_string(),
            test_count: 12,
            estimated_duration_minutes: 45,
        }
    }

    /// Create metadata for the Application suite
    pub fn application() -> Self {
        Self {
            id: SuiteId::Application,
            name: "Application Test Suite".to_string(),
            description: "Tests for Workspace, Buddy, and Omni-Bot applications".to_string(),
            version: "0.1.0".to_string(),
            test_count: 9,
            estimated_duration_minutes: 30,
        }
    }

    /// Create metadata for the Hardware Equivalence suite
    pub fn hardware_equivalence() -> Self {
        Self {
            id: SuiteId::HardwareEquivalence,
            name: "Hardware Equivalence Test Suite".to_string(),
            description: "Tests for x86_64, ARM, and RISC-V hardware equivalence".to_string(),
            version: "0.1.0".to_string(),
            test_count: 15,
            estimated_duration_minutes: 60,
        }
    }

    /// Create metadata for the FullStack suite
    pub fn fullstack() -> Self {
        Self {
            id: SuiteId::FullStack,
            name: "Full Stack Test Suite".to_string(),
            description: "End-to-end integrated tests and chaos engineering scenarios".to_string(),
            version: "0.1.0".to_string(),
            test_count: 14,
            estimated_duration_minutes: 120,
        }
    }
}

/// Central registry for test suites
pub struct TestSuiteRegistry {
    suites: Arc<DashMap<String, SuiteMetadata>>,
}

impl TestSuiteRegistry {
    /// Create a new test suite registry with all available suites
    pub fn new() -> Self {
        let suites = Arc::new(DashMap::new());

        // Register all default suites
        let default_suites = vec![
            SuiteMetadata::kernel(),
            SuiteMetadata::service(),
            SuiteMetadata::language(),
            SuiteMetadata::application(),
            SuiteMetadata::hardware_equivalence(),
            SuiteMetadata::fullstack(),
        ];

        for suite in default_suites {
            let key = suite.id.to_string();
            suites.insert(key, suite);
        }

        Self { suites }
    }

    /// Get metadata for a specific suite
    pub fn get_suite(&self, id: &SuiteId) -> Option<SuiteMetadata> {
        let key = id.to_string();
        self.suites.get(&key).map(|r| r.value().clone())
    }

    /// Get all registered suites
    pub fn get_all_suites(&self) -> Vec<SuiteMetadata> {
        self.suites
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// List all available suite IDs
    pub fn list_suite_ids(&self) -> Vec<SuiteId> {
        self.suites
            .iter()
            .filter_map(|entry| {
                let metadata = entry.value();
                Some(metadata.id.clone())
            })
            .collect()
    }

    /// Get total test count across all suites
    pub fn total_test_count(&self) -> usize {
        self.suites.iter().map(|entry| entry.value().test_count).sum()
    }

    /// Get total estimated duration (minutes) across all suites
    pub fn total_estimated_duration_minutes(&self) -> u32 {
        self.suites
            .iter()
            .map(|entry| entry.value().estimated_duration_minutes)
            .sum()
    }

    /// Register a custom suite
    pub fn register_suite(&self, metadata: SuiteMetadata) {
        debug!("Registering test suite: {}", metadata.id);
        let key = metadata.id.to_string();
        self.suites.insert(key, metadata);
    }

    /// Unregister a suite
    pub fn unregister_suite(&self, id: &SuiteId) -> Option<SuiteMetadata> {
        let key = id.to_string();
        self.suites.remove(&key).map(|(_, v)| v)
    }
}

impl Default for TestSuiteRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suite_metadata_creation() {
        let meta = SuiteMetadata::kernel();
        assert_eq!(meta.id, SuiteId::Kernel);
        assert_eq!(meta.test_count, 12);
    }

    #[test]
    fn test_registry_initialization() {
        let registry = TestSuiteRegistry::new();
        assert_eq!(registry.get_all_suites().len(), 6);
    }

    #[test]
    fn test_registry_get_suite() {
        let registry = TestSuiteRegistry::new();
        let suite = registry.get_suite(&SuiteId::Service);
        assert!(suite.is_some());
        assert_eq!(suite.unwrap().test_count, 12);
    }

    #[test]
    fn test_registry_list_ids() {
        let registry = TestSuiteRegistry::new();
        let ids = registry.list_suite_ids();
        assert_eq!(ids.len(), 6);
        assert!(ids.contains(&SuiteId::Kernel));
        assert!(ids.contains(&SuiteId::FullStack));
    }

    #[test]
    fn test_registry_total_counts() {
        let registry = TestSuiteRegistry::new();
        let total_tests = registry.total_test_count();
        assert!(total_tests > 0);

        let total_minutes = registry.total_estimated_duration_minutes();
        assert!(total_minutes > 0);
    }

    #[test]
    fn test_registry_custom_suite() {
        let registry = TestSuiteRegistry::new();
        let custom = SuiteMetadata {
            id: SuiteId::Kernel,
            name: "Custom Suite".to_string(),
            description: "A custom test suite".to_string(),
            version: "1.0.0".to_string(),
            test_count: 5,
            estimated_duration_minutes: 10,
        };

        registry.register_suite(custom.clone());
        let retrieved = registry.get_suite(&SuiteId::Kernel);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Custom Suite");
    }
}
