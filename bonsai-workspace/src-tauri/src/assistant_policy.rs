use std::collections::HashMap;
use serde_json::Value;
use crate::tool_core::RiskLevel;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum PolicyDecision {
    Allow,
    Deny(String),
    RequireConfirmation(String),
}

#[derive(Debug, Clone)]
pub struct ArgRule {
    pub field:          String,
    pub required:       bool,
    pub max_len:        Option<usize>,
    pub max_value:      Option<f64>,
    pub allowed_values: Option<Vec<String>>,
    pub is_path:        bool,  // if true, check against path_sandbox
    pub is_url:         bool,  // if true, check against domain_allowlist
}

#[derive(Debug, Clone)]
pub struct ToolPolicy {
    pub requires_confirm:           bool,
    pub high_risk_prompt:           String,
    pub path_sandbox_applies:       bool,
    pub domain_allowlist_applies:   bool,
    pub arg_rules:                  Vec<ArgRule>,
}

// ── PolicyEngine ──────────────────────────────────────────────────────────────

pub struct PolicyEngine {
    default_policies: HashMap<String, ToolPolicy>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        let mut policies = HashMap::new();

        // fetch_url — low-risk read, but domain allowlist applies
        policies.insert("fetch_url".into(), ToolPolicy {
            requires_confirm: false,
            high_risk_prompt: String::new(),
            path_sandbox_applies: false,
            domain_allowlist_applies: true,
            arg_rules: vec![
                ArgRule { field: "url".into(), required: true, max_len: Some(2048), max_value: None, allowed_values: None, is_path: false, is_url: true },
                ArgRule { field: "strip_html".into(), required: false, max_len: None, max_value: None, allowed_values: None, is_path: false, is_url: false },
            ],
        });

        // get_weather — read-only, network, no confirmation
        policies.insert("get_weather".into(), ToolPolicy {
            requires_confirm: false,
            high_risk_prompt: String::new(),
            path_sandbox_applies: false,
            domain_allowlist_applies: false,
            arg_rules: vec![
                ArgRule { field: "location".into(), required: false, max_len: Some(256), max_value: None, allowed_values: None, is_path: false, is_url: false },
            ],
        });

        // get_datetime — trivial
        policies.insert("get_datetime".into(), ToolPolicy {
            requires_confirm: false,
            high_risk_prompt: String::new(),
            path_sandbox_applies: false,
            domain_allowlist_applies: false,
            arg_rules: vec![
                ArgRule { field: "format".into(), required: false, max_len: Some(64), max_value: None, allowed_values: None, is_path: false, is_url: false },
            ],
        });

        // get_system_stats — read-only
        policies.insert("get_system_stats".into(), ToolPolicy {
            requires_confirm: false,
            high_risk_prompt: String::new(),
            path_sandbox_applies: false,
            domain_allowlist_applies: false,
            arg_rules: vec![],
        });

        // render_chart — no I/O
        policies.insert("render_chart".into(), ToolPolicy {
            requires_confirm: false,
            high_risk_prompt: String::new(),
            path_sandbox_applies: false,
            domain_allowlist_applies: false,
            arg_rules: vec![
                ArgRule { field: "chart_type".into(), required: true, max_len: Some(16), max_value: None, allowed_values: Some(vec!["bar".into(), "line".into(), "pie".into()]), is_path: false, is_url: false },
                ArgRule { field: "data_json".into(), required: true, max_len: Some(32768), max_value: None, allowed_values: None, is_path: false, is_url: false },
                ArgRule { field: "title".into(), required: false, max_len: Some(256), max_value: None, allowed_values: None, is_path: false, is_url: false },
            ],
        });

        // find_files — path sandbox applies
        policies.insert("find_files".into(), ToolPolicy {
            requires_confirm: false,
            high_risk_prompt: String::new(),
            path_sandbox_applies: true,
            domain_allowlist_applies: false,
            arg_rules: vec![
                ArgRule { field: "path".into(), required: true, max_len: Some(1024), max_value: None, allowed_values: None, is_path: true, is_url: false },
                ArgRule { field: "pattern".into(), required: true, max_len: Some(256), max_value: None, allowed_values: None, is_path: false, is_url: false },
                ArgRule { field: "max_results".into(), required: false, max_len: None, max_value: Some(500.0), allowed_values: None, is_path: false, is_url: false },
            ],
        });

        // read_file_assistant — high privilege, path sandbox, confirmation
        policies.insert("read_file_assistant".into(), ToolPolicy {
            requires_confirm: false,
            high_risk_prompt: String::new(),
            path_sandbox_applies: true,
            domain_allowlist_applies: false,
            arg_rules: vec![
                ArgRule { field: "path".into(), required: true, max_len: Some(1024), max_value: None, allowed_values: None, is_path: true, is_url: false },
            ],
        });

        // write_file_assistant — requires confirmation, path sandbox
        policies.insert("write_file_assistant".into(), ToolPolicy {
            requires_confirm: true,
            high_risk_prompt: "Write to file on disk?".into(),
            path_sandbox_applies: true,
            domain_allowlist_applies: false,
            arg_rules: vec![
                ArgRule { field: "path".into(), required: true, max_len: Some(1024), max_value: None, allowed_values: None, is_path: true, is_url: false },
                ArgRule { field: "content".into(), required: true, max_len: Some(1_048_576), max_value: None, allowed_values: None, is_path: false, is_url: false },
            ],
        });

        // open_url — opens system browser, confirmation optional
        policies.insert("open_url".into(), ToolPolicy {
            requires_confirm: false,
            high_risk_prompt: String::new(),
            path_sandbox_applies: false,
            domain_allowlist_applies: true,
            arg_rules: vec![
                ArgRule { field: "url".into(), required: true, max_len: Some(2048), max_value: None, allowed_values: None, is_path: false, is_url: true },
            ],
        });

        // send_email — high-risk: requires explicit confirmation
        policies.insert("send_email".into(), ToolPolicy {
            requires_confirm: true,
            high_risk_prompt: "Send an email on your behalf?".into(),
            path_sandbox_applies: false,
            domain_allowlist_applies: false,
            arg_rules: vec![
                ArgRule { field: "to".into(), required: true, max_len: Some(512), max_value: None, allowed_values: None, is_path: false, is_url: false },
                ArgRule { field: "subject".into(), required: true, max_len: Some(512), max_value: None, allowed_values: None, is_path: false, is_url: false },
                ArgRule { field: "body".into(), required: true, max_len: Some(65536), max_value: None, allowed_values: None, is_path: false, is_url: false },
            ],
        });

        // run_shell_command — highest risk, always requires confirmation
        policies.insert("run_command".into(), ToolPolicy {
            requires_confirm: true,
            high_risk_prompt: "Run a shell command on your system?".into(),
            path_sandbox_applies: false,
            domain_allowlist_applies: false,
            arg_rules: vec![
                ArgRule { field: "command".into(), required: true, max_len: Some(4096), max_value: None, allowed_values: None, is_path: false, is_url: false },
            ],
        });

        PolicyEngine { default_policies: policies }
    }

    /// Evaluate whether a tool call should be allowed, denied, or requires user confirmation.
    /// `profile_permissions` is the `tool_permissions` JSON from AssistantProfile.
    pub fn evaluate(
        &self,
        tool: &str,
        args: &Value,
        profile_permissions: &Value,
    ) -> PolicyDecision {
        self.evaluate_with_risk(tool, args, profile_permissions, None)
    }

    /// Evaluate policy with an advisory tool risk ceiling.
    ///
    /// The advisory ceiling can only make decisions stricter (escalate from
    /// Allow -> RequireConfirmation). It can never weaken Deny/Confirmation.
    pub fn evaluate_with_risk(
        &self,
        tool: &str,
        args: &Value,
        profile_permissions: &Value,
        advisory_max_risk: Option<RiskLevel>,
    ) -> PolicyDecision {
        // 0. Offline strict mode: deny all network tools
        const NETWORK_TOOLS: &[&str] = &["fetch_url", "get_weather", "send_email", "open_url"];
        if profile_permissions.get("offline_strict_mode") == Some(&Value::Bool(true))
            && NETWORK_TOOLS.contains(&tool)
        {
            return PolicyDecision::Deny(format!(
                "Tool '{tool}' is disabled: offline strict mode is active."
            ));
        }

        // 1. Check profile-level permission toggle
        if let Some(perms) = profile_permissions.as_object() {
            if let Some(enabled) = perms.get(tool) {
                if enabled == &Value::Bool(false) {
                    return PolicyDecision::Deny(format!("Tool '{tool}' is disabled in your assistant profile."));
                }
            }
        }

        // 2. Look up default policy
        let policy = match self.default_policies.get(tool) {
            Some(p) => p,
            None => {
                // Unknown tools are denied by default
                return PolicyDecision::Deny(format!("Tool '{tool}' is not registered in the policy engine."));
            }
        };

        // 3. Validate arguments
        if let Err(e) = self.validate_args(tool, args, &policy.arg_rules) {
            return PolicyDecision::Deny(format!("Invalid arguments for '{tool}': {e}"));
        }

        // 4. Require confirmation if policy says so
        if policy.requires_confirm {
            let prompt = if policy.high_risk_prompt.is_empty() {
                format!("Allow '{tool}' to execute?")
            } else {
                policy.high_risk_prompt.clone()
            };
            return self.apply_advisory_ceiling(tool, PolicyDecision::RequireConfirmation(prompt), advisory_max_risk);
        }

        self.apply_advisory_ceiling(tool, PolicyDecision::Allow, advisory_max_risk)
    }

    fn apply_advisory_ceiling(
        &self,
        tool: &str,
        decision: PolicyDecision,
        advisory_max_risk: Option<RiskLevel>,
    ) -> PolicyDecision {
        let Some(risk) = advisory_max_risk else {
            return decision;
        };

        // Never deescalate policy outcomes.
        match decision {
            PolicyDecision::Deny(_) | PolicyDecision::RequireConfirmation(_) => decision,
            PolicyDecision::Allow => {
                match risk {
                    RiskLevel::Safe => PolicyDecision::Allow,
                    RiskLevel::LowRisk => PolicyDecision::RequireConfirmation(
                        format!("Allow '{tool}' to execute? (advisory risk: low)"),
                    ),
                    RiskLevel::Destructive => PolicyDecision::RequireConfirmation(
                        format!("Allow '{tool}' to execute? (advisory risk: destructive)"),
                    ),
                    RiskLevel::AlwaysConfirm => PolicyDecision::RequireConfirmation(
                        format!("Allow '{tool}' to execute? (advisory risk: always confirm)"),
                    ),
                }
            }
        }
    }

    fn validate_args(&self, _tool: &str, args: &Value, rules: &[ArgRule]) -> Result<(), String> {
        for rule in rules {
            let val = args.get(&rule.field);

            // Required check
            if rule.required && val.is_none() {
                return Err(format!("Required field '{}' is missing", rule.field));
            }

            if let Some(v) = val {
                // Max length for strings
                if let Some(max) = rule.max_len {
                    if let Some(s) = v.as_str() {
                        if s.len() > max {
                            return Err(format!("Field '{}' exceeds max length {max}", rule.field));
                        }
                    }
                }

                // Max value for numbers
                if let Some(max) = rule.max_value {
                    if let Some(n) = v.as_f64() {
                        if n > max {
                            return Err(format!("Field '{}' exceeds max value {max}", rule.field));
                        }
                    }
                }

                // Allowed values check
                if let Some(allowed) = &rule.allowed_values {
                    if let Some(s) = v.as_str() {
                        if !allowed.iter().any(|a| a == s) {
                            return Err(format!(
                                "Field '{}' must be one of: {}",
                                rule.field,
                                allowed.join(", ")
                            ));
                        }
                    }
                }

                // Path traversal guard using component analysis (robust against encoded variants)
                if rule.is_path {
                    if let Some(s) = v.as_str() {
                        if std::path::Path::new(s).components().any(|c| {
                            c == std::path::Component::ParentDir
                        }) {
                            return Err(format!("Field '{}' contains path traversal sequences", rule.field));
                        }
                    }
                }

                // Basic URL scheme check for URL fields
                if rule.is_url {
                    if let Some(s) = v.as_str() {
                        if !s.starts_with("https://") && !s.starts_with("http://") {
                            return Err(format!("Field '{}' must start with http:// or https://", rule.field));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn is_path_sandbox_tool(&self, tool: &str) -> bool {
        self.default_policies.get(tool).map(|p| p.path_sandbox_applies).unwrap_or(false)
    }

    pub fn is_domain_restricted_tool(&self, tool: &str) -> bool {
        self.default_policies.get(tool).map(|p| p.domain_allowlist_applies).unwrap_or(false)
    }
}

// ── Pending confirmation tokens ───────────────────────────────────────────────

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ConfirmationGate {
    pending: Mutex<HashMap<String, PendingConfirm>>,
}

struct PendingConfirm {
    tool:       String,
    args:       Value,
    expires_at: u64,
}

impl ConfirmationGate {
    pub fn new() -> Self {
        ConfirmationGate { pending: Mutex::new(HashMap::new()) }
    }

    /// Register a pending confirmation. Returns the single-use token.
    pub fn register(&self, tool: &str, args: Value) -> String {
        use rand::distributions::Alphanumeric;
        use rand::Rng;
        let token: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        let expires_at = now_secs() + 60; // 60s TTL
        self.pending.lock().unwrap().insert(token.clone(), PendingConfirm {
            tool: tool.to_string(),
            args,
            expires_at,
        });
        token
    }

    /// Consume a confirmation token. Returns (tool, args) if valid, else Err.
    pub fn consume(&self, token: &str) -> Result<(String, Value), String> {
        let mut map = self.pending.lock().unwrap();
        // Purge expired entries
        let now = now_secs();
        map.retain(|_, v| v.expires_at > now);

        match map.remove(token) {
            Some(c) if c.expires_at > now => Ok((c.tool, c.args)),
            Some(_) => Err("Confirmation token has expired.".into()),
            None    => Err("Unknown or already-used confirmation token.".into()),
        }
    }

    /// Cancel a pending confirmation.
    pub fn cancel(&self, token: &str) {
        self.pending.lock().unwrap().remove(token);
    }
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn advisory_risk_escalates_allow_to_confirmation() {
        let engine = PolicyEngine::new();
        let args = json!({ "location": "Zurich" });
        let perms = json!({});

        let decision = engine.evaluate_with_risk(
            "get_weather",
            &args,
            &perms,
            Some(RiskLevel::AlwaysConfirm),
        );

        assert!(matches!(decision, PolicyDecision::RequireConfirmation(_)));
    }

    #[test]
    fn advisory_risk_does_not_override_deny() {
        let engine = PolicyEngine::new();
        let args = json!({ "location": "Zurich" });
        let perms = json!({ "get_weather": false });

        let decision = engine.evaluate_with_risk(
            "get_weather",
            &args,
            &perms,
            Some(RiskLevel::Safe),
        );

        assert!(matches!(decision, PolicyDecision::Deny(_)));
    }

    #[test]
    fn advisory_risk_does_not_deescalate_existing_confirmation() {
        let engine = PolicyEngine::new();
        let args = json!({
            "path": "C:/tmp/test.txt",
            "content": "hello"
        });
        let perms = json!({});

        let decision = engine.evaluate_with_risk(
            "write_file_assistant",
            &args,
            &perms,
            Some(RiskLevel::Safe),
        );

        assert!(matches!(decision, PolicyDecision::RequireConfirmation(_)));
    }
}
