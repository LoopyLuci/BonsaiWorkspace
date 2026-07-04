use crate::campaign::{FuzzStrategy, TargetKind};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Result of a single fuzzing execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionResult {
    Success { duration_ms: u64 },
    Crash    { error: String, backtrace: String, input_hash: String, duration_ms: u64 },
    Hang     { timeout_ms: u64 },
    Assertion { message: String, input_hash: String },
    Violation { capability: String, attempted_action: String },
}

/// A discovered failure ready for Survival KB import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureReport {
    pub id:              String,
    pub campaign_id:     String,
    pub target:          String,
    pub strategy:        String,
    pub error_pattern:   String,
    pub backtrace:       String,
    pub minimal_input:   serde_json::Value,
    pub input_hash:      String,
    pub reproduction_cmd: String,
    pub timestamp_ns:    u64,
    pub auto_fix_script: Option<String>,
}

impl FailureReport {
    pub fn fingerprint(&self) -> String {
        blake3::hash(
            format!("{}:{}", self.error_pattern, &self.backtrace[..self.backtrace.len().min(200)])
                .as_bytes()
        ).to_hex().to_string()
    }
}

/// Input mutator — generates and mutates test inputs.
pub struct Mutator {
    rng: StdRng,
    seed: u64,
    strategy: MutationMode,
}

#[derive(Debug, Clone)]
enum MutationMode {
    Random,
    Boundary,
    PathTraversal,
    TypeConfusion,
    LargeInput { size: usize },
}

impl Mutator {
    pub fn new(seed: u64) -> Self {
        Self { rng: StdRng::seed_from_u64(seed), seed, strategy: MutationMode::Random }
    }

    /// Generate the next test input as JSON.
    pub fn next_input(&mut self, iteration: u64) -> serde_json::Value {
        // Rotate through mutation modes
        self.strategy = match iteration % 6 {
            0 => MutationMode::Random,
            1 => MutationMode::Boundary,
            2 => MutationMode::PathTraversal,
            3 => MutationMode::TypeConfusion,
            4 => MutationMode::LargeInput { size: 1_000_000 },
            _ => MutationMode::Random,
        };

        match &self.strategy {
            MutationMode::Random => self.random_json(),
            MutationMode::Boundary => self.boundary_json(),
            MutationMode::PathTraversal => self.path_traversal_json(),
            MutationMode::TypeConfusion => self.type_confusion_json(),
            MutationMode::LargeInput { size } => serde_json::Value::String("A".repeat(*size)),
        }
    }

    fn random_json(&mut self) -> serde_json::Value {
        let n = self.rng.gen_range(0..6);
        match n {
            0 => serde_json::Value::Null,
            1 => serde_json::Value::Bool(self.rng.gen()),
            2 => serde_json::Value::Number(serde_json::Number::from(self.rng.gen::<i64>())),
            3 => { let len = self.rng.gen_range(0..4096); serde_json::Value::String(self.random_string(len)) }
            4 => serde_json::json!([]),
            _ => serde_json::json!({}),
        }
    }

    fn boundary_json(&mut self) -> serde_json::Value {
        let boundaries = [
            serde_json::json!(i64::MIN),
            serde_json::json!(i64::MAX),
            serde_json::json!(u64::MAX),
            serde_json::json!(0),
            serde_json::json!(-1),
            serde_json::Value::String(String::new()),
            serde_json::Value::String("\0".to_string()),
            serde_json::Value::String("\u{FEFF}".to_string()), // BOM
        ];
        boundaries[self.rng.gen_range(0..boundaries.len())].clone()
    }

    fn path_traversal_json(&mut self) -> serde_json::Value {
        let payloads = [
            "../../etc/passwd",
            "..\\..\\windows\\system32\\config\\sam",
            "/proc/self/environ",
            "~/.ssh/id_rsa",
            "/dev/null",
            "C:\\Windows\\System32",
            "\x00malicious",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ];
        let idx = self.rng.gen_range(0..payloads.len());
        serde_json::Value::String(payloads[idx].to_string())
    }

    fn type_confusion_json(&mut self) -> serde_json::Value {
        // Send unexpected types for fields that expect specific types
        let wrong_types = [
            serde_json::json!({"path": 42}),
            serde_json::json!({"path": null}),
            serde_json::json!({"path": [1,2,3]}),
            serde_json::json!({"content": {"nested": "object"}}),
            serde_json::json!({"enabled": "yes"}),  // expects bool
        ];
        wrong_types[self.rng.gen_range(0..wrong_types.len())].clone()
    }

    fn random_string(&mut self, len: usize) -> String {
        (0..len).map(|_| self.rng.gen::<char>()).collect()
    }

    pub fn feedback(&mut self, _coverage_delta: f64) {
        // In a coverage-guided fuzzer, this would adjust mutation weights
        // based on which mutations produced new coverage paths.
    }
}

/// Sandboxed worker that executes a single fuzzing task.
pub struct FuzzWorker {
    pub campaign_id: String,
    pub target:      TargetKind,
    pub strategy:    FuzzStrategy,
    mutator:         Mutator,
    failures:        Vec<FailureReport>,
    seen_fingerprints: std::collections::HashSet<String>,
}

impl FuzzWorker {
    pub fn new(campaign_id: String, target: TargetKind, strategy: FuzzStrategy, seed: u64) -> Self {
        Self {
            campaign_id,
            target,
            strategy,
            mutator: Mutator::new(seed),
            failures: Vec::new(),
            seen_fingerprints: std::collections::HashSet::new(),
        }
    }

    pub async fn run(&mut self, max_iterations: u64, timeout: Duration) -> Vec<FailureReport> {
        let deadline = Instant::now() + timeout;
        let mut i = 0u64;

        while i < max_iterations && Instant::now() < deadline {
            let input = self.mutator.next_input(i);
            let result = self.execute_target(&input).await;
            self.process_result(result, input, i);
            i += 1;
        }

        self.failures.clone()
    }

    async fn execute_target(&self, input: &serde_json::Value) -> ExecutionResult {
        let start = Instant::now();

        // In production: spawn the target in a sandboxed process/WASM instance.
        // Phase 1: simulate execution with validation checks only.
        match &self.target {
            TargetKind::TauriCommand { name } => {
                self.simulate_tauri_command(name, input, start.elapsed().as_millis() as u64)
            }
            TargetKind::RustCrate { name, test_filter } => {
                self.simulate_cargo_test(name, test_filter.as_deref(), input, start.elapsed().as_millis() as u64).await
            }
            TargetKind::CrdtDocument => {
                self.simulate_crdt_ops(input, start.elapsed().as_millis() as u64)
            }
            _ => ExecutionResult::Success { duration_ms: start.elapsed().as_millis() as u64 },
        }
    }

    fn simulate_tauri_command(&self, name: &str, input: &serde_json::Value, ms: u64) -> ExecutionResult {
        // Check for known-dangerous input patterns
        if let Some(s) = input.as_str() {
            if s.contains("../") || s.contains("..\\") {
                return ExecutionResult::Crash {
                    error: "Path traversal attempt blocked by TrustGuard".into(),
                    backtrace: format!("tauri_command::{}::path_guard", name),
                    input_hash: blake3::hash(s.as_bytes()).to_hex().to_string(),
                    duration_ms: ms,
                };
            }
            if s.len() > 1_000_000 {
                return ExecutionResult::Crash {
                    error: "Input too large — potential DoS".into(),
                    backtrace: format!("tauri_command::{}::size_check", name),
                    input_hash: blake3::hash(&s.len().to_le_bytes()).to_hex().to_string(),
                    duration_ms: ms,
                };
            }
        }
        if input.is_null() && name != "universe_event_count" {
            return ExecutionResult::Crash {
                error: format!("Command {} received null input where string expected", name),
                backtrace: format!("tauri_command::{}::type_check", name),
                input_hash: "null_input".into(),
                duration_ms: ms,
            };
        }
        ExecutionResult::Success { duration_ms: ms }
    }

    async fn simulate_cargo_test(&self, crate_name: &str, filter: Option<&str>, _input: &serde_json::Value, ms: u64) -> ExecutionResult {
        let mut cmd = tokio::process::Command::new("cargo");
        cmd.args(["test", "-p", crate_name]);
        if let Some(f) = filter { cmd.arg(f); }
        cmd.args(["--", "--test-threads=1"]);

        match tokio::time::timeout(Duration::from_secs(120), cmd.output()).await {
            Ok(Ok(out)) if out.status.success() => ExecutionResult::Success { duration_ms: ms },
            Ok(Ok(out)) => ExecutionResult::Crash {
                error: String::from_utf8_lossy(&out.stderr).chars().take(500).collect(),
                backtrace: format!("cargo test -p {}", crate_name),
                input_hash: "cargo_test_failure".into(),
                duration_ms: ms,
            },
            Ok(Err(e)) => ExecutionResult::Crash {
                error: e.to_string(),
                backtrace: "cargo_process_spawn".into(),
                input_hash: "spawn_error".into(),
                duration_ms: ms,
            },
            Err(_) => ExecutionResult::Hang { timeout_ms: 120_000 },
        }
    }

    fn simulate_crdt_ops(&self, _input: &serde_json::Value, ms: u64) -> ExecutionResult {
        // CRDT invariant: concurrent ops must converge
        // Phase 1: stub — real implementation calls bonsai-crdt directly
        ExecutionResult::Success { duration_ms: ms }
    }

    fn process_result(&mut self, result: ExecutionResult, input: serde_json::Value, iteration: u64) {
        let target_name = format!("{:?}", self.target).chars().take(60).collect::<String>();

        let report = match result {
            ExecutionResult::Crash { error, backtrace, input_hash, duration_ms } => {
                Some(FailureReport {
                    id:               uuid::Uuid::new_v4().to_string(),
                    campaign_id:      self.campaign_id.clone(),
                    target:           target_name.clone(),
                    strategy:         format!("{:?}", self.strategy).chars().take(40).collect(),
                    error_pattern:    error.chars().take(200).collect(),
                    backtrace:        backtrace.chars().take(500).collect(),
                    minimal_input:    input.clone(),
                    input_hash,
                    reproduction_cmd: format!("fff_replay --campaign {} --iteration {}", self.campaign_id, iteration),
                    timestamp_ns:     chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
                    auto_fix_script:  None,
                })
            }
            ExecutionResult::Hang { timeout_ms } => {
                Some(FailureReport {
                    id:               uuid::Uuid::new_v4().to_string(),
                    campaign_id:      self.campaign_id.clone(),
                    target:           target_name.clone(),
                    strategy:         "timeout".into(),
                    error_pattern:    format!("Hang: timeout after {}ms", timeout_ms),
                    backtrace:        "hang_detector".into(),
                    minimal_input:    input.clone(),
                    input_hash:       "hang".into(),
                    reproduction_cmd: format!("fff_replay --campaign {} --iteration {}", self.campaign_id, iteration),
                    timestamp_ns:     chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
                    auto_fix_script:  None,
                })
            }
            ExecutionResult::Assertion { message, input_hash } => {
                Some(FailureReport {
                    id:               uuid::Uuid::new_v4().to_string(),
                    campaign_id:      self.campaign_id.clone(),
                    target:           target_name.clone(),
                    strategy:         "assertion".into(),
                    error_pattern:    message.chars().take(200).collect(),
                    backtrace:        "assertion_failure".into(),
                    minimal_input:    input.clone(),
                    input_hash,
                    reproduction_cmd: format!("fff_replay --campaign {} --iteration {}", self.campaign_id, iteration),
                    timestamp_ns:     chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
                    auto_fix_script:  None,
                })
            }
            ExecutionResult::Violation { capability, attempted_action } => {
                Some(FailureReport {
                    id:               uuid::Uuid::new_v4().to_string(),
                    campaign_id:      self.campaign_id.clone(),
                    target:           target_name.clone(),
                    strategy:         "capability_violation".into(),
                    error_pattern:    format!("Capability violation: {} attempted {}", capability, attempted_action),
                    backtrace:        "sns_capability_check".into(),
                    minimal_input:    input.clone(),
                    input_hash:       "violation".into(),
                    reproduction_cmd: format!("fff_replay --campaign {} --iteration {}", self.campaign_id, iteration),
                    timestamp_ns:     chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
                    auto_fix_script:  Some(format!("Update sandbox policy to grant {} capability for {}", capability, target_name)),
                })
            }
            ExecutionResult::Success { .. } => None,
        };

        if let Some(r) = report {
            let fp = r.fingerprint();
            if !self.seen_fingerprints.contains(&fp) {
                self.seen_fingerprints.insert(fp);
                self.failures.push(r);
            }
        }

        self.mutator.feedback(0.0);
    }

    pub fn failure_count(&self) -> usize { self.failures.len() }
}
