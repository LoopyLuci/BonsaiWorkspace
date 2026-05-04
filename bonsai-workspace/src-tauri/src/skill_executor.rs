/// Skill Executor — typed skill manifests, sequence execution, shell hardening.
///
/// Provides:
/// - `SkillManifest`: typed schema for skill declarations (input/output contracts).
/// - `SequenceExecutor`: run multi-step sequence skills with per-step retry and rollback.
/// - `ShellGuard`: command allow/deny list and resource quotas for shell skills.
/// - `preflight_validate()`: schema + compatibility check before execution.
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::tool_core::{ToolContext, ToolError, ToolOutput, ToolResult, RetryPolicy};

/// Runtime resource ceilings applied per skill execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_cpu_seconds: u64,
    pub max_memory_mb: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_seconds: 30,
            max_memory_mb: 512,
        }
    }
}

/// Common execution settings used by language-specific skill runtimes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillConfig {
    #[serde(default)]
    pub resource_limits: ResourceLimits,
    #[serde(default)]
    pub required_bb_version: Option<String>,
}

/// Compose a BONSAI_ALLOWED_PATHS value from workspace root and explicit skill paths.
pub fn build_allowed_paths_env(workspace_root: &Path, skill_paths: &[String]) -> String {
    let mut roots = std::collections::BTreeSet::new();
    roots.insert(workspace_root.to_string_lossy().to_string());
    for p in skill_paths {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            roots.insert(trimmed.to_string());
        }
    }

    let sep = if cfg!(target_os = "windows") { ";" } else { ":" };
    roots.into_iter().collect::<Vec<_>>().join(sep)
}

/// Returns true when the installed babashka version does not match a required pin.
pub fn bb_version_mismatch(required_bb_version: Option<&str>, installed_version: &str) -> bool {
    match required_bb_version.map(str::trim).filter(|v| !v.is_empty()) {
        Some(required) => !installed_version.contains(required),
        None => false,
    }
}

// ── Skill Manifest ────────────────────────────────────────────────────────────

/// Formal contract for a skill: describes its input schema, output schema,
/// required capabilities, and compatibility constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    /// Unique skill name (must match `user_skills.name`).
    pub name:         String,
    /// Human-readable description.
    pub description:  String,
    /// Skill kind: "shell" | "sequence".
    pub kind:         String,
    /// Semver-compatible version string.
    pub version:      String,
    /// Minimum app version required to run this skill.
    pub min_app_version: Option<String>,
    /// Required platform capability tags (e.g. ["internet", "filesystem"]).
    pub requires:     Vec<String>,
    /// JSON Schema for accepted input `args`.
    pub input_schema: Value,
    /// JSON Schema describing the expected output shape (informational).
    pub output_schema: Value,
    /// Whether the skill produces side effects outside the process.
    pub side_effects: bool,
    /// Per-step retry policy for sequence skills (ignored for shell).
    pub retry_policy: Option<SequenceRetryPolicy>,
}

impl SkillManifest {
    /// Validate that the provided `args` value matches `input_schema`.
    /// Uses a lightweight structural check (type + required field presence).
    pub fn validate_args(&self, args: &Value) -> Result<(), String> {
        let schema = &self.input_schema;

        // If schema says "type": "object", check required fields.
        if schema.get("type").and_then(|v| v.as_str()) == Some("object") {
            let obj = args.as_object()
                .ok_or_else(|| "args must be a JSON object".to_string())?;

            if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
                for field in required {
                    let name = field.as_str().unwrap_or("");
                    if !obj.contains_key(name) {
                        return Err(format!("missing required arg: '{name}'"));
                    }
                }
            }

            if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
                for (key, prop_schema) in props {
                    if let Some(val) = obj.get(key) {
                        let expected_type = prop_schema.get("type").and_then(|t| t.as_str());
                        let actual_ok = match expected_type {
                            Some("string")  => val.is_string(),
                            Some("number")  => val.is_number(),
                            Some("boolean") => val.is_boolean(),
                            Some("array")   => val.is_array(),
                            Some("object")  => val.is_object(),
                            _               => true,
                        };
                        if !actual_ok {
                            return Err(format!(
                                "arg '{key}' expected type '{}', got '{}'",
                                expected_type.unwrap_or("any"),
                                json_type_name(val),
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check that the current runtime satisfies all `requires` capabilities.
    pub fn check_compatibility(&self, granted: &HashSet<String>) -> Result<(), String> {
        for cap in &self.requires {
            if !granted.contains(cap) {
                return Err(format!("skill '{}' requires capability '{cap}' which is not granted", self.name));
            }
        }
        Ok(())
    }
}

fn json_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null    => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_)  => "array",
        Value::Object(_) => "object",
    }
}

// ── Preflight validation ──────────────────────────────────────────────────────

/// Run all preflight checks before executing a skill.
/// Returns Ok(()) if the skill can proceed, Err with a user-facing message otherwise.
pub fn preflight_validate(
    manifest:  &SkillManifest,
    args:      &Value,
    granted:   &HashSet<String>,
) -> Result<(), String> {
    manifest.validate_args(args)?;
    manifest.check_compatibility(granted)?;
    Ok(())
}

// ── Sequence step types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceStep {
    /// Tool name to invoke.
    pub tool:       String,
    /// Args to pass (can reference `$prev` for prior step output).
    pub args:       Value,
    /// Per-step retry override.
    pub retry:      Option<SequenceRetryPolicy>,
    /// Rollback tool to call if a later step fails.
    pub rollback:   Option<RollbackSpec>,
    /// Human-readable label for diagnostics.
    pub label:      Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceRetryPolicy {
    pub max_attempts:    u32,
    pub backoff_ms:      u64,
}

impl Default for SequenceRetryPolicy {
    fn default() -> Self {
        Self { max_attempts: 1, backoff_ms: 0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackSpec {
    pub tool: String,
    pub args: Value,
}

// ── Sequence executor ─────────────────────────────────────────────────────────

/// Invoker function type for sequence execution.
/// Takes (tool_name, args) and returns a ToolResult.
pub type StepInvoker = Arc<dyn Fn(String, Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolResult> + Send>> + Send + Sync>;

/// Execute a sequence of steps; on any step failure attempt rollback
/// in reverse order. Returns all step outputs or the first error.
pub async fn run_sequence(
    steps:         &[SequenceStep],
    ctx:           &ToolContext,
    invoke:        StepInvoker,
    default_retry: &SequenceRetryPolicy,
) -> Result<Vec<Value>, ToolError> {
    let mut outputs: Vec<Value> = Vec::with_capacity(steps.len());
    let mut completed: Vec<usize> = Vec::new();

    for (i, step) in steps.iter().enumerate() {
        if ctx.is_cancelled() {
            return Err(ToolError::Internal { message: "sequence cancelled".into() });
        }

        let retry = step.retry.as_ref().unwrap_or(default_retry);
        let label = step.label.as_deref().unwrap_or(&step.tool).to_string();

        let args = substitute_prev(&step.args, outputs.last());

        let mut last_err: Option<ToolError> = None;
        let mut succeeded = false;

        for attempt in 0..retry.max_attempts.max(1) {
            if attempt > 0 && retry.backoff_ms > 0 {
                tokio::time::sleep(Duration::from_millis(retry.backoff_ms)).await;
            }

            match invoke(step.tool.clone(), args.clone()).await {
                Ok(ToolOutput::Complete(v)) => {
                    outputs.push(v);
                    succeeded = true;
                    break;
                }
                Ok(ToolOutput::Streaming(_)) => {
                    last_err = Some(ToolError::Internal {
                        message: format!("step '{label}': streaming output not supported in sequences"),
                    });
                    break;
                }
                Err(e) => {
                    last_err = Some(e);
                }
            }
        }

        if succeeded {
            completed.push(i);
        } else {
            // Rollback completed steps in reverse order
            for &done_idx in completed.iter().rev() {
                if let Some(rb) = &steps[done_idx].rollback {
                    let _ = invoke(rb.tool.clone(), rb.args.clone()).await;
                }
            }
            return Err(last_err.unwrap_or_else(|| ToolError::Internal {
                message: format!("step '{label}' failed"),
            }));
        }
    }

    Ok(outputs)
}

fn substitute_prev(args: &Value, prev: Option<&Value>) -> Value {
    match args {
        Value::String(s) if s == "$prev" => {
            prev.cloned().unwrap_or(Value::Null)
        }
        Value::Object(map) => {
            let replaced = map.iter().map(|(k, v)| {
                (k.clone(), substitute_prev(v, prev))
            })
            .collect();
            Value::Object(replaced)
        }
        other => other.clone(),
    }
}

// ── Shell guard ───────────────────────────────────────────────────────────────

/// Command allow/deny list and resource quotas for shell skill execution.
#[derive(Debug, Clone)]
pub struct ShellGuard {
    /// If non-empty, only commands in this list are allowed.
    pub allow_list: Vec<String>,
    /// Commands that are always denied regardless of allow list.
    pub deny_list:  Vec<String>,
    /// Maximum wall-clock execution time.
    pub timeout:    Duration,
    /// Maximum output bytes (stdout + stderr combined).
    pub max_output: usize,
}

impl Default for ShellGuard {
    fn default() -> Self {
        Self {
            allow_list: Vec::new(),
            deny_list:  vec![
                "rm -rf /".into(), "mkfs".into(), "dd if=/dev/zero".into(),
                ":(){ :|:& };:".into(), "shutdown".into(), "reboot".into(),
                "halt".into(), "poweroff".into(),
            ],
            timeout:    Duration::from_secs(30),
            max_output: 256 * 1024, // 256 KB
        }
    }
}

impl ShellGuard {
    /// Check if a shell script is permitted to run.
    pub fn check(&self, script: &str) -> Result<(), String> {
        let lower = script.to_lowercase();

        // Deny list takes priority.
        for denied in &self.deny_list {
            if lower.contains(denied.to_lowercase().as_str()) {
                return Err(format!("shell script contains denied pattern: '{denied}'"));
            }
        }

        // Allow list: if non-empty, the first token of the script must be in it.
        if !self.allow_list.is_empty() {
            let first_token = script.split_whitespace().next().unwrap_or("");
            let allowed = self.allow_list.iter().any(|a| {
                a == first_token || first_token.ends_with(&format!("/{a}"))
            });
            if !allowed {
                return Err(format!(
                    "shell command '{first_token}' is not in the allow list"
                ));
            }
        }

        Ok(())
    }

    /// Execute a shell script under this guard's constraints.
    pub async fn execute(&self, script: &str) -> ToolResult {
        self.check(script).map_err(|e| ToolError::PolicyDenied { reason: e })?;

        // Use platform-appropriate shell
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = tokio::process::Command::new("cmd");
            c.arg("/C").arg(script);
            c
        } else {
            let mut c = tokio::process::Command::new("sh");
            c.arg("-c").arg(script);
            c
        };
        let output_fut = cmd.kill_on_drop(true).output();

        let output = match tokio::time::timeout(self.timeout, output_fut).await {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => return Err(ToolError::Internal {
                message: format!("shell spawn failed: {e}"),
            }),
            Err(_) => return Err(ToolError::Timeout {
                duration_ms: self.timeout.as_millis() as u64,
            }),
        };

        let mut stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let mut stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined_len = stdout.len() + stderr.len();

        if combined_len > self.max_output {
            let cap = self.max_output / 2;
            stdout.truncate(cap);
            stderr.truncate(cap);
            stderr.push_str("\n[output truncated: exceeded max_output limit]");
        }

        Ok(ToolOutput::Complete(json!({
            "stdout":    stdout,
            "stderr":    stderr,
            "exit_code": output.status.code().unwrap_or(-1),
        })))
    }
}

// ── Policy simulation mode ────────────────────────────────────────────────────

/// Simulate skill execution without actually running side effects.
/// Returns what *would* happen: guard result, manifest validation, compatibility.
pub fn simulate_execution(
    manifest: &SkillManifest,
    args:     &Value,
    granted:  &HashSet<String>,
    guard:    Option<&ShellGuard>,
    script:   Option<&str>,
) -> SimulationReport {
    let mut issues = Vec::new();

    if let Err(e) = manifest.validate_args(args) {
        issues.push(format!("arg validation: {e}"));
    }
    if let Err(e) = manifest.check_compatibility(granted) {
        issues.push(format!("compatibility: {e}"));
    }
    if let (Some(guard), Some(script)) = (guard, script) {
        if let Err(e) = guard.check(script) {
            issues.push(format!("shell guard: {e}"));
        }
    }

    SimulationReport {
        skill:   manifest.name.clone(),
        allowed: issues.is_empty(),
        issues,
    }
}

#[derive(Debug, Serialize)]
pub struct SimulationReport {
    pub skill:   String,
    pub allowed: bool,
    pub issues:  Vec<String>,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn basic_manifest() -> SkillManifest {
        SkillManifest {
            name:            "test_skill".into(),
            description:     "A test skill".into(),
            kind:            "shell".into(),
            version:         "1.0.0".into(),
            min_app_version: None,
            requires:        vec!["filesystem".into()],
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            }),
            output_schema: json!({}),
            side_effects:  true,
            retry_policy:  None,
        }
    }

    #[test]
    fn manifest_validate_args_ok() {
        let m = basic_manifest();
        let args = json!({ "path": "/tmp/file.txt" });
        assert!(m.validate_args(&args).is_ok());
    }

    #[test]
    fn manifest_validate_args_missing_required() {
        let m = basic_manifest();
        let args = json!({});
        let err = m.validate_args(&args).unwrap_err();
        assert!(err.contains("missing required arg: 'path'"));
    }

    #[test]
    fn manifest_validate_args_wrong_type() {
        let m = basic_manifest();
        let args = json!({ "path": 42 }); // should be string
        let err = m.validate_args(&args).unwrap_err();
        assert!(err.contains("path"));
    }

    #[test]
    fn compatibility_check_missing_cap() {
        let m = basic_manifest();
        let granted: HashSet<String> = HashSet::new();
        let err = m.check_compatibility(&granted).unwrap_err();
        assert!(err.contains("filesystem"));
    }

    #[test]
    fn compatibility_check_ok() {
        let m = basic_manifest();
        let granted: HashSet<String> = ["filesystem".to_string()].into();
        assert!(m.check_compatibility(&granted).is_ok());
    }

    #[test]
    fn shell_guard_deny_list_blocks() {
        let guard = ShellGuard::default();
        let err = guard.check("rm -rf /").unwrap_err();
        assert!(err.contains("denied pattern"));
    }

    #[test]
    fn shell_guard_allow_list_blocks_unlisted() {
        let guard = ShellGuard {
            allow_list: vec!["echo".into()],
            ..ShellGuard::default()
        };
        let err = guard.check("cat /etc/passwd").unwrap_err();
        assert!(err.contains("not in the allow list"));
    }

    #[test]
    fn shell_guard_allow_list_permits_listed() {
        let guard = ShellGuard {
            allow_list: vec!["echo".into()],
            ..ShellGuard::default()
        };
        assert!(guard.check("echo hello").is_ok());
    }

    #[test]
    fn shell_guard_empty_allow_list_permits_any_safe() {
        let guard = ShellGuard::default();
        assert!(guard.check("echo hello world").is_ok());
    }

    #[tokio::test]
    async fn shell_guard_executes_safe_command() {
        let guard = ShellGuard::default();
        let result = guard.execute("echo hello").await.unwrap();
        if let ToolOutput::Complete(v) = result {
            assert!(v["stdout"].as_str().unwrap_or("").contains("hello"));
        } else {
            panic!("expected Complete output");
        }
    }

    #[tokio::test]
    async fn shell_guard_rejects_denied_command() {
        let guard = ShellGuard::default();
        let err = guard.execute("shutdown now").await.unwrap_err();
        assert!(matches!(err, ToolError::PolicyDenied { .. }));
    }

    #[test]
    fn simulate_blocked_by_missing_capability() {
        let m = basic_manifest();
        let args = json!({ "path": "/tmp" });
        let granted: HashSet<String> = HashSet::new();
        let report = simulate_execution(&m, &args, &granted, None, None);
        assert!(!report.allowed);
        assert!(report.issues.iter().any(|i| i.contains("filesystem")));
    }

    #[test]
    fn simulate_allowed_when_all_ok() {
        let m = basic_manifest();
        let args = json!({ "path": "/tmp" });
        let granted: HashSet<String> = ["filesystem".to_string()].into();
        let report = simulate_execution(&m, &args, &granted, None, None);
        assert!(report.allowed);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn substitute_prev_replaces_dollar_prev() {
        let args = json!("$prev");
        let prev = json!({"result": 42});
        let out = substitute_prev(&args, Some(&prev));
        assert_eq!(out, json!({"result": 42}));
    }

    #[test]
    fn substitute_prev_nested_replacement() {
        let args = json!({ "input": "$prev" });
        let prev = json!("hello");
        let out = substitute_prev(&args, Some(&prev));
        assert_eq!(out["input"], json!("hello"));
    }
}
