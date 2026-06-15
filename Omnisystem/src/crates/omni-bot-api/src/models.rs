//! API request/response models for Environments and Modules

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// ENVIRONMENT MODELS
// ============================================================================

/// Environment specification for creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSpec {
    /// Unique identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Environment type (e.g., "container", "sandbox", "vm")
    pub env_type: String,
    /// Resource allocation
    pub resources: ResourceAllocation,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Initial modules to install
    pub modules: Option<Vec<ModuleRef>>,
    /// Snapshot to restore from
    pub snapshot_id: Option<String>,
    /// Custom metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    /// Tags for filtering
    pub tags: Option<Vec<String>>,
}

/// Resource allocation specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    /// Memory in MB
    pub memory_mb: u32,
    /// CPU cores
    pub cpu_cores: u32,
    /// Maximum CPU percentage
    pub cpu_percent_max: u32,
    /// Disk space in MB
    pub disk_mb: u32,
    /// IOPS limit
    pub iops_limit: u32,
    /// Network bandwidth in Mbps
    pub bandwidth_mbps: u32,
}

impl Default for ResourceAllocation {
    fn default() -> Self {
        Self {
            memory_mb: 512,
            cpu_cores: 2,
            cpu_percent_max: 100,
            disk_mb: 1024,
            iops_limit: 1000,
            bandwidth_mbps: 100,
        }
    }
}

/// Module reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleRef {
    /// Module name
    pub name: String,
    /// Optional version (defaults to latest)
    pub version: Option<String>,
}

/// Full environment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    /// Unique identifier
    pub id: String,
    /// Name
    pub name: String,
    /// Type
    pub env_type: String,
    /// Current state
    pub state: EnvironmentState,
    /// Resource allocation
    pub resources: ResourceAllocation,
    /// Current resource usage
    pub resource_usage: ResourceUsage,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Installed modules
    pub modules: Vec<InstalledModule>,
    /// Available snapshots
    pub snapshots: Vec<SnapshotInfo>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modified timestamp
    pub updated_at: DateTime<Utc>,
    /// Last started timestamp
    pub last_started: Option<DateTime<Utc>>,
    /// Tags
    pub tags: Option<Vec<String>>,
    /// Custom metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Environment state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnvironmentState {
    #[serde(rename = "created")]
    Created,
    #[serde(rename = "starting")]
    Starting,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "paused")]
    Paused,
    #[serde(rename = "pausing")]
    Pausing,
    #[serde(rename = "stopping")]
    Stopping,
    #[serde(rename = "stopped")]
    Stopped,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "migrating")]
    Migrating,
}

impl EnvironmentState {
    pub fn is_running(&self) -> bool {
        *self == EnvironmentState::Running
    }

    pub fn can_start(&self) -> bool {
        matches!(self, EnvironmentState::Created | EnvironmentState::Stopped)
    }

    pub fn can_stop(&self) -> bool {
        matches!(
            self,
            EnvironmentState::Running
                | EnvironmentState::Starting
                | EnvironmentState::Paused
        )
    }
}

/// Resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage percentage
    pub cpu_percent: f32,
    /// Memory usage in MB
    pub memory_mb: u32,
    /// Disk usage in MB
    pub disk_mb: u32,
    /// Network bandwidth in Mbps
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

/// Installed module information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledModule {
    /// Module name
    pub name: String,
    /// Installed version
    pub version: String,
    /// Installation status
    pub status: ModuleStatus,
    /// Installation timestamp
    pub installed_at: DateTime<Utc>,
}

/// Module status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleStatus {
    #[serde(rename = "installed")]
    Installed,
    #[serde(rename = "initializing")]
    Initializing,
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "updating")]
    Updating,
}

/// Environment status snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentStatus {
    /// Environment ID
    pub id: String,
    /// Current state
    pub state: EnvironmentState,
    /// Resource usage
    pub resource_usage: ResourceUsage,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Health check timestamp
    pub last_health_check: DateTime<Utc>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Snapshot information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    /// Snapshot ID
    pub id: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Size in MB
    pub size_mb: u32,
    /// Snapshot description
    pub description: Option<String>,
    /// CAS hash for integrity
    pub cas_hash: String,
}

/// Snapshot restore request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreSnapshotRequest {
    /// Snapshot ID to restore
    pub snapshot_id: String,
}

/// Command execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteCommandRequest {
    /// Command to execute
    pub command: String,
    /// Command arguments
    pub args: Option<Vec<String>>,
    /// Working directory
    pub working_dir: Option<String>,
    /// Environment variables for execution
    pub env: Option<HashMap<String, String>>,
    /// Timeout in seconds
    pub timeout_seconds: Option<u32>,
}

/// Command execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecutionResult {
    /// Exit code
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution duration in ms
    pub duration_ms: u64,
}

/// Migration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRequest {
    /// Destination host/region
    pub destination: String,
    /// Whether to keep source after migration
    pub keep_source: Option<bool>,
    /// Custom options
    pub options: Option<HashMap<String, String>>,
}

// ============================================================================
// MODULE (UMS) MODELS
// ============================================================================

/// Module search/filter request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSearchRequest {
    /// Search query (name/description)
    pub query: Option<String>,
    /// Filter by tags
    pub tags: Option<Vec<String>>,
    /// Filter by author
    pub author: Option<String>,
    /// Minimum version
    pub min_version: Option<String>,
    /// Maximum version
    pub max_version: Option<String>,
    /// Page number (0-indexed)
    pub page: Option<u32>,
    /// Results per page
    pub per_page: Option<u32>,
    /// Sort by field (name, version, created_at, updated_at)
    pub sort_by: Option<String>,
    /// Sort order (asc, desc)
    pub sort_order: Option<String>,
}

/// Module installation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInstallRequest {
    /// Module name
    pub name: String,
    /// Version to install
    pub version: Option<String>,
    /// Force reinstall if exists
    pub force: Option<bool>,
    /// Verify signature before install
    pub verify_signature: Option<bool>,
    /// Custom options
    pub options: Option<HashMap<String, String>>,
}

/// Module update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleUpdateRequest {
    /// Target version
    pub target_version: Option<String>,
    /// Force update
    pub force: Option<bool>,
    /// Custom options
    pub options: Option<HashMap<String, String>>,
}

/// Module signature verification request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleVerifyRequest {
    /// Module name
    pub name: String,
    /// Module version
    pub version: String,
    /// Module signature (base64)
    pub signature: String,
    /// Signing key ID
    pub key_id: String,
}

/// Module information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    /// Module name
    pub name: String,
    /// Version
    pub version: String,
    /// Description
    pub description: Option<String>,
    /// Module type (e.g., "core", "extension", "utility")
    pub module_type: String,
    /// Author
    pub author: String,
    /// License
    pub license: String,
    /// Homepage URL
    pub homepage: Option<String>,
    /// Repository URL
    pub repository: Option<String>,
    /// Dependencies
    pub dependencies: Vec<ModuleDependency>,
    /// Supported environments
    pub supported_environments: Vec<String>,
    /// Module tags
    pub tags: Vec<String>,
    /// Release date
    pub released_at: DateTime<Utc>,
    /// Last updated
    pub updated_at: DateTime<Utc>,
    /// Download count
    pub download_count: u64,
    /// Checksum (CAS hash)
    pub checksum: String,
    /// Digital signature metadata
    pub signature: SignatureMetadata,
    /// Available versions
    pub available_versions: Vec<String>,
}

/// Module dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDependency {
    /// Dependency module name
    pub name: String,
    /// Required version range
    pub version_range: String,
    /// Is optional
    pub optional: bool,
}

/// Signature metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureMetadata {
    /// Signing key ID
    pub key_id: String,
    /// Signature value (base64)
    pub signature: String,
    /// Signature algorithm
    pub algorithm: String,
    /// Signed timestamp
    pub signed_at: DateTime<Utc>,
}

/// Module verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Is signature valid
    pub valid: bool,
    /// Verification message
    pub message: String,
    /// Key ID used
    pub key_id: String,
    /// Verification timestamp
    pub verified_at: DateTime<Utc>,
}

/// Module operation progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationProgress {
    /// Operation ID
    pub operation_id: String,
    /// Operation type (install, update, remove, verify)
    pub operation_type: String,
    /// Current status
    pub status: ProgressStatus,
    /// Percentage complete (0-100)
    pub progress_percent: u32,
    /// Current step description
    pub current_step: String,
    /// Total steps
    pub total_steps: u32,
    /// Estimated time remaining in seconds
    pub eta_seconds: Option<u32>,
    /// Detailed messages
    pub messages: Vec<ProgressMessage>,
}

/// Progress status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgressStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "in_progress")]
    InProgress,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
}

/// Progress message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressMessage {
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Message level (info, warn, error)
    pub level: String,
    /// Message content
    pub message: String,
}

// ============================================================================
// VALIDATION (UVM) MODELS
// ============================================================================

/// Unique validation run identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValidationRunId(pub uuid::Uuid);

impl ValidationRunId {
    pub fn new() -> Self {
        ValidationRunId(Uuid::new_v4())
    }
}

impl Default for ValidationRunId {
    fn default() -> Self {
        Self::new()
    }
}

/// Matrix axis configuration for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixAxis {
    pub name: String,
    pub values: Vec<String>,
}

/// Matrix configuration for test execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixConfig {
    pub axes: Vec<MatrixAxis>,
    pub total_combinations: usize,
}

/// Parallelism settings for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelismSettings {
    pub max_parallel_tests: u32,
    pub worker_pool_size: u32,
    pub queue_depth: u32,
}

impl Default for ParallelismSettings {
    fn default() -> Self {
        Self {
            max_parallel_tests: 8,
            worker_pool_size: 4,
            queue_depth: 64,
        }
    }
}

/// Timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    pub per_test_secs: u64,
    pub total_run_secs: u64,
    pub warmup_secs: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            per_test_secs: 300,
            total_run_secs: 3600,
            warmup_secs: 60,
        }
    }
}

/// Request to run validation suite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRunRequest {
    pub name: String,
    pub description: Option<String>,
    pub matrix: MatrixConfig,
    pub parallelism: Option<ParallelismSettings>,
    pub timeout: Option<TimeoutConfig>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Individual test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
    pub error: Option<String>,
    pub metrics: TestMetrics,
}

/// Test execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Timeout,
    Error,
}

/// Metrics collected during test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestMetrics {
    pub memory_peak_mb: u32,
    pub cpu_avg_percent: u32,
    pub iops: u32,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl Default for TestMetrics {
    fn default() -> Self {
        Self {
            memory_peak_mb: 0,
            cpu_avg_percent: 0,
            iops: 0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }
}

/// Validation run status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ValidationStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Complete validation run results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResults {
    pub run_id: ValidationRunId,
    pub name: String,
    pub status: ValidationStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub timeout: usize,
    pub results: Vec<TestResult>,
    pub summary_metrics: ValidationMetrics,
}

/// Aggregated validation metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMetrics {
    pub total_duration_ms: u64,
    pub avg_test_duration_ms: u64,
    pub peak_memory_mb: u32,
    pub avg_cpu_percent: u32,
    pub success_rate_percent: f64,
}

/// Heatmap data for visual results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapData {
    pub run_id: ValidationRunId,
    pub axes: Vec<String>,
    pub cells: Vec<HeatmapCell>,
    pub legend: HeatmapLegend,
}

/// Individual heatmap cell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapCell {
    pub coordinates: Vec<usize>,
    pub value: f64,
    pub status: TestStatus,
    pub label: String,
}

/// Heatmap legend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapLegend {
    pub min_value: f64,
    pub max_value: f64,
    pub color_scale: String,
}

/// Request to replay validation run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReplayRequest {
    pub original_run_id: ValidationRunId,
    pub specific_tests: Option<Vec<String>>,
}

/// Execution trace event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub details: HashMap<String, serde_json::Value>,
}

/// Execution trace for a validation run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub run_id: ValidationRunId,
    pub events: Vec<TraceEvent>,
    pub total_events: usize,
}

/// Historical validation run summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationHistoryEntry {
    pub run_id: ValidationRunId,
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub status: ValidationStatus,
    pub passed: usize,
    pub failed: usize,
    pub total_tests: usize,
    pub duration_ms: u64,
}

// ============================================================================
// DRIVER CONVERTER MODELS
// ============================================================================

/// Unique conversion job identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConversionJobId(pub Uuid);

impl ConversionJobId {
    pub fn new() -> Self {
        ConversionJobId(Uuid::new_v4())
    }
}

impl Default for ConversionJobId {
    fn default() -> Self {
        Self::new()
    }
}

/// Target platform for driver conversion
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetPlatform {
    Linux,
    Windows,
    Macos,
    Generic,
}

/// Optimization flags for conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationFlags {
    pub enable_lto: bool,
    pub codegen_units: u32,
    pub vectorization: bool,
    pub inline_threshold: u32,
}

impl Default for OptimizationFlags {
    fn default() -> Self {
        Self {
            enable_lto: true,
            codegen_units: 16,
            vectorization: true,
            inline_threshold: 100,
        }
    }
}

/// Request to convert DIS to driver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverConversionRequest {
    pub dis_content: String,
    pub dis_name: String,
    pub target_platform: TargetPlatform,
    pub optimization: Option<OptimizationFlags>,
    pub background: Option<bool>,
}

/// Driver conversion job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ConversionStatus {
    Queued,
    Converting,
    Compiling,
    Optimizing,
    Completed,
    Failed,
    Cancelled,
}

/// Driver conversion results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionResult {
    pub job_id: ConversionJobId,
    pub dis_name: String,
    pub target_platform: TargetPlatform,
    pub status: ConversionStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub driver_binary: Option<Vec<u8>>,
    pub driver_checksum: Option<String>,
    pub compilation_log: Option<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// Request to install converted driver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverInstallRequest {
    pub version: String,
    pub auto_activate: Option<bool>,
    pub rollback_on_error: Option<bool>,
}

// ============================================================================
// HDE MANAGEMENT MODELS
// ============================================================================

/// AI model identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId(pub String);

/// Model deployment state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelState {
    Shadow,
    Active,
    Deprecated,
    Archived,
}

/// Safety envelope status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyEnvelope {
    pub max_context_length: usize,
    pub allowed_operations: Vec<String>,
    pub resource_limits: ResourceLimits,
    pub validation_required: bool,
}

/// Resource limits for model execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory_mb: u32,
    pub cpu_percent: u32,
    pub max_tokens: u32,
    pub timeout_secs: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_mb: 2048,
            cpu_percent: 50,
            max_tokens: 4096,
            timeout_secs: 300,
        }
    }
}

/// AI model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: ModelId,
    pub name: String,
    pub version: String,
    pub state: ModelState,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub metrics: ModelMetrics,
    pub safety_envelope: SafetyEnvelope,
}

/// Model performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetrics {
    pub accuracy: f64,
    pub latency_ms: f64,
    pub throughput_rps: f64,
    pub error_rate: f64,
    pub resource_efficiency: f64,
}

/// Request to promote model from shadow to active
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPromoteRequest {
    pub version: String,
    pub validation_passed: bool,
    pub rollout_percentage: Option<u8>,
}

/// Request to demote model from active to shadow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDemoteRequest {
    pub reason: String,
    pub preserve_shadow: Option<bool>,
}

/// Validation data for shadow model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowValidationReport {
    pub model_id: ModelId,
    pub model_version: String,
    pub validation_timestamp: DateTime<Utc>,
    pub tests_run: usize,
    pub tests_passed: usize,
    pub tests_failed: usize,
    pub safety_violations: Vec<SafetyViolation>,
    pub performance_deltas: PerformanceDeltas,
    pub ready_for_promotion: bool,
}

/// Safety violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyViolation {
    pub severity: ViolationSeverity,
    pub description: String,
    pub context: HashMap<String, serde_json::Value>,
}

/// Severity levels for violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViolationSeverity {
    Info,
    Warning,
    Critical,
}

/// Performance comparison between active and shadow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceDeltas {
    pub accuracy_delta: f64,
    pub latency_delta_ms: f64,
    pub throughput_delta_rps: f64,
    pub error_rate_delta: f64,
}

// ============================================================================
// REQUEST/RESPONSE WRAPPERS
// ============================================================================

/// List response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse<T> {
    /// Items in the list
    pub items: Vec<T>,
    /// Total count
    pub total: u32,
    /// Current page (0-indexed)
    pub page: u32,
    /// Items per page
    pub per_page: u32,
    /// Total pages
    pub total_pages: u32,
}

impl<T> ListResponse<T> {
    pub fn new(items: Vec<T>, total: u32, page: u32, per_page: u32) -> Self {
        let total_pages = (total + per_page - 1) / per_page;
        Self {
            items,
            total,
            page,
            per_page,
            total_pages,
        }
    }
}

/// Async operation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncOperationResponse {
    /// Operation task ID
    pub task_id: String,
    /// Operation status
    pub status: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// WebSocket URL for progress updates (optional)
    pub progress_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_state() {
        assert!(EnvironmentState::Running.is_running());
        assert!(!EnvironmentState::Stopped.is_running());
        assert!(EnvironmentState::Created.can_start());
        assert!(EnvironmentState::Running.can_stop());
    }

    #[test]
    fn test_resource_allocation_default() {
        let res = ResourceAllocation::default();
        assert_eq!(res.memory_mb, 512);
        assert_eq!(res.cpu_cores, 2);
    }

    #[test]
    fn test_module_dependency() {
        let dep = ModuleDependency {
            name: "core".to_string(),
            version_range: ">=1.0.0".to_string(),
            optional: false,
        };
        assert_eq!(dep.name, "core");
    }

    #[test]
    fn test_list_response() {
        let items = vec![1, 2, 3];
        let resp = ListResponse::new(items, 10, 0, 3);
        assert_eq!(resp.total_pages, 4);
        assert_eq!(resp.page, 0);
    }

    // Validation tests
    #[test]
    fn test_validation_run_id_generation() {
        let id1 = ValidationRunId::new();
        let id2 = ValidationRunId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_parallelism_defaults() {
        let defaults = ParallelismSettings::default();
        assert_eq!(defaults.max_parallel_tests, 8);
        assert_eq!(defaults.worker_pool_size, 4);
    }

    #[test]
    fn test_timeout_defaults() {
        let defaults = TimeoutConfig::default();
        assert_eq!(defaults.per_test_secs, 300);
        assert_eq!(defaults.total_run_secs, 3600);
    }

    // Driver tests
    #[test]
    fn test_conversion_job_id_generation() {
        let id1 = ConversionJobId::new();
        let id2 = ConversionJobId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_optimization_flags_defaults() {
        let flags = OptimizationFlags::default();
        assert!(flags.enable_lto);
        assert!(flags.vectorization);
    }

    // HDE tests
    #[test]
    fn test_resource_limits_defaults() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.memory_mb, 2048);
        assert_eq!(limits.max_tokens, 4096);
    }

    #[test]
    fn test_test_metrics_defaults() {
        let metrics = TestMetrics::default();
        assert_eq!(metrics.memory_peak_mb, 0);
        assert_eq!(metrics.cache_hits, 0);
    }
}

// ============================================================================
// ASSET MANAGEMENT MODELS
// ============================================================================

/// Asset generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetGenerationRequest {
    pub asset_type: String,
    pub description: String,
    pub style: Option<String>,
    pub quality: Option<AssetQuality>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Asset quality levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AssetQuality {
    Low,
    Medium,
    High,
    Ultra,
}

impl AssetQuality {
    pub fn scale_factor(&self) -> f32 {
        match self {
            AssetQuality::Low => 0.5,
            AssetQuality::Medium => 1.0,
            AssetQuality::High => 1.5,
            AssetQuality::Ultra => 2.0,
        }
    }
}

/// Asset specification with full metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSpec {
    pub id: String,
    pub asset_type: String,
    pub description: String,
    pub style: Option<String>,
    pub quality: AssetQuality,
    pub size_bytes: u64,
    pub format: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u32>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Asset information with preview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub spec: AssetSpec,
    pub preview_url: String,
    pub download_url: String,
    pub published_to_ums: bool,
    pub ums_reference_id: Option<String>,
    pub tags: Vec<String>,
    pub checksum: String,
}

/// Asset list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetListResponse {
    pub assets: Vec<AssetInfo>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

/// Asset batch operation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchAssetOperation {
    pub operation: BatchOperation,
    pub asset_ids: Vec<String>,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Batch operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BatchOperation {
    Resize { width: u32, height: u32 },
    Convert { format: String },
    Optimize { compression: u8 },
    ApplyFilter { filter_name: String },
    Tag { tags: Vec<String> },
    Delete,
}

/// Batch operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationResult {
    pub operation_id: String,
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub results: Vec<AssetOperationResult>,
}

/// Single asset operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetOperationResult {
    pub asset_id: String,
    pub success: bool,
    pub message: String,
    pub result: Option<AssetInfo>,
}

/// Asset publish request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPublishRequest {
    pub visibility: AssetVisibility,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Asset visibility levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetVisibility {
    Private,
    Internal,
    Public,
}

/// Asset progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetProgress {
    pub asset_id: String,
    pub stage: AssetGenerationStage,
    pub progress_percent: u8,
    pub message: String,
    pub estimated_time_remaining_secs: Option<u32>,
    pub error: Option<String>,
}

/// Asset generation stages
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetGenerationStage {
    Queued,
    Validating,
    Generating,
    Processing,
    Publishing,
    Complete,
    Failed,
}

// ============================================================================
// WORKFLOW MANAGEMENT MODELS
// ============================================================================

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub category: String,
    pub tags: Vec<String>,
    pub dag: WorkflowDAG,
    pub parameters: Vec<WorkflowParameter>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Workflow DAG (Directed Acyclic Graph)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDAG {
    pub steps: Vec<WorkflowStep>,
    pub edges: Vec<WorkflowEdge>,
}

impl WorkflowDAG {
    /// Validate DAG structure (no cycles, all references valid)
    pub fn validate(&self) -> Result<(), String> {
        // Check all edge references are valid
        let step_ids: std::collections::HashSet<_> = self.steps.iter().map(|s| s.id.clone()).collect();

        for edge in &self.edges {
            if !step_ids.contains(&edge.from) {
                return Err(format!("Invalid edge source: {}", edge.from));
            }
            if !step_ids.contains(&edge.to) {
                return Err(format!("Invalid edge target: {}", edge.to));
            }
        }

        // Check for cycles using DFS
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();

        for step in &self.steps {
            if !visited.contains(&step.id) {
                if self.has_cycle(&step.id, &mut visited, &mut rec_stack) {
                    return Err("Workflow contains a cycle".to_string());
                }
            }
        }

        Ok(())
    }

    fn has_cycle(&self, node: &str, visited: &mut std::collections::HashSet<String>,
                 rec_stack: &mut std::collections::HashSet<String>) -> bool {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        for edge in &self.edges {
            if edge.from == node {
                if !visited.contains(&edge.to) {
                    if self.has_cycle(&edge.to, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(&edge.to) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }
}

/// Workflow step definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub action: String,
    pub description: Option<String>,
    pub step_type: StepType,
    pub config: HashMap<String, serde_json::Value>,
    pub timeout_secs: Option<u32>,
    pub retry_policy: Option<RetryPolicy>,
    pub on_failure: Option<FailureAction>,
}

/// Step execution type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepType {
    Sequential,
    Parallel,
    Conditional,
    Loop,
}

/// Retry policy for steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_delay_ms: u32,
    pub backoff_multiplier: f32,
}

/// Failure action
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureAction {
    Rollback,
    Continue,
    Halt,
}

/// Workflow DAG edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub from: String,
    pub to: String,
    pub condition: Option<String>,
}

/// Workflow parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowParameter {
    pub name: String,
    pub param_type: String,
    pub description: Option<String>,
    pub default_value: Option<serde_json::Value>,
    pub required: bool,
}

/// Workflow execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionRequest {
    pub parameters: HashMap<String, serde_json::Value>,
    pub timeout_secs: Option<u32>,
    pub tags: Option<Vec<String>>,
}

/// Workflow execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub execution_id: String,
    pub workflow_id: String,
    pub workflow_version: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub status: ExecutionStatus,
    pub current_step: Option<String>,
    pub completed_steps: Vec<String>,
    pub failed_steps: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
}

/// Execution status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    RolledBack,
    Cancelled,
}

/// Step execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub step_name: String,
    pub status: StepExecutionStatus,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub retries_used: u32,
}

/// Step execution status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// Complete execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub context: ExecutionContext,
    pub steps: Vec<StepResult>,
    pub final_output: Option<serde_json::Value>,
}

/// Workflow list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowListResponse {
    pub workflows: Vec<WorkflowDefinition>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

/// Workflow creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCreateRequest {
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub dag: WorkflowDAG,
    pub parameters: Vec<WorkflowParameter>,
}

/// Workflow execution update (WebSocket message)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionUpdate {
    pub execution_id: String,
    pub event_type: ExecutionEventType,
    pub step_result: Option<StepResult>,
    pub context: Option<ExecutionContext>,
    pub error: Option<String>,
}

/// Execution event types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionEventType {
    Started,
    StepStarted,
    StepCompleted,
    StepFailed,
    Completed,
    Failed,
    RolledBack,
}

// ============================================================================
// SERVICE MANAGEMENT MODELS (Phase 1 API)
// ============================================================================

/// Generic API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            request_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
        }
    }
}

/// Service request types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartServiceRequest {
    pub name: String,
    pub config: Option<serde_json::Value>,
    pub wait_for_ready: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopServiceRequest {
    pub name: String,
    pub graceful: Option<bool>,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartServiceRequest {
    pub name: String,
    pub graceful: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigureServiceRequest {
    pub name: String,
    pub config: serde_json::Value,
    pub merge: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotServiceRequest {
    pub name: String,
    pub snapshot_name: Option<String>,
    pub description: Option<String>,
}

/// Service response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceListResponse {
    pub services: Vec<ServiceSummary>,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSummary {
    pub name: String,
    pub version: String,
    pub state: String,
    pub status: String,
    pub uptime_seconds: u64,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDetailResponse {
    pub name: String,
    pub version: String,
    pub state: String,
    pub status: String,
    pub uptime_seconds: u64,
    pub pid: Option<u32>,
    pub cpu_percent: f32,
    pub memory_mb: u32,
    pub disk_mb: u32,
    pub bandwidth_mbps: f32,
    pub last_health_check: DateTime<Utc>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartServiceResponse {
    pub name: String,
    pub state: String,
    pub pid: Option<u32>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopServiceResponse {
    pub name: String,
    pub state: String,
    pub uptime_seconds: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartServiceResponse {
    pub name: String,
    pub state: String,
    pub old_pid: Option<u32>,
    pub new_pid: Option<u32>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigureServiceResponse {
    pub name: String,
    pub config: serde_json::Value,
    pub applied: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotResponse {
    pub name: String,
    pub snapshot_id: String,
    pub snapshot_name: String,
    pub timestamp: DateTime<Utc>,
    pub size_bytes: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsResponse {
    pub name: String,
    pub lines: Vec<LogLine>,
    pub total_lines: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
}
