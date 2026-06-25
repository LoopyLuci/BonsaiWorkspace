//! Declarative tool composition DSL — YAML/JSON pipeline skill kind.
//!
//! Extends `user_skills.rs` with a `composed` skill kind that lets users build
//! multi-tool pipelines without writing code. Pipelines support:
//!   - Sequential and conditional steps
//!   - Output capture (`output_as`) and variable interpolation (`{variable}`)
//!   - Error handling (`on_error`: continue | abort | retry)
//!   - Loops over arrays (`foreach`)
//!   - Tool calls via the assistant `ToolRegistry`
//!
//! ## Example skill body (YAML)
//! ```yaml
//! steps:
//!   - tool: read_file
//!     args:
//!       path: "{file}"
//!     output_as: source
//!
//!   - tool: grep_files
//!     args:
//!       path: "{workspace}"
//!       pattern: "TODO|FIXME"
//!     output_as: todos
//!     condition: "source != ''"
//!
//!   - tool: remember
//!     args:
//!       key: "last_todos"
//!       value: "{todos}"
//!     on_error: continue
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::tool_core::{ToolContext, ToolOutput, ToolRegistry};

// ── DSL schema types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OnError {
    Abort,
    Continue,
    Retry,
}

impl Default for OnError {
    fn default() -> Self {
        Self::Abort
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ComposedStep {
    /// Tool name from the registry.
    pub tool: String,
    /// JSON object whose string values may contain `{variable}` interpolations.
    #[serde(default)]
    pub args: Value,
    /// Store this step's output into a named variable.
    #[serde(default)]
    pub output_as: Option<String>,
    /// Only execute this step if this expression is truthy.
    /// Supported: `"{var} != ''"`, `"{var.field} > 0"`, bare variable name.
    #[serde(default)]
    pub condition: Option<String>,
    /// Iterate over each element of an array variable, running this step once per element.
    /// The element is bound to `{item}`.
    #[serde(default)]
    pub foreach: Option<String>,
    /// What to do when the tool returns an error. Default: abort.
    #[serde(default)]
    pub on_error: OnError,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ComposedPipeline {
    pub steps: Vec<ComposedStep>,
}

impl ComposedPipeline {
    /// Parse a pipeline from YAML or JSON body text.
    pub fn parse(body: &str) -> Result<Self, String> {
        // Try YAML first (superset of JSON)
        serde_yaml::from_str(body)
            .or_else(|_| serde_json::from_str(body).map_err(|e| e.to_string()))
            .map_err(|e| format!("Pipeline parse error: {e}"))
    }
}

// ── Execution result ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct PipelineResult {
    pub steps_run: usize,
    pub steps_skipped: usize,
    pub steps_failed: usize,
    /// Final accumulated variable scope after all steps.
    pub output: HashMap<String, Value>,
    pub errors: Vec<String>,
}

// ── Variable interpolation ────────────────────────────────────────────────────

/// Replace `{variable}` and `{variable.field}` references in a string.
fn interpolate(template: &str, vars: &HashMap<String, Value>) -> String {
    let mut result = template.to_string();
    for (key, val) in vars {
        let placeholder = format!("{{{key}}}");
        let replacement = match val {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        result = result.replace(&placeholder, &replacement);
    }
    // Also handle dot-notation: {var.field}
    let re_dot =
        regex::Regex::new(r"\{([a-zA-Z_][a-zA-Z0-9_]*)\.([a-zA-Z_][a-zA-Z0-9_]*)\}").unwrap();
    result = re_dot
        .replace_all(&result, |caps: &regex::Captures| {
            let var = &caps[1];
            let field = &caps[2];
            vars.get(var)
                .and_then(|v| v.get(field))
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .unwrap_or_else(|| caps[0].to_string())
        })
        .to_string();
    result
}

/// Recursively interpolate all string values in a JSON value.
fn interpolate_value(v: &Value, vars: &HashMap<String, Value>) -> Value {
    match v {
        Value::String(s) => Value::String(interpolate(s, vars)),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), interpolate_value(v, vars)))
                .collect(),
        ),
        Value::Array(arr) => Value::Array(arr.iter().map(|v| interpolate_value(v, vars)).collect()),
        other => other.clone(),
    }
}

// ── Condition evaluation ──────────────────────────────────────────────────────

/// Evaluate a simple condition expression against the variable scope.
/// Supported forms:
///   `{var} != ''`        — variable is non-empty string
///   `{var} == 'value'`   — string equality
///   `varname`            — variable is truthy (non-null, non-empty, non-false)
fn eval_condition(expr: &str, vars: &HashMap<String, Value>) -> bool {
    let interpolated = interpolate(expr.trim(), vars);
    // Simple equality checks
    if let Some((left, right)) = interpolated.split_once(" != ") {
        return left.trim().trim_matches('\'') != right.trim().trim_matches('\'');
    }
    if let Some((left, right)) = interpolated.split_once(" == ") {
        return left.trim().trim_matches('\'') == right.trim().trim_matches('\'');
    }
    if let Some((left, right)) = interpolated.split_once(" > ") {
        let l: f64 = left.trim().parse().unwrap_or(0.0);
        let r: f64 = right.trim().parse().unwrap_or(0.0);
        return l > r;
    }
    // Bare variable: check if truthy
    let original = expr.trim().trim_matches('{').trim_matches('}');
    match vars.get(original) {
        None => false,
        Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0) != 0.0,
    }
}

// ── Pipeline executor ─────────────────────────────────────────────────────────

pub struct PipelineExecutor {
    registry: Arc<RwLock<ToolRegistry>>,
}

impl PipelineExecutor {
    pub fn new(registry: Arc<RwLock<ToolRegistry>>) -> Self {
        Self { registry }
    }

    /// Execute a `ComposedPipeline` with the given initial variable bindings and context.
    pub async fn run(
        &self,
        pipeline: &ComposedPipeline,
        initial_vars: HashMap<String, Value>,
        ctx: &ToolContext,
    ) -> PipelineResult {
        let mut vars = initial_vars;
        let mut steps_run = 0usize;
        let mut steps_skipped = 0usize;
        let mut steps_failed = 0usize;
        let mut errors = Vec::new();

        for step in &pipeline.steps {
            if ctx.is_cancelled() {
                warn!("[tool_compose] pipeline cancelled");
                break;
            }

            // Check condition
            if let Some(cond) = &step.condition {
                if !eval_condition(cond, &vars) {
                    debug!(tool=%step.tool, cond=%cond, "[tool_compose] step skipped (condition false)");
                    steps_skipped += 1;
                    continue;
                }
            }

            // Foreach loop
            if let Some(foreach_var) = &step.foreach {
                let items = vars
                    .get(foreach_var)
                    .cloned()
                    .unwrap_or(Value::Array(vec![]));
                let arr = match items {
                    Value::Array(a) => a,
                    other => vec![other],
                };
                for item in arr {
                    if ctx.is_cancelled() {
                        break;
                    }
                    let mut loop_vars = vars.clone();
                    loop_vars.insert("item".into(), item);
                    let step_result = self.run_step(step, &loop_vars, ctx).await;
                    match step_result {
                        Ok(output) => {
                            steps_run += 1;
                            if let Some(ref name) = step.output_as {
                                vars.insert(name.clone(), output);
                            }
                        }
                        Err(e) => {
                            steps_failed += 1;
                            errors.push(e.clone());
                            if matches!(step.on_error, OnError::Abort) {
                                return PipelineResult {
                                    steps_run,
                                    steps_skipped,
                                    steps_failed,
                                    output: vars,
                                    errors,
                                };
                            }
                        }
                    }
                }
                continue;
            }

            // Single step
            let step_result = self.run_step(step, &vars, ctx).await;
            match step_result {
                Ok(output) => {
                    steps_run += 1;
                    if let Some(ref name) = step.output_as {
                        vars.insert(name.clone(), output);
                    }
                }
                Err(e) => {
                    steps_failed += 1;
                    errors.push(e.clone());
                    match step.on_error {
                        OnError::Abort => break,
                        OnError::Continue => continue,
                        OnError::Retry => {
                            // One retry attempt
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            match self.run_step(step, &vars, ctx).await {
                                Ok(output) => {
                                    if let Some(ref name) = step.output_as {
                                        vars.insert(name.clone(), output);
                                    }
                                }
                                Err(e2) => {
                                    errors.push(e2);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        PipelineResult {
            steps_run,
            steps_skipped,
            steps_failed,
            output: vars,
            errors,
        }
    }

    async fn run_step(
        &self,
        step: &ComposedStep,
        vars: &HashMap<String, Value>,
        ctx: &ToolContext,
    ) -> Result<Value, String> {
        let reg = self.registry.read().await;
        let tool = reg
            .get(&step.tool)
            .ok_or_else(|| format!("Tool '{}' not found in registry", step.tool))?;

        let args = interpolate_value(&step.args, vars);
        debug!(tool=%step.tool, "[tool_compose] executing step");

        match tool.execute(&args, ctx).await {
            Ok(ToolOutput::Complete(v)) => Ok(v),
            Ok(ToolOutput::Streaming(_)) => Ok(json!({ "streaming": true })),
            Err(e) => Err(format!("Step '{}' failed: {}", step.tool, e)),
        }
    }
}

// ── Tauri command ─────────────────────────────────────────────────────────────

/// Validate and dry-run a composed pipeline body (no tool execution).
/// Returns the parsed step list so the UI can show a preview.
#[tauri::command]
pub async fn validate_composed_skill(body: String) -> Result<Vec<String>, String> {
    let pipeline = ComposedPipeline::parse(&body)?;
    Ok(pipeline
        .steps
        .iter()
        .map(|s| {
            format!(
                "{}{}",
                s.tool,
                s.condition
                    .as_ref()
                    .map(|c| format!(" [if {c}]"))
                    .unwrap_or_default()
            )
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolate_simple_variable() {
        let mut vars = HashMap::new();
        vars.insert("path".into(), Value::String("/home/user/file.rs".into()));
        assert_eq!(
            interpolate("Read {path} please", &vars),
            "Read /home/user/file.rs please"
        );
    }

    #[test]
    fn interpolate_dot_notation() {
        let mut vars = HashMap::new();
        vars.insert("result".into(), json!({ "text": "hello world" }));
        assert_eq!(interpolate("{result.text}", &vars), "hello world");
    }

    #[test]
    fn condition_ne_empty() {
        let mut vars = HashMap::new();
        vars.insert("source".into(), Value::String("some code".into()));
        assert!(eval_condition("{source} != ''", &vars));
        vars.insert("source".into(), Value::String(String::new()));
        assert!(!eval_condition("{source} != ''", &vars));
    }

    #[test]
    fn condition_bare_variable() {
        let mut vars = HashMap::new();
        vars.insert("flag".into(), Value::Bool(true));
        assert!(eval_condition("flag", &vars));
        vars.insert("flag".into(), Value::Bool(false));
        assert!(!eval_condition("flag", &vars));
    }

    #[test]
    fn parse_yaml_pipeline() {
        let yaml = r#"
steps:
  - tool: read_file
    args:
      path: "{file}"
    output_as: content
  - tool: remember
    args:
      key: "last_read"
      value: "{content}"
    on_error: continue
"#;
        let pipeline = ComposedPipeline::parse(yaml).unwrap();
        assert_eq!(pipeline.steps.len(), 2);
        assert_eq!(pipeline.steps[0].tool, "read_file");
        assert_eq!(
            pipeline.steps[1].on_error.clone() as u8,
            OnError::Continue as u8
        );
    }
}
