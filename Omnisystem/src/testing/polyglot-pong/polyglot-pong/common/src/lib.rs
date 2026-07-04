//! Polyglot Pong – Common Types & Canonical Specification
//!
//! Shared data structures for orchestrator, sandboxes, and analytics.

pub mod spec;

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Unique test run identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct TestId(pub uuid::Uuid);

/// Programming language name.
pub type Language = String;

/// A job assigned to a sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub job_id: TestId,
    pub source_lang: Language,
    pub target_lang: Language,
    pub conversion_round: u32,
    pub canonical_spec: spec::CanonicalSpec,
    pub random_seed: u64,
}

/// Integer-based game state (16.16 fixed-point, deterministic across all languages).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GameState {
    pub ball_x: i32,      // 0..65536 (0.0..1.0 normalized)
    pub ball_y: i32,      // 0..65536
    pub ball_dx: i32,     // delta per frame
    pub ball_dy: i32,
    pub paddle1_y: i32,   // 0..65536
    pub paddle2_y: i32,
    pub score1: u8,
    pub score2: u8,
}

/// Energy measurement (joules).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnergyMetrics {
    pub package_joules: f64,
    pub core_joules: f64,
    pub dram_joules: f64,
    pub total_joules: f64,
}

/// Runtime execution metrics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeMetrics {
    pub exec_time_us: u64,
    pub memory_peak_bytes: u64,
    pub binary_size_bytes: u64,
    pub energy: EnergyMetrics,
}

/// ZK-STARK proof of behavioural equivalence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkProof {
    pub proof_bytes: Vec<u8>,
    pub public_inputs: Vec<u8>,
    pub hash_algorithm: String, // "Blake3", "Poseidon"
}

/// TEE attestation quote (SGX/TDX/CCA).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeeAttestation {
    pub platform: String, // "SGX", "TDX", "CCA"
    pub quote: Vec<u8>,
    pub mr_enclave: [u8; 32],
    pub mr_signer: [u8; 32],
    pub report_data: Vec<u8>, // hash of execution trace
}

/// Result of a test run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub job_id: TestId,
    pub success: bool,
    pub trace: Vec<GameState>,  // One state per frame
    pub generated_source: Option<String>,
    pub metrics: RuntimeMetrics,

    // Optional enhancements (feature-gated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zk_proof: Option<ZkProof>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tee_attestation: Option<TeeAttestation>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Bug report automatically generated from test divergence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugReport {
    pub bug_id: String,
    pub source_lang: Language,
    pub target_lang: Language,
    pub job_id: TestId,
    pub failure_type: FailureType,
    pub minimized_source: String,
    pub compiler_version: String,
    pub universe_hash: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Classification of failure detected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailureType {
    CompilationError,
    RuntimeCrash,
    BehaviouralDivergence,
    PerformanceAnomaly,
    EnergyAnomaly,
}

/// Language compatibility edge in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityEdge {
    pub source: Language,
    pub target: Language,
    pub fidelity: f32,                  // 0.0..1.0
    pub conversion_difficulty: u32,     // 0..100
    pub bridge_centrality: f32,         // 0.0..1.0
}

/// Language clustering result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageCluster {
    pub name: String,
    pub members: Vec<Language>,
    pub centroid_language: Language,
    pub avg_internal_fidelity: f32,
}

/// Metrics aggregation for a test run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateMetrics {
    pub total_tests: usize,
    pub successful_tests: usize,
    pub avg_fidelity: f32,
    pub avg_exec_time_us: u64,
    pub avg_energy_joules: f64,
    pub highest_energy_lang: (Language, f64),
    pub lowest_energy_lang: (Language, f64),
    pub bugs_discovered: usize,
}

impl TestResult {
    pub fn new(job_id: TestId, success: bool) -> Self {
        Self {
            job_id,
            success,
            trace: Vec::new(),
            generated_source: None,
            metrics: RuntimeMetrics::default(),
            zk_proof: None,
            tee_attestation: None,
            error_message: None,
        }
    }

    /// Compute behavioral fidelity between this result and a reference.
    pub fn fidelity(&self, reference: &TestResult) -> f32 {
        if self.trace.is_empty() || reference.trace.is_empty() {
            return 0.0;
        }
        let min_len = self.trace.len().min(reference.trace.len());
        let mut matches = 0;
        for i in 0..min_len {
            if self.trace[i] == reference.trace[i] {
                matches += 1;
            }
        }
        matches as f32 / min_len as f32
    }
}

impl BugReport {
    pub fn new(
        source_lang: Language,
        target_lang: Language,
        job_id: TestId,
        failure_type: FailureType,
        minimized_source: String,
        compiler_version: String,
    ) -> Self {
        Self {
            bug_id: uuid::Uuid::new_v4().to_string(),
            source_lang,
            target_lang,
            job_id,
            failure_type,
            minimized_source,
            compiler_version,
            universe_hash: String::new(),
            timestamp: chrono::Utc::now(),
        }
    }
}
