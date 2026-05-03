/// Shared foundation for all Bonsai tool runtimes.
///
/// Both the assistant window (assistant_tools) and the workspace chat (tools)
/// implement the `Tool` trait defined here. They each maintain a separate
/// `ToolRegistry` instance — no forced merger of semantically distinct tool sets.
use std::collections::HashMap;
use std::sync::{atomic::AtomicBool, Arc, OnceLock, Mutex};
use serde_json::{json, Value};
use tokio::sync::mpsc;

/// Deduplicated string interning pool for `&'static str` tool names / descriptions.
/// `Box::leak` is called at most once per unique string, preventing repeated leaks on
/// hot paths like MCP server reconnections or skill reloads.
pub(crate) fn intern_str(s: String) -> &'static str {
    static POOL: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    let pool = POOL.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = pool.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(&ptr) = guard.get(&s) {
        return ptr;
    }
    let leaked: &'static str = Box::leak(s.clone().into_boxed_str());
    guard.insert(s, leaked);
    leaked
}

// ── Side-effect profile ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum SideEffectProfile {
    /// Safe to parallelize with anything: datetime, weather, render_chart.
    None,
    /// Safe to parallelize with other Reads to non-overlapping paths.
    Read,
    /// Must be serialized; blocks concurrent Reads to the same path.
    Write,
    /// Always serialized, always confirmation-gated regardless of policy.
    External,
    /// Treat as External (conservative default for dynamic / MCP tools).
    Unknown,
}

impl SideEffectProfile {
    pub fn can_parallelize_with(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::None, _) | (_, Self::None) | (Self::Read, Self::Read)
        )
    }
    pub fn is_cacheable(&self) -> bool {
        matches!(self, Self::None | Self::Read)
    }
}

// ── Risk level ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Copy)]
pub enum RiskLevel {
    Safe,          // no confirmation needed
    LowRisk,       // confirm on first use per session
    Destructive,   // always confirm
    AlwaysConfirm, // explicit approval every time, no exceptions
}

// ── Tool policy hint ─────────────────────────────────────────────────────────
// Advisory ceiling: PolicyEngine may only raise risk from this baseline, never lower.

#[derive(Debug, Clone)]
pub struct ToolPolicyHint {
    pub max_risk:        RiskLevel,
    pub is_network:      bool,
    pub is_filesystem:   bool,
    pub max_duration_ms: u64,
}

impl ToolPolicyHint {
    pub fn safe() -> Self {
        Self { max_risk: RiskLevel::Safe, is_network: false, is_filesystem: false, max_duration_ms: 5_000 }
    }
    pub fn network() -> Self {
        Self { max_risk: RiskLevel::Safe, is_network: true, is_filesystem: false, max_duration_ms: 30_000 }
    }
    pub fn filesystem_read() -> Self {
        Self { max_risk: RiskLevel::Safe, is_network: false, is_filesystem: true, max_duration_ms: 5_000 }
    }
    pub fn filesystem_write() -> Self {
        Self { max_risk: RiskLevel::Destructive, is_network: false, is_filesystem: true, max_duration_ms: 10_000 }
    }
    pub fn external() -> Self {
        Self { max_risk: RiskLevel::AlwaysConfirm, is_network: true, is_filesystem: false, max_duration_ms: 60_000 }
    }
}

// ── Structured tool error taxonomy ───────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ToolError {
    /// Network hiccup, server 503, DNS timeout — safe to retry.
    Transient { message: String, retry_after_ms: Option<u64> },
    /// Missing credentials or misconfigured integration.
    Configuration { message: String, fix_hint: String },
    /// OS or policy denied access.
    Permission { message: String },
    /// File/resource does not exist.
    NotFound { resource: String },
    /// Remote rate limit hit.
    RateLimited { retry_after_ms: u64 },
    /// Execution exceeded its max_duration budget.
    Timeout { duration_ms: u64 },
    /// Prompt-injection pattern detected — tool call aborted.
    InjectionBlocked,
    /// Model requested a tool name that was not injected into this context.
    NotInContext { tool_name: String },
    /// PolicyEngine denied the call.
    PolicyDenied { reason: String },
    /// Arg schema validation failed.
    ValidationFailed { field: String, reason: String },
    /// Unexpected internal error.
    Internal { message: String },
}

impl ToolError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Transient { .. } | Self::RateLimited { .. } | Self::Timeout { .. })
    }

    /// Human-readable message suitable for feeding back into the LLM context.
    pub fn to_llm_message(&self) -> String {
        match self {
            Self::Transient { message, .. } =>
                format!("Temporary error: {message}. This may succeed if retried."),
            Self::Configuration { message, fix_hint } =>
                format!("Configuration error: {message}. How to fix: {fix_hint}"),
            Self::Permission { message } =>
                format!("Permission denied: {message}"),
            Self::NotFound { resource } =>
                format!("Not found: {resource}"),
            Self::RateLimited { retry_after_ms } =>
                format!("Rate limited. Retry after {retry_after_ms}ms."),
            Self::Timeout { duration_ms } =>
                format!("Tool timed out after {duration_ms}ms."),
            Self::InjectionBlocked =>
                "Tool call blocked: potential prompt injection detected in reasoning.".into(),
            Self::NotInContext { tool_name } =>
                format!("Tool '{tool_name}' was not available in this context window. Ask me to use an available tool."),
            Self::PolicyDenied { reason } =>
                format!("Tool denied by policy: {reason}"),
            Self::ValidationFailed { field, reason } =>
                format!("Invalid argument '{field}': {reason}"),
            Self::Internal { message } =>
                format!("Internal error: {message}"),
        }
    }
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_llm_message())
    }
}

// ── Retry policy ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts:  u8,
    pub base_delay_ms: u64,
}

impl RetryPolicy {
    pub fn none()    -> Self { Self { max_attempts: 1, base_delay_ms: 0 } }
    pub fn network() -> Self { Self { max_attempts: 3, base_delay_ms: 500 } }
    pub fn backoff_ms(&self, attempt: u8) -> u64 {
        self.base_delay_ms * (1u64 << attempt.min(4))
    }
}

// ── Streaming tool output ─────────────────────────────────────────────────────

pub struct ToolChunk {
    pub delta: String,
    pub done:  bool,
    pub data:  Option<Value>, // final structured result when done=true
}

pub enum ToolOutput {
    Complete(Value),
    Streaming(mpsc::Receiver<ToolChunk>),
}

impl std::fmt::Debug for ToolOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolOutput::Complete(v) => f.debug_tuple("Complete").field(v).finish(),
            ToolOutput::Streaming(_) => f.write_str("Streaming(<mpsc::Receiver>)"),
        }
    }
}

pub type ToolResult = Result<ToolOutput, ToolError>;

// ── Tool execution context ────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ToolContext {
    /// Current open workspace root (None if no workspace loaded).
    pub workspace_path: Option<String>,
    /// Profile ID whose tool_permissions apply to this execution.
    pub profile_id:     String,
    /// Session ID for cache tenancy and audit correlation.
    pub session_id:     String,
    /// Turn-level correlation ID — every tool call in a turn shares this.
    pub turn_id:        String,
    /// Prevents tool→tool recursion. Starts at 0; tools must reject if ≥ 4.
    pub call_depth:     u8,
    /// Cancellation: `true` means the user cancelled the turn.
    pub cancel:         Arc<AtomicBool>,
    /// OS keychain abstraction for SMTP and future secrets.
    pub secrets:        Arc<crate::secrets_store::SecretsStore>,
}

impl ToolContext {
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Produce a child context for a nested tool call with depth incremented.
    pub fn child(&self) -> Self {
        Self { call_depth: self.call_depth + 1, ..self.clone() }
    }

    /// Turn a raw args Value into a typed struct, returning ValidationFailed on error.
    pub fn parse_args<T: serde::de::DeserializeOwned>(args: &Value) -> Result<T, ToolError> {
        serde_json::from_value(args.clone()).map_err(|e| ToolError::ValidationFailed {
            field: "args".into(),
            reason: e.to_string(),
        })
    }
}

// ── The Tool trait ────────────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait Tool: Send + Sync + 'static {
    /// Stable machine-readable name used in LLM tool_calls and the registry key.
    fn name(&self)         -> &'static str;
    /// Natural-language description injected into the LLM's tool schema.
    fn description(&self)  -> &'static str;
    /// JSON Schema object for the `input_schema` field in the tool definition.
    fn schema(&self)       -> Value;
    /// Advisory policy ceiling — PolicyEngine may only raise risk, never lower.
    fn policy_hint(&self)  -> ToolPolicyHint;
    /// Execution side-effect classification — drives the parallel scheduler.
    fn side_effects(&self) -> SideEffectProfile;
    /// Topic tags for the semantic selector keyword index.
    fn tags(&self)         -> &'static [&'static str];
    /// Cache TTL in seconds. `None` disables caching (default for side-effecting tools).
    fn cache_ttl_secs(&self) -> Option<u64> {
        if self.side_effects().is_cacheable() { Some(300) } else { None }
    }
    /// Retry policy for transient failures.
    fn retry_policy(&self) -> RetryPolicy { RetryPolicy::none() }

    /// Execute the tool. Called only after PolicyEngine has approved the call.
    async fn execute(&self, args: &Value, ctx: &ToolContext) -> ToolResult;
}

// ── Tool registry ─────────────────────────────────────────────────────────────

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: HashMap::new() }
    }

    pub fn register(&mut self, tool: impl Tool) {
        let t = Arc::new(tool) as Arc<dyn Tool>;
        self.tools.insert(t.name().to_string(), t);
    }

    pub fn register_arc(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    pub fn len(&self) -> usize { self.tools.len() }
    pub fn is_empty(&self) -> bool { self.tools.is_empty() }

    /// Full tool definitions (JSON schema array for the LLM).
    /// Optionally filtered to `names` subset; filtered by profile permissions.
    pub fn definitions(
        &self,
        names:   Option<&[String]>,
        perms:   Option<&Value>,
    ) -> Vec<Value> {
        self.tools.values()
            .filter(|t| {
                if let Some(n) = names {
                    if !n.iter().any(|x| x == t.name()) { return false; }
                }
                if let Some(p) = perms {
                    if p.get(t.name()) == Some(&Value::Bool(false)) { return false; }
                }
                true
            })
            .map(|t| json!({
                "name":         t.name(),
                "description":  t.description(),
                "input_schema": t.schema(),
            }))
            .collect()
    }

    /// All tool definitions (no permission filter) — used by the selector to build its index.
    pub fn all_definitions(&self) -> Vec<Value> {
        self.tools.values().map(|t| json!({
            "name":         t.name(),
            "description":  t.description(),
            "tags":         t.tags(),
        })).collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self { Self::new() }
}

// ── Execution result envelope (for the ReAct loop) ────────────────────────────

#[derive(Debug, Clone)]
pub struct ToolCallOutcome {
    pub tool_call_id: String,
    pub tool_name:    String,
    pub args:         Value,
    pub result_json:  String,   // serialised ToolOutput or error message
    pub decision:     String,   // "allowed" | "denied" | "confirm_required" | "injected" | "cached"
    pub duration_ms:  u64,
    pub from_cache:   bool,
}

impl ToolCallOutcome {
    /// Converts to the `{"role":"tool", ...}` message entry for the LLM context.
    pub fn to_context_message(&self) -> Value {
        json!({
            "role":        "tool",
            "tool_call_id": self.tool_call_id,
            "content":     self.result_json,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_errors_are_limited_to_transient_rate_limited_and_timeout() {
        assert!(ToolError::Transient { message: "tmp".into(), retry_after_ms: None }.is_retryable());
        assert!(ToolError::RateLimited { retry_after_ms: 250 }.is_retryable());
        assert!(ToolError::Timeout { duration_ms: 5_000 }.is_retryable());
    }

    #[test]
    fn non_retryable_errors_remain_non_retryable() {
        assert!(!ToolError::ValidationFailed { field: "path".into(), reason: "missing".into() }.is_retryable());
        assert!(!ToolError::PolicyDenied { reason: "blocked".into() }.is_retryable());
        assert!(!ToolError::Permission { message: "denied".into() }.is_retryable());
        assert!(!ToolError::InjectionBlocked.is_retryable());
    }

    #[test]
    fn retry_backoff_is_bounded_exponential() {
        let policy = RetryPolicy { max_attempts: 5, base_delay_ms: 100 };
        assert_eq!(policy.backoff_ms(0), 100);
        assert_eq!(policy.backoff_ms(1), 200);
        assert_eq!(policy.backoff_ms(2), 400);
        assert_eq!(policy.backoff_ms(4), 1_600);
        // attempt>4 clamps to 4 for bounded growth
        assert_eq!(policy.backoff_ms(8), 1_600);
    }

    // ── Scheduler / parallelization tests ────────────────────────────────────

    #[test]
    fn none_can_parallelize_with_anything() {
        assert!(SideEffectProfile::None.can_parallelize_with(&SideEffectProfile::None));
        assert!(SideEffectProfile::None.can_parallelize_with(&SideEffectProfile::Read));
        assert!(SideEffectProfile::None.can_parallelize_with(&SideEffectProfile::Write));
        assert!(SideEffectProfile::None.can_parallelize_with(&SideEffectProfile::External));
        assert!(SideEffectProfile::None.can_parallelize_with(&SideEffectProfile::Unknown));
    }

    #[test]
    fn reads_can_parallelize_with_reads_but_not_writes() {
        assert!(SideEffectProfile::Read.can_parallelize_with(&SideEffectProfile::Read));
        assert!(!SideEffectProfile::Read.can_parallelize_with(&SideEffectProfile::Write));
        assert!(!SideEffectProfile::Read.can_parallelize_with(&SideEffectProfile::External));
    }

    #[test]
    fn writes_cannot_parallelize_with_non_none() {
        // None is a no-op side-effect so it parallelizes with everything including Write.
        // Any non-None pair involving Write must be serialized.
        assert!(!SideEffectProfile::Write.can_parallelize_with(&SideEffectProfile::Read));
        assert!(!SideEffectProfile::Write.can_parallelize_with(&SideEffectProfile::Write));
        assert!(!SideEffectProfile::Write.can_parallelize_with(&SideEffectProfile::External));
        // But Write with None is fine — None has no side effects to conflict with
        assert!(SideEffectProfile::Write.can_parallelize_with(&SideEffectProfile::None));
    }

    #[test]
    fn external_and_unknown_cannot_parallelize_with_each_other() {
        assert!(!SideEffectProfile::External.can_parallelize_with(&SideEffectProfile::External));
        assert!(!SideEffectProfile::Unknown.can_parallelize_with(&SideEffectProfile::Unknown));
        assert!(!SideEffectProfile::External.can_parallelize_with(&SideEffectProfile::Unknown));
    }

    #[test]
    fn cacheability_only_for_none_and_read() {
        assert!(SideEffectProfile::None.is_cacheable());
        assert!(SideEffectProfile::Read.is_cacheable());
        assert!(!SideEffectProfile::Write.is_cacheable());
        assert!(!SideEffectProfile::External.is_cacheable());
        assert!(!SideEffectProfile::Unknown.is_cacheable());
    }

    // ── Cancellation tests ───────────────────────────────────────────────────

    fn make_test_ctx(cancel: Arc<std::sync::atomic::AtomicBool>) -> ToolContext {
        use crate::secrets_store::SecretsStore;
        ToolContext {
            session_id:     "s1".into(),
            turn_id:        "t1".into(),
            workspace_path: Some("/tmp".into()),
            profile_id:     "default".into(),
            call_depth:     0,
            cancel,
            secrets:        Arc::new(SecretsStore::new()),
        }
    }

    #[test]
    fn context_starts_not_cancelled() {
        let ctx = make_test_ctx(Arc::new(std::sync::atomic::AtomicBool::new(false)));
        assert!(!ctx.is_cancelled());
    }

    #[test]
    fn context_reports_cancelled_after_flag_set() {
        let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let ctx = make_test_ctx(flag.clone());
        assert!(!ctx.is_cancelled());
        flag.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(ctx.is_cancelled());
    }

    // ── ToolError taxonomy completeness tests ────────────────────────────────

    #[test]
    fn configuration_error_is_not_retryable() {
        assert!(!ToolError::Configuration {
            message: "missing api key".into(),
            fix_hint: "set OPENAI_API_KEY".into(),
        }.is_retryable());
    }

    #[test]
    fn not_found_error_is_not_retryable() {
        assert!(!ToolError::NotFound { resource: "file.txt".into() }.is_retryable());
    }

    #[test]
    fn injection_blocked_is_not_retryable() {
        assert!(!ToolError::InjectionBlocked.is_retryable());
    }

    #[test]
    fn not_in_context_is_not_retryable() {
        assert!(!ToolError::NotInContext { tool_name: "unknown_tool".into() }.is_retryable());
    }

    #[test]
    fn internal_error_is_not_retryable() {
        assert!(!ToolError::Internal { message: "panic".into() }.is_retryable());
    }

    #[test]
    fn rate_limited_with_zero_delay_is_retryable() {
        assert!(ToolError::RateLimited { retry_after_ms: 0 }.is_retryable());
    }

    // ── ToolCallOutcome context message tests ────────────────────────────────

    #[test]
    fn outcome_to_context_message_has_correct_structure() {
        let outcome = ToolCallOutcome {
            tool_call_id: "tc1".into(),
            tool_name:    "get_datetime".into(),
            args:         serde_json::json!({}),
            result_json:  r#"{"now":"2026-04-19"}"#.into(),
            decision:     "allowed".into(),
            duration_ms:  12,
            from_cache:   false,
        };
        let msg = outcome.to_context_message();
        assert_eq!(msg["role"], "tool");
        assert_eq!(msg["tool_call_id"], "tc1");
        assert_eq!(msg["content"], r#"{"now":"2026-04-19"}"#);
    }
}
