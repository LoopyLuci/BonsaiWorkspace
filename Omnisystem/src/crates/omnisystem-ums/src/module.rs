// Module trait and types for Universal Module System

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for a module
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleId(Uuid);

impl ModuleId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_name(name: &str) -> Self {
        Self(Uuid::new_v5(&Uuid::NAMESPACE_DNS, name.as_bytes()))
    }
}

impl Default for ModuleId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Module state lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleState {
    /// Module registered but not loaded
    Registered,
    /// Module loaded but not initialized
    Loaded,
    /// Module initialized and ready to use
    Ready,
    /// Module executing
    Running,
    /// Module paused
    Paused,
    /// Module shutting down
    Shutting,
    /// Module shut down
    Stopped,
    /// Module encountered error
    Error,
}

/// Metadata for a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    /// Unique identifier
    pub id: ModuleId,

    /// Human-readable name
    pub name: String,

    /// Version (semantic versioning)
    pub version: String,

    /// Description
    pub description: String,

    /// Author
    pub author: String,

    /// Module dependencies (other module names)
    pub dependencies: Vec<String>,

    /// Module capabilities (what this module provides)
    pub capabilities: Vec<String>,

    /// When module was created
    pub created_at: DateTime<Utc>,

    /// When module was last updated
    pub updated_at: DateTime<Utc>,

    /// Module interface version (for compatibility)
    pub interface_version: String,

    /// Module phase (1-13 for Omnisystem)
    pub phase: u32,

    /// Location of source code (UMD path)
    pub source_path: String,

    /// Location of canonical implementation (Sylva)
    pub canonical_path: String,

    /// Formal specification (Axiom)
    pub spec_path: String,

    /// Metadata (arbitrary key-value)
    pub metadata: HashMap<String, String>,
}

/// Configuration passed to module initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfig {
    /// Module-specific configuration
    pub config: serde_json::Value,

    /// Parent runtime configuration
    pub runtime_config: serde_json::Value,

    /// Data directories
    pub data_dirs: ModuleDataDirs,
}

/// Data directories for module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDataDirs {
    /// UMD source folder (read-only)
    pub umd_source: std::path::PathBuf,

    /// Generated code folder
    pub generated: std::path::PathBuf,

    /// User data folder
    pub user_data: std::path::PathBuf,

    /// Cache folder
    pub cache: std::path::PathBuf,
}

/// Core module trait - everything in Omnisystem is a module
#[async_trait]
pub trait Module: Send + Sync {
    /// Get module information
    fn info(&self) -> &ModuleInfo;

    /// Initialize module with configuration
    /// Called once when module is loaded
    async fn initialize(&mut self, config: ModuleConfig) -> anyhow::Result<()>;

    /// Start module execution
    async fn start(&mut self) -> anyhow::Result<()>;

    /// Stop module execution
    async fn stop(&mut self) -> anyhow::Result<()>;

    /// Execute a request in the module
    /// This is the main entry point for module operations
    async fn execute(&self, request: ModuleRequest) -> anyhow::Result<ModuleResponse>;

    /// Get module state
    fn state(&self) -> ModuleState;

    /// Verify module correctness against formal specification
    async fn verify(&self) -> anyhow::Result<VerificationResult>;

    /// Get module metrics
    fn metrics(&self) -> ModuleMetrics;
}

/// Request sent to a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleRequest {
    /// Request ID for tracing
    pub request_id: String,

    /// Operation to perform
    pub operation: String,

    /// Arguments (operation-specific)
    pub args: serde_json::Value,

    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Response from a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleResponse {
    /// Request ID being responded to
    pub request_id: String,

    /// Success or error
    pub success: bool,

    /// Result data
    pub data: serde_json::Value,

    /// Error message if failed
    pub error: Option<String>,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

/// Result of module verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Overall verification passed
    pub passed: bool,

    /// Checks performed
    pub checks: Vec<VerificationCheck>,

    /// Errors found
    pub errors: Vec<String>,
}

/// Individual verification check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCheck {
    /// Check name
    pub name: String,

    /// Check passed
    pub passed: bool,

    /// Details
    pub details: String,
}

/// Module runtime metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetrics {
    /// Total requests processed
    pub requests_total: u64,

    /// Currently executing requests
    pub requests_active: u32,

    /// Average request latency in milliseconds
    pub latency_avg_ms: f64,

    /// P99 latency in milliseconds
    pub latency_p99_ms: f64,

    /// Memory usage in bytes
    pub memory_bytes: u64,

    /// Last execution time
    pub last_execution: Option<DateTime<Utc>>,

    /// Error count
    pub errors: u64,
}

impl Default for ModuleMetrics {
    fn default() -> Self {
        Self {
            requests_total: 0,
            requests_active: 0,
            latency_avg_ms: 0.0,
            latency_p99_ms: 0.0,
            memory_bytes: 0,
            last_execution: None,
            errors: 0,
        }
    }
}

/// Global module registry for automatic registration
/// Use #[inventory::submit] to automatically register module factories
pub struct ModuleFactory {
    pub name: String,
    pub create: fn() -> Box<dyn Module>,
}

inventory::collect!(ModuleFactory);
