/// ReAct (Reason + Act) inference loop — next-gen edition.
///
/// Changes from v1:
/// - Tool dispatch goes through ToolRegistry (trait objects), not a match arm.
/// - Parallel execution: side-effect-free tools in a single turn are run
///   concurrently via join_all; External/Write tools remain serial + gated.
/// - Real SSE streaming: llama-server is called with stream:true; tokens are
///   emitted per-delta rather than collected into a String first.
/// - Structured ToolError taxonomy with per-error retry logic.
/// - ToolContext carries workspace_path, profile_id, session_id, turn_id,
///   call_depth, cancel flag, and secrets — eliminating naked param threading.
/// - Tool results are summarized to ≤200 tokens if over 2KB to protect
///   the context window.
/// - Top-K tool selection via ToolSelector; unknown-tool fallback expands
///   the candidate set and retries once.
use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use std::sync::OnceLock;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use futures::StreamExt;

use crate::assistant_policy::{ConfirmationGate, PolicyDecision, PolicyEngine};
use crate::assistant_store::{AssistantMessage, AssistantProfile, AssistantStore};
use crate::assistant_audit_log::AuditLog;
use crate::model_orchestrator::ModelOrchestrator;
use crate::secrets_store::SecretsStore;
use crate::tool_core::{
    SideEffectProfile, ToolCallOutcome, ToolContext, ToolError, ToolOutput,
    ToolRegistry,
};
use crate::tool_selector::ToolSelector;
use crate::tool_cache::ToolCache;

const MAX_REACT_ITERATIONS: usize = 10;
const MAX_TOKENS: u32 = 2048;
/// Max tool result bytes before we summarize rather than dump raw into context.
const RESULT_SUMMARIZE_THRESHOLD: usize = 2048;
/// Top-K tools selected per turn by the semantic selector.
const TOOL_SELECTOR_TOP_K: usize = 8;
/// Always inject these tools regardless of selector score.
const ALWAYS_INJECT: &[&str] = &["get_datetime", "get_system_stats"];

const QUESTION_PREFIXES: &[&str] = &[
    "what", "why", "how", "when", "where", "who", "which", "explain", "describe", "compare",
];

const TOOL_ACTION_HINTS: &[&str] = &[
    "read", "open", "find", "search", "list", "write", "edit", "update", "create", "delete", "remove",
    "run", "execute", "command", "file", "folder", "directory", "url", "http", "email", "send",
    "weather", "time", "date", "system stats", "cpu", "memory",
];

const TARGET_HINTS: &[&str] = &[
    "http://", "https://", "/", "\\", ".rs", ".ts", ".js", ".json", ".md", "file", "folder", "path",
    "command", "email", "weather", "time", "date", "system",
];

const LIVE_DATA_HINTS: &[&str] = &[
    "current time", "time now", "date today", "weather", "forecast",
    "system cpu", "cpu usage", "memory usage", "ram usage", "disk usage", "system stats",
    // system specs / hardware queries
    "system spec", "system info", "system information", "hardware info", "hardware spec",
    "my computer", "my machine", "computer spec", "machine spec", "os version",
    "operating system", "processor info", "my specs", "disk space", "free space",
    "how much ram", "how much memory", "how much disk", "what os", "what cpu",
    "what are my", "tell me my", "show my system", "check my",
];

const SYSTEM_STATS_HINTS: &[&str] = &[
    "cpu", "memory", "ram", "system stats", "disk usage", "system usage",
    "system spec", "system info", "system information", "hardware",
    "my computer", "my machine", "operating system", "os version", "processor",
    "my specs", "disk space", "free space", "how much ram", "how much memory",
    "storage", "drives", "what os", "what cpu", "computer info", "machine info",
    "what are my", "tell me my", "show my system",
];
const DATETIME_HINTS: &[&str] = &["time", "date", "clock", "what time", "today", "now"];
const WEATHER_HINTS: &[&str] = &["weather", "forecast", "temperature", "rain", "wind", "humidity"];
const FILE_READ_HINTS: &[&str] = &["read file", "open file", "show file", "file contents", "cat file"];
const FILE_LIST_HINTS: &[&str] = &["find file", "find files", "list files", "search files", "locate file", "directory listing"];
const URL_FETCH_HINTS: &[&str] = &["fetch", "url", "web page", "website", "http://", "https://"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntentMode {
    AnswerOnly,
    ToolPreferred,
    Hybrid,
    ClarifyFirst,
}

#[derive(Debug, Clone)]
struct IntentDecision {
    mode: IntentMode,
    tool_top_k: usize,
    rationale: String,
    clarifying_question: Option<String>,
}

// Prompt-injection heuristics — abort tool dispatch if found in model's reasoning.
const INJECTION_PATTERNS: &[&str] = &[
    "ignore previous instructions", "ignore all previous", "disregard previous",
    "system:", "SYSTEM:", "<|system|>", "you are now", "new instruction:", "override:",
];

fn contains_injection(text: &str) -> bool {
    let lower = text.to_lowercase();
    INJECTION_PATTERNS.iter().any(|p| lower.contains(&p.to_lowercase()))
}

fn should_expand_for_non_injected(tool_name: &str, injected_names: &[String]) -> bool {
    !injected_names.iter().any(|n| n == tool_name)
}

fn queue_expansion_tool(expand_names: &mut Vec<String>, tool_name: &str) {
    if !expand_names.iter().any(|n| n == tool_name) {
        expand_names.push(tool_name.to_string());
    }
}

fn should_retry_with_expansion(expand_names: &[String], iter: usize) -> bool {
    !expand_names.is_empty() && iter < MAX_REACT_ITERATIONS - 1
}

fn cache_enabled_for_tool(tool: &Arc<dyn crate::tool_core::Tool>) -> Option<u64> {
    tool.cache_ttl_secs().filter(|ttl| *ttl > 0)
}

fn normalize_text(s: &str) -> String {
    s.trim().to_lowercase()
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| text.contains(n))
}

fn starts_with_question_prefix(text: &str) -> bool {
    QUESTION_PREFIXES.iter().any(|p| text.starts_with(&format!("{p} ")))
}

fn has_path_like_target(text: &str) -> bool {
    let has_quotes = text.contains('"') || text.contains('\'');
    let has_ext = text.split_whitespace().any(|w| w.contains('.'));
    has_quotes || has_ext || contains_any(text, TARGET_HINTS)
}

fn build_clarifying_question(user_text: &str) -> String {
    let mut asks = vec![
        "What exact target should I operate on (file/path/URL/command/model)?",
        "What output do you want (inspect, edit, create, delete, run, summarize)?",
        "Any constraints I must respect (scope, format, safety limits)?",
    ];

    let lower = normalize_text(user_text);
    if lower.contains("delete") || lower.contains("remove") {
        asks.push("If this is destructive, confirm whether I should proceed once target is identified.");
    }

    format!(
        "I can do that. Before I call tools, I need a bit more context:\n1. {}\n2. {}\n3. {}{}",
        asks[0],
        asks[1],
        asks[2],
        if asks.len() > 3 { format!("\n4. {}", asks[3]) } else { String::new() }
    )
}

fn analyze_intent(last_user_text: &str, history: &[Value]) -> IntentDecision {
    let text = normalize_text(last_user_text);
    let has_question_mark = text.contains('?');
    let question_like = has_question_mark || starts_with_question_prefix(&text)
        || text.starts_with("can you explain") || text.starts_with("help me understand")
        || text.starts_with("what is") || text.starts_with("how to");

    let action_like = contains_any(&text, TOOL_ACTION_HINTS);
    let live_data_like = contains_any(&text, LIVE_DATA_HINTS);
    let explicit_target = has_path_like_target(&text);
    let ambiguous_reference = text.contains(" this ") || text.contains(" that ") || text.contains(" it ");
    let has_prior_context = history.len() > 2;

    if action_like && !explicit_target && (!has_prior_context || ambiguous_reference) {
        return IntentDecision {
            mode: IntentMode::ClarifyFirst,
            tool_top_k: 0,
            rationale: "action intent detected without enough concrete target context".to_string(),
            clarifying_question: Some(build_clarifying_question(last_user_text)),
        };
    }

    if live_data_like {
        return IntentDecision {
            mode: IntentMode::ToolPreferred,
            tool_top_k: TOOL_SELECTOR_TOP_K,
            rationale: "live-data query detected: prefer trusted tool retrieval over model-only answer".to_string(),
            clarifying_question: None,
        };
    }

    if question_like && !action_like {
        return IntentDecision {
            mode: IntentMode::AnswerOnly,
            tool_top_k: 0,
            rationale: "question-first intent: prefer direct answer over tool calls".to_string(),
            clarifying_question: None,
        };
    }

    if action_like && question_like {
        return IntentDecision {
            mode: IntentMode::Hybrid,
            tool_top_k: TOOL_SELECTOR_TOP_K.saturating_sub(2).max(3),
            rationale: "mixed intent: answer + optional tools".to_string(),
            clarifying_question: None,
        };
    }

    if action_like {
        return IntentDecision {
            mode: IntentMode::ToolPreferred,
            tool_top_k: TOOL_SELECTOR_TOP_K,
            rationale: "task execution intent: enable focused tool calling".to_string(),
            clarifying_question: None,
        };
    }

    IntentDecision {
        mode: IntentMode::AnswerOnly,
        tool_top_k: 0,
        rationale: "default to conversational answer unless tool need is explicit".to_string(),
        clarifying_question: None,
    }
}

fn protocol_message_for_intent(intent: &IntentDecision) -> String {
    let mode_hint = match intent.mode {
        IntentMode::AnswerOnly => "The user is asking a conceptual question. Answer directly and avoid tools unless strictly required.",
        IntentMode::ToolPreferred => "The user likely wants concrete actions. Use tools only when needed and keep actions minimal.",
        IntentMode::Hybrid => "The user may need both explanation and actions. Explain first, then use tools if they improve accuracy.",
        IntentMode::ClarifyFirst => "The request is under-specified. Ask a concise clarifying question before any tool use.",
    };

    format!(
        "Context Recognition Protocol:\n- Distinguish intent: question vs task execution.\n- If context is insufficient for safe action, ask targeted clarifying questions first.\n- Do not call tools unless they materially improve correctness.\n- If tools are not needed, provide the best direct answer.\nMode hint: {mode_hint}\nRationale: {}",
        intent.rationale
    )
}

fn intent_mode_label(mode: IntentMode) -> &'static str {
    match mode {
        IntentMode::AnswerOnly => "answer_only",
        IntentMode::ToolPreferred => "tool_preferred",
        IntentMode::Hybrid => "hybrid",
        IntentMode::ClarifyFirst => "clarify_first",
    }
}

fn intent_confidence(intent: &IntentDecision) -> f32 {
    match intent.mode {
        IntentMode::ClarifyFirst => 0.95,
        IntentMode::ToolPreferred => 0.90,
        IntentMode::AnswerOnly => 0.85,
        IntentMode::Hybrid => 0.75,
    }
}

fn emit_intent_router_trace(
    app: &AppHandle,
    session_id: &str,
    turn_id: &str,
    intent: &IntentDecision,
    phase: &str,
    user_text: &str,
) {
    let preview = user_text.chars().take(180).collect::<String>();
    let _ = app.emit("intent-router-trace", json!({
        "session_id": session_id,
        "turn_id": turn_id,
        "phase": phase,
        "mode": intent_mode_label(intent.mode),
        "confidence": intent_confidence(intent),
        "tool_top_k": intent.tool_top_k,
        "rationale": intent.rationale,
        "has_clarifying_question": intent.clarifying_question.is_some(),
        "user_text_preview": preview,
    }));
}

fn extract_weather_location(text: &str) -> Option<String> {
    let lower = normalize_text(text);
    if let Some(idx) = lower.find(" in ") {
        let loc = text[idx + 4..].trim().trim_matches(&['?', '.', '!', ','][..]);
        if !loc.is_empty() {
            return Some(loc.to_string());
        }
    }
    None
}

fn extract_url(text: &str) -> Option<String> {
    text.split_whitespace()
        .find(|t| t.starts_with("http://") || t.starts_with("https://"))
        .map(|t| t.trim_matches(|c: char| c == ')' || c == ']' || c == '"' || c == '\'' || c == ',' || c == '.').to_string())
}

fn extract_quoted_segment(text: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let mut parts = text.split(quote);
        let _ = parts.next();
        if let Some(seg) = parts.next() {
            let s = seg.trim();
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }
    None
}

fn extract_path_like_token(text: &str) -> Option<String> {
    if let Some(q) = extract_quoted_segment(text) {
        return Some(q);
    }
    text.split_whitespace()
        .find(|t| {
            t.contains('\\') || t.contains('/') || t.ends_with(".rs") || t.ends_with(".ts")
                || t.ends_with(".js") || t.ends_with(".json") || t.ends_with(".md")
                || t.ends_with(".txt") || t.ends_with(".toml") || t.ends_with(".yaml") || t.ends_with(".yml")
        })
        .map(|t| t.trim_matches(|c: char| c == ')' || c == ']' || c == '"' || c == '\'' || c == ',' || c == '.').to_string())
}

fn extract_glob_pattern(text: &str) -> Option<String> {
    if let Some(q) = extract_quoted_segment(text) {
        if q.contains('*') {
            return Some(q);
        }
    }
    text.split_whitespace()
        .find(|t| t.contains('*') || t.starts_with("*.") || t.starts_with("**/"))
        .map(|t| t.trim_matches(|c: char| c == ')' || c == ']' || c == '"' || c == '\'' || c == ',' || c == '.').to_string())
}

fn infer_pattern_from_text(text: &str) -> String {
    let lower = normalize_text(text);
    if lower.contains("markdown") || lower.contains("md files") {
        return "*.md".to_string();
    }
    if lower.contains("json files") {
        return "*.json".to_string();
    }
    if lower.contains("text files") || lower.contains("txt files") {
        return "*.txt".to_string();
    }
    for ext in ["rs", "ts", "js", "json", "md", "txt", "toml", "yaml", "yml", "py", "java"] {
        if lower.contains(&format!(".{ext}")) || lower.contains(&format!(" {ext} files")) {
            return format!("*.{ext}");
        }
    }
    "*".to_string()
}

async fn try_model_independent_live_data_reply(
    intent: &IntentDecision,
    last_user_text: &str,
    registry: &ToolRegistry,
    ctx: &ToolContext,
    cache: &ToolCache,
    audit: &AuditLog,
    app: &AppHandle,
) -> Option<AssistantTurn> {
    if intent.mode != IntentMode::ToolPreferred {
        return None;
    }

    let lower = normalize_text(last_user_text);
    let (tool_name, args) = if contains_any(&lower, SYSTEM_STATS_HINTS) {
        ("get_system_stats", json!({}))
    } else if contains_any(&lower, DATETIME_HINTS) && !contains_any(&lower, WEATHER_HINTS) {
        ("get_datetime", json!({}))
    } else if contains_any(&lower, WEATHER_HINTS) {
        let location = extract_weather_location(last_user_text).unwrap_or_else(|| "auto".to_string());
        ("get_weather", json!({ "location": location }))
    } else {
        return None;
    };

    let tool = registry.get(tool_name)?;
    let args_str = args.to_string();
    let outcome = execute_single_with_retry(
        "intent_fastpath_0".to_string(),
        tool_name.to_string(),
        args,
        &args_str,
        tool,
        ctx,
        cache,
        audit,
        app,
    ).await;

    let reply = if outcome.decision == "allowed" {
        match serde_json::from_str::<Value>(&outcome.result_json) {
            Ok(v) => {
                match tool_name {
                    "get_system_stats" => {
                        let os      = v.get("os_name").and_then(|x| x.as_str()).unwrap_or("Unknown OS");
                        let os_ver  = v.get("os_version").and_then(|x| x.as_str()).unwrap_or("");
                        let arch    = v.get("architecture").and_then(|x| x.as_str()).unwrap_or("");
                        let host    = v.get("hostname").and_then(|x| x.as_str()).unwrap_or("");
                        let cpu_mod = v.get("cpu_model").and_then(|x| x.as_str()).unwrap_or("Unknown CPU");
                        let cpu_ph  = v.get("cpu_cores_physical").and_then(|x| x.as_u64()).unwrap_or(0);
                        let cpu_lg  = v.get("cpu_cores_logical").and_then(|x| x.as_u64()).unwrap_or(0);
                        let cpu_pct = v.get("cpu_usage_pct").and_then(|x| x.as_f64()).unwrap_or(0.0);
                        let mem_tot = v.get("memory_total_mb").and_then(|x| x.as_u64()).unwrap_or(0);
                        let mem_use = v.get("memory_used_mb").and_then(|x| x.as_u64()).unwrap_or(0);
                        let mem_pct = v.get("memory_used_pct").and_then(|x| x.as_f64()).unwrap_or(0.0);
                        let sw_tot  = v.get("swap_total_mb").and_then(|x| x.as_u64()).unwrap_or(0);
                        let sw_use  = v.get("swap_used_mb").and_then(|x| x.as_u64()).unwrap_or(0);

                        let mut s = format!(
                            "**System specs for {}:**\n\n\
                             **OS:** {} {} ({})\n\
                             **CPU:** {} — {} physical / {} logical cores — {}% usage\n\
                             **RAM:** {} MB / {} MB used ({}%)\n\
                             **Swap:** {} MB / {} MB used",
                            host, os, os_ver, arch,
                            cpu_mod, cpu_ph, cpu_lg, cpu_pct,
                            mem_use, mem_tot, mem_pct,
                            sw_use, sw_tot,
                        );

                        if let Some(disks) = v.get("disks").and_then(|x| x.as_array()) {
                            if !disks.is_empty() {
                                s.push_str("\n**Storage:**");
                                for d in disks {
                                    let name  = d.get("name").and_then(|x| x.as_str()).unwrap_or("?");
                                    let mount = d.get("mount").and_then(|x| x.as_str()).unwrap_or("?");
                                    let tot   = d.get("total_gb").and_then(|x| x.as_f64()).unwrap_or(0.0);
                                    let avail = d.get("available_gb").and_then(|x| x.as_f64()).unwrap_or(0.0);
                                    let pct   = d.get("used_pct").and_then(|x| x.as_f64()).unwrap_or(0.0);
                                    s.push_str(&format!(
                                        "\n  - {} ({}): {:.1} GB total, {:.1} GB free ({:.0}% used)",
                                        name, mount, tot, avail, pct
                                    ));
                                }
                            }
                        }
                        s
                    }
                    "get_datetime" => {
                        let dt = v.get("datetime").and_then(|x| x.as_str()).unwrap_or("unknown");
                        format!("Current date/time: {dt}")
                    }
                    "get_weather" => {
                        format!(
                            "Current weather for {}:\n- Temperature: {} C\n- Condition: {}\n- Wind: {} km/h\n- Humidity: {}%",
                            v.get("location").and_then(|x| x.as_str()).unwrap_or("unknown"),
                            v.get("temperature_c").and_then(|x| x.as_f64()).unwrap_or(0.0),
                            v.get("condition").and_then(|x| x.as_str()).unwrap_or("Unknown"),
                            v.get("wind_kmh").and_then(|x| x.as_f64()).unwrap_or(0.0),
                            v.get("humidity_pct").and_then(|x| x.as_f64()).unwrap_or(0.0),
                        )
                    }
                    _ => outcome.result_json.clone(),
                }
            }
            Err(_) => outcome.result_json.clone(),
        }
    } else {
        format!("I attempted to retrieve live data but hit an issue: {}", outcome.result_json)
    };

    Some(AssistantTurn {
        reply,
        outcomes: vec![outcome],
        confirm_token: None,
    })
}

async fn try_model_independent_read_ops_reply(
    intent: &IntentDecision,
    last_user_text: &str,
    registry: &ToolRegistry,
    ctx: &ToolContext,
    cache: &ToolCache,
    audit: &AuditLog,
    app: &AppHandle,
) -> Option<AssistantTurn> {
    if !matches!(intent.mode, IntentMode::ToolPreferred | IntentMode::Hybrid) {
        return None;
    }

    let lower = normalize_text(last_user_text);

    // URL fetch
    if contains_any(&lower, URL_FETCH_HINTS) && extract_url(last_user_text).is_some() {
        let url = extract_url(last_user_text)?;
        let args = json!({ "url": url, "strip_html": true, "max_bytes": 8192 });
        let outcome = execute_single_with_retry(
            "intent_fastpath_1".to_string(),
            "fetch_url".to_string(),
            args.clone(),
            &args.to_string(),
            registry.get("fetch_url")?,
            ctx,
            cache,
            audit,
            app,
        ).await;

        let reply = match serde_json::from_str::<Value>(&outcome.result_json) {
            Ok(v) => {
                let text = v.get("text").and_then(|x| x.as_str()).unwrap_or("");
                format!("Fetched URL successfully. Content preview:\n{}", &text[..text.len().min(1200)])
            }
            Err(_) => format!("Fetched URL. Raw result: {}", outcome.result_json),
        };
        return Some(AssistantTurn { reply, outcomes: vec![outcome], confirm_token: None });
    }

    // Read file
    if contains_any(&lower, FILE_READ_HINTS) || (lower.contains("read") && lower.contains("file")) {
        let Some(path) = extract_path_like_token(last_user_text) else {
            return Some(AssistantTurn {
                reply: "I can read it, but I need the exact file path first. Please provide the file path (quoted if it has spaces).".to_string(),
                outcomes: vec![],
                confirm_token: None,
            });
        };

        let args = json!({ "path": path, "max_bytes": 65536 });
        let outcome = execute_single_with_retry(
            "intent_fastpath_2".to_string(),
            "read_file_assistant".to_string(),
            args.clone(),
            &args.to_string(),
            registry.get("read_file_assistant")?,
            ctx,
            cache,
            audit,
            app,
        ).await;

        let reply = match serde_json::from_str::<Value>(&outcome.result_json) {
            Ok(v) => {
                let p = v.get("path").and_then(|x| x.as_str()).unwrap_or("(unknown)");
                let c = v.get("content").and_then(|x| x.as_str()).unwrap_or("");
                let truncated = v.get("truncated").and_then(|x| x.as_bool()).unwrap_or(false);
                format!(
                    "Read file: {p}\n{}{}",
                    &c[..c.len().min(1600)],
                    if truncated { "\n\n[Output truncated. Ask for a specific section if needed.]" } else { "" }
                )
            }
            Err(_) => format!("Read file result: {}", outcome.result_json),
        };
        return Some(AssistantTurn { reply, outcomes: vec![outcome], confirm_token: None });
    }

    // Find/list files
    if contains_any(&lower, FILE_LIST_HINTS) || (lower.contains("list") && lower.contains("files")) || (lower.contains("find") && lower.contains("files")) {
        let path = extract_path_like_token(last_user_text).unwrap_or_else(|| ".".to_string());
        let pattern = extract_glob_pattern(last_user_text).unwrap_or_else(|| infer_pattern_from_text(last_user_text));

        let args = json!({ "path": path, "pattern": pattern, "max_results": 50 });
        let outcome = execute_single_with_retry(
            "intent_fastpath_3".to_string(),
            "find_files".to_string(),
            args.clone(),
            &args.to_string(),
            registry.get("find_files")?,
            ctx,
            cache,
            audit,
            app,
        ).await;

        let reply = match serde_json::from_str::<Value>(&outcome.result_json) {
            Ok(v) => {
                let count = v.get("count").and_then(|x| x.as_u64()).unwrap_or(0);
                let files = v.get("files").and_then(|x| x.as_array()).cloned().unwrap_or_default();
                let preview = files.iter().take(20)
                    .filter_map(|f| f.as_str())
                    .collect::<Vec<_>>()
                    .join("\n- ");
                if preview.is_empty() {
                    format!("Found {count} matching files.")
                } else {
                    format!("Found {count} matching files:\n- {preview}")
                }
            }
            Err(_) => format!("File search result: {}", outcome.result_json),
        };
        return Some(AssistantTurn { reply, outcomes: vec![outcome], confirm_token: None });
    }

    if contains_any(&lower, URL_FETCH_HINTS) && extract_url(last_user_text).is_none() {
        return Some(AssistantTurn {
            reply: "I can fetch that page, but I need the full URL first (including http:// or https://).".to_string(),
            outcomes: vec![],
            confirm_token: None,
        });
    }

    None
}

// ── Public API ─────────────────────────────────────────────────────────────────

pub struct AssistantTurn {
    pub reply:         String,
    pub outcomes:      Vec<ToolCallOutcome>,
    /// Set when a tool call required user confirmation. Contains the single-use token,
    /// tool name, args, human-readable prompt, and expiry — so callers can surface
    /// a structured confirm request (e.g. bonsai-bot bonsai_ext envelope).
    pub confirm_token: Option<ConfirmRequest>,
}

pub struct ConfirmRequest {
    pub token:      String,
    pub tool:       String,
    pub args:       serde_json::Value,
    pub prompt:     String,
    pub expires_at: u64,
}

static ASSISTANT_REGISTRY: OnceLock<tokio::sync::RwLock<ToolRegistry>> = OnceLock::new();
static ASSISTANT_SELECTOR: OnceLock<ToolSelector> = OnceLock::new();
static ASSISTANT_CACHE: OnceLock<ToolCache> = OnceLock::new();

pub fn assistant_registry() -> &'static tokio::sync::RwLock<ToolRegistry> {
    ASSISTANT_REGISTRY.get_or_init(|| {
        tokio::sync::RwLock::new(crate::assistant_tools::build_registry())
    })
}

fn assistant_selector() -> &'static ToolSelector {
    ASSISTANT_SELECTOR.get_or_init(|| {
        // Build selector from a blocking snapshot; selector is rebuilt on reload via the registry
        let reg = crate::assistant_tools::build_registry();
        ToolSelector::build(&reg)
    })
}

fn assistant_cache() -> &'static ToolCache {
    ASSISTANT_CACHE.get_or_init(ToolCache::with_default_capacity)
}

/// Hot-reload user-defined skills into the running registry.
pub async fn reload_user_skills(store: &crate::user_skills::UserSkillStore) -> Result<usize, String> {
    let mut reg = assistant_registry().write().await;
    store.load_into_registry(&mut *reg).await
}

pub async fn run_assistant_turn(
    history:    Vec<Value>,
    profile:    &AssistantProfile,
    store:      &AssistantStore,
    policy:     &PolicyEngine,
    gate:       &ConfirmationGate,
    orch:       &ModelOrchestrator,
    secrets:    &Arc<SecretsStore>,
    audit:      &AuditLog,
    app:        &AppHandle,
    cancel:     Arc<AtomicBool>,
    stream_tx:  Option<mpsc::UnboundedSender<String>>,
    session_id: &str,
) -> Result<AssistantTurn, String> {
    let registry_lock = assistant_registry();
    let registry_guard = registry_lock.read().await;
    let registry = &*registry_guard;
    let selector = assistant_selector();
    let cache = assistant_cache();

    // Build ToolContext for this turn.
    let turn_id = {
        use rand::distributions::Alphanumeric;
        use rand::Rng;
        rand::thread_rng().sample_iter(&Alphanumeric).take(16).map(char::from).collect::<String>()
    };
    let ctx = ToolContext {
        workspace_path: None, // TODO: pass from AppState when workspace is known
        profile_id: profile.id.clone(),
        session_id: session_id.to_string(),
        turn_id: turn_id.clone(),
        call_depth: 0,
        cancel: cancel.clone(),
        secrets: secrets.clone(),
    };

    let profile_perms: Value = serde_json::from_str(&profile.tool_permissions)
        .unwrap_or_else(|_| json!({}));

    // Extract the last user message text for the selector.
    let last_user_text = history.iter().rev()
        .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
        .and_then(|m| m.get("content").and_then(|c| c.as_str()))
        .unwrap_or("")
        .to_string();

    let intent = analyze_intent(&last_user_text, &history);
    emit_intent_router_trace(
        app,
        session_id,
        &turn_id,
        &intent,
        "analyzed",
        &last_user_text,
    );
    audit.log_decision_with_context(
        "intent_router",
        intent_mode_label(intent.mode),
        &last_user_text,
        Some(intent.rationale.clone()),
        None,
        Some(session_id),
        Some(&turn_id),
        None,
    );

    if let Some(question) = intent.clarifying_question.clone() {
        emit_intent_router_trace(
            app,
            session_id,
            &turn_id,
            &intent,
            "clarify_first",
            &last_user_text,
        );
        return Ok(AssistantTurn {
            reply: question,
            outcomes: vec![],
            confirm_token: None,
        });
    }

    if let Some(turn) = try_model_independent_live_data_reply(
        &intent,
        &last_user_text,
        registry,
        &ctx,
        cache,
        audit,
        app,
    ).await {
        emit_intent_router_trace(
            app,
            session_id,
            &turn_id,
            &intent,
            "fast_path_live_data",
            &last_user_text,
        );
        return Ok(turn);
    }

    if let Some(turn) = try_model_independent_read_ops_reply(
        &intent,
        &last_user_text,
        registry,
        &ctx,
        cache,
        audit,
        app,
    ).await {
        emit_intent_router_trace(
            app,
            session_id,
            &turn_id,
            &intent,
            "fast_path_read_ops",
            &last_user_text,
        );
        return Ok(turn);
    }

    emit_intent_router_trace(
        app,
        session_id,
        &turn_id,
        &intent,
        "model_loop",
        &last_user_text,
    );

    let base_url = {
        let mut url = orch.active_slot_url().await;
        for _ in 0..3 {
            if url.is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            url = orch.active_slot_url().await;
        }

        if let Some(url) = url {
            url
        } else {
            let hint = orch
                .readiness_hint()
                .await
                .unwrap_or_else(|| "No model is currently loading. Load a model from Model Selector.".to_string());
            return Err(format!("No model slot is ready. {hint}"));
        }
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| e.to_string())?;

    let mut messages = history.clone();
    let protocol = protocol_message_for_intent(&intent);
    let protocol_msg = json!({ "role": "system", "content": protocol });
    if messages.first().and_then(|m| m.get("role").and_then(|r| r.as_str())) == Some("system") {
        messages.insert(1, protocol_msg);
    } else {
        messages.insert(0, protocol_msg);
    }
    let mut all_outcomes: Vec<ToolCallOutcome> = Vec::new();
    // Track which tool names were selected last turn for fallback expansion.
    let mut injected_names: Vec<String> = Vec::new();
    // Track tools that need expansion on next iteration (unknown-tool fallback).
    let mut expand_names: Vec<String> = Vec::new();

    for iter in 0..MAX_REACT_ITERATIONS {
        if cancel.load(Ordering::SeqCst) {
            return Err("Cancelled".into());
        }

        // ── Tool selection ───────────────────────────────────────────────────
        let mut selected = match intent.mode {
            IntentMode::AnswerOnly => Vec::new(),
            IntentMode::ToolPreferred => selector.select(&last_user_text, intent.tool_top_k, &injected_names),
            IntentMode::Hybrid => selector.select(&last_user_text, intent.tool_top_k, &injected_names),
            IntentMode::ClarifyFirst => Vec::new(),
        };

        // Add always-inject tools for all modes except ClarifyFirst; AnswerOnly still
        // benefits from get_datetime/get_system_stats if the LLM decides to use them.
        if !matches!(intent.mode, IntentMode::ClarifyFirst) {
            for name in ALWAYS_INJECT {
                if !selected.contains(&name.to_string()) { selected.push(name.to_string()); }
            }
        }
        // Add fallback-expansion tools from previous iteration
        for name in &expand_names {
            if !selected.contains(name) { selected.push(name.clone()); }
        }
        expand_names.clear();
        injected_names = selected.clone();

        let tool_defs = registry.definitions(Some(&selected), Some(&profile_perms));

        // ── Inference request (streaming) ────────────────────────────────────
        let body = json!({
            "model":       "local",
            "messages":    messages,
            "tools":       tool_defs,
            "tool_choice": if selected.is_empty() { "none" } else { "auto" },
            "max_tokens":  MAX_TOKENS,
            "temperature": 0.7,
            "stream":      true,
        });

        let resp = client
            .post(format!("{base_url}/v1/chat/completions"))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("llama-server: {e}"))?;

        if !resp.status().is_success() {
            let code = resp.status().as_u16();
            if code == 400 || code == 422 || code >= 500 {
                // Server doesn't support tools — fall back to plain turn
                return run_plain_turn(history, profile, orch, audit, app, cancel, stream_tx, session_id).await;
            }
            return Err(format!("llama-server HTTP {code}"));
        }

        // ── Stream accumulation ──────────────────────────────────────────────
        let (full_message, tool_calls_raw) = accumulate_stream(
            resp, app, &stream_tx, cancel.clone()
        ).await?;

        // Append assistant message to context
        messages.push(full_message.clone());

        // ── Tool calls? ──────────────────────────────────────────────────────
        if let Some(calls) = tool_calls_raw {
            let reasoning = full_message.get("content").and_then(|c| c.as_str()).unwrap_or("");

            // Separate into parallelizable vs. serial groups
            let mut parallel_group: Vec<(&Value, Arc<dyn crate::tool_core::Tool>)> = Vec::new();
            let mut parallel_effects: Vec<SideEffectProfile> = Vec::new();
            let mut serial_queue:   Vec<(&Value, Option<Arc<dyn crate::tool_core::Tool>>, PolicyDecision)> = Vec::new();

            for tc in &calls {
                let tool_name = tc["function"]["name"].as_str().unwrap_or("").to_string();
                let args_str  = tc["function"]["arguments"].as_str().unwrap_or("{}");
                let args: Value = serde_json::from_str(args_str).unwrap_or_else(|_| json!({}));

                // If the model references a valid tool that was not injected,
                // trigger one expansion retry instead of executing it immediately.
                if should_expand_for_non_injected(&tool_name, &injected_names) {
                    queue_expansion_tool(&mut expand_names, &tool_name);
                    let outcome = ToolCallOutcome {
                        tool_call_id: tc["id"].as_str().unwrap_or("").to_string(),
                        tool_name: tool_name.clone(),
                        args,
                        result_json: ToolError::NotInContext { tool_name: tool_name.clone() }.to_llm_message(),
                        decision: "not_in_context".into(),
                        duration_ms: 0,
                        from_cache: false,
                    };
                    messages.push(outcome.to_context_message());
                    all_outcomes.push(outcome);
                    continue;
                }

                // Prompt-injection guard
                if contains_injection(reasoning) {
                    let call_id = tc["id"].as_str().unwrap_or("");
                    audit.log_decision_with_context(
                        &tool_name,
                        "injection_blocked",
                        args_str,
                        Some("injection".into()),
                        None,
                        Some(&ctx.session_id),
                        Some(&ctx.turn_id),
                        Some(call_id),
                    );
                    let outcome = ToolCallOutcome {
                        tool_call_id: call_id.to_string(),
                        tool_name: tool_name.clone(),
                        args: args.clone(),
                        result_json: ToolError::InjectionBlocked.to_llm_message(),
                        decision: "injection_blocked".into(),
                        duration_ms: 0,
                        from_cache: false,
                    };
                    messages.push(outcome.to_context_message());
                    all_outcomes.push(outcome);
                    continue;
                }

                // Unknown-tool fallback
                let tool_arc = match registry.get(&tool_name) {
                    Some(t) => t,
                    None => {
                        let outcome = ToolCallOutcome {
                            tool_call_id: tc["id"].as_str().unwrap_or("").to_string(),
                            tool_name: tool_name.clone(),
                            args,
                            result_json: ToolError::NotInContext { tool_name: tool_name.clone() }.to_llm_message(),
                            decision: "not_in_context".into(),
                            duration_ms: 0,
                            from_cache: false,
                        };
                        messages.push(outcome.to_context_message());
                        all_outcomes.push(outcome);
                        continue;
                    }
                };

                let advisory_risk = Some(tool_arc.policy_hint().max_risk);
                let decision = policy.evaluate_with_risk(&tool_name, &args, &profile_perms, advisory_risk);

                match decision {
                    PolicyDecision::Allow => {
                        let side_effects = tool_arc.side_effects();
                        let compatible_with_group = parallel_effects.iter()
                            .all(|existing| side_effects.can_parallelize_with(existing));

                        if compatible_with_group {
                            parallel_effects.push(side_effects);
                            parallel_group.push((tc, tool_arc));
                        } else {
                            serial_queue.push((tc, Some(tool_arc), PolicyDecision::Allow));
                        }
                    }
                    other => {
                        serial_queue.push((tc, Some(tool_arc), other));
                    }
                }
            }

            // ── Execute parallel group ───────────────────────────────────────
            if !parallel_group.is_empty() {
                let par_outcomes = execute_parallel(
                    parallel_group, &ctx, cache, audit, app
                ).await;
                for o in &par_outcomes {
                    messages.push(o.to_context_message());
                }
                all_outcomes.extend(par_outcomes);
            }

            // ── Execute serial queue ─────────────────────────────────────────
            for (tc, tool_opt, decision) in serial_queue {
                if cancel.load(Ordering::SeqCst) { break; }
                let tool_name = tc["function"]["name"].as_str().unwrap_or("").to_string();
                let args_str  = tc["function"]["arguments"].as_str().unwrap_or("{}");
                let args: Value = serde_json::from_str(args_str).unwrap_or_else(|_| json!({}));
                let call_id   = tc["id"].as_str().unwrap_or("call_0").to_string();

                let outcome = match decision {
                    PolicyDecision::Deny(reason) => {
                        audit.log_decision_with_context(
                            &tool_name,
                            "denied",
                            args_str,
                            Some(reason.clone()),
                            None,
                            Some(&ctx.session_id),
                            Some(&ctx.turn_id),
                            Some(&call_id),
                        );
                        ToolCallOutcome {
                            tool_call_id: call_id,
                            tool_name,
                            args,
                            result_json: ToolError::PolicyDenied { reason }.to_llm_message(),
                            decision: "denied".into(),
                            duration_ms: 0,
                            from_cache: false,
                        }
                    }

                    PolicyDecision::RequireConfirmation(prompt) => {
                        let token = gate.register(&tool_name, args.clone());
                        let expires_at = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default().as_secs() + 120; // 2-minute TTL for bot confirmations
                        let _ = app.emit(
                            "assistant-confirm-required",
                            json!({
                                "token": token,
                                "tool": tool_name,
                                "prompt": prompt,
                                "expires_at": expires_at,
                                "session_id": ctx.session_id,
                                "turn_id": ctx.turn_id,
                                "tool_call_id": call_id,
                            }),
                        );
                        audit.log_decision_with_context(
                            &tool_name,
                            "confirm_required",
                            args_str,
                            None,
                            None,
                            Some(&ctx.session_id),
                            Some(&ctx.turn_id),
                            Some(&call_id),
                        );
                        // Return early; frontend or bot resubmits after approval
                        let reply = full_message["content"].as_str()
                            .unwrap_or("I need your confirmation to proceed.").to_string();
                        return Ok(AssistantTurn {
                            reply,
                            outcomes: all_outcomes,
                            confirm_token: Some(ConfirmRequest {
                                token,
                                tool:       tool_name.clone(),
                                args:       args.clone(),
                                prompt,
                                expires_at,
                            }),
                        });
                    }

                    PolicyDecision::Allow => {
                        let tool = tool_opt.unwrap();
                        execute_single_with_retry(
                            call_id, tool_name.clone(), args, args_str,
                            tool, &ctx, cache, audit, app
                        ).await
                    }
                };

                // Persist tool messages in session
                let _ = store.append_message(AssistantMessage {
                    id: String::new(),
                    session_id: session_id.to_string(),
                    role: "tool".into(),
                    content: outcome.result_json.clone(),
                    tool_name: Some(outcome.tool_name.clone()),
                    tool_result: None,
                    tts_synthesized: false,
                    created_at: 0,
                    tool_call_id: Some(outcome.tool_call_id.clone()),
                }).await;

                messages.push(outcome.to_context_message());
                all_outcomes.push(outcome);
            }

            // If we have expansion candidates, re-run immediately with expanded tools
            if should_retry_with_expansion(&expand_names, iter) {
                continue;
            }
            continue; // next ReAct iteration
        }

        // ── Plain text reply ─────────────────────────────────────────────────
        let reply = full_message["content"].as_str().unwrap_or("").to_string();
        return Ok(AssistantTurn { reply, outcomes: all_outcomes, confirm_token: None });
    }

    Err("Max ReAct iterations reached without a final answer.".into())
}

// ── Parallel executor ─────────────────────────────────────────────────────────

async fn execute_parallel(
    group:  Vec<(&Value, Arc<dyn crate::tool_core::Tool>)>,
    ctx:    &ToolContext,
    cache:  &ToolCache,
    audit:  &AuditLog,
    app:    &AppHandle,
) -> Vec<ToolCallOutcome> {
    let futures: Vec<_> = group.into_iter().map(|(tc, tool)| async move {
        let call_id   = tc["id"].as_str().unwrap_or("").to_string();
        let tool_name = tc["function"]["name"].as_str().unwrap_or("").to_string();
        let args_str  = tc["function"]["arguments"].as_str().unwrap_or("{}").to_string();
        let args: Value = serde_json::from_str(&args_str).unwrap_or_else(|_| json!({}));
        execute_single_with_retry(call_id, tool_name, args, &args_str, tool, ctx, cache, audit, app).await
    }).collect();

    futures::future::join_all(futures).await
}

async fn execute_single_with_retry(
    call_id:   String,
    tool_name: String,
    args:      Value,
    args_str:  &str,
    tool:      Arc<dyn crate::tool_core::Tool>,
    ctx:       &ToolContext,
    cache:     &ToolCache,
    audit:     &AuditLog,
    app:       &AppHandle,
) -> ToolCallOutcome {
    let cache_ttl = cache_enabled_for_tool(&tool);

    // Cache check only for explicitly cacheable tools.
    if cache_ttl.is_some() {
        if let Some(cached) = cache.get(&tool_name, &args, &ctx.profile_id, ctx.workspace_path.as_deref()) {
            audit.log_decision_with_context(
                &tool_name,
                "cache_hit",
                args_str,
                None,
                Some(0),
                Some(&ctx.session_id),
                Some(&ctx.turn_id),
                Some(&call_id),
            );
            return ToolCallOutcome {
                tool_call_id: call_id,
                tool_name,
                args,
                result_json: serde_json::to_string(&cached).unwrap_or_default(),
                decision: "allowed".into(),
                duration_ms: 0,
                from_cache: true,
            };
        }
    }

    let retry = tool.retry_policy();
    let mut last_outcome = None;

    for attempt in 0..retry.max_attempts {
        if ctx.is_cancelled() { break; }
        if attempt > 0 {
            let delay = retry.backoff_ms(attempt - 1);
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
        }

        let _ = app.emit(
            "assistant-tool-start",
            json!({
                "tool": tool_name,
                "args": args,
                "session_id": ctx.session_id,
                "turn_id": ctx.turn_id,
                "tool_call_id": call_id,
            }),
        );

        let t0 = std::time::Instant::now();
        let result = tool.execute(&args, ctx).await;
        let dur_ms = t0.elapsed().as_millis() as u64;

        match result {
            Ok(ToolOutput::Complete(val)) => {
                let result_str = maybe_summarize(&val, &tool_name);
                audit.log_decision_with_context(
                    &tool_name,
                    "allowed",
                    args_str,
                    None,
                    Some(dur_ms),
                    Some(&ctx.session_id),
                    Some(&ctx.turn_id),
                    Some(&call_id),
                );
                let _ = app.emit(
                    "assistant-tool-done",
                    json!({
                        "tool": tool_name,
                        "result": result_str,
                        "session_id": ctx.session_id,
                        "turn_id": ctx.turn_id,
                        "tool_call_id": call_id,
                    }),
                );
                // Cache the result
                if let Some(ttl) = cache_ttl {
                    cache.put(&tool_name, &args, &ctx.profile_id, ctx.workspace_path.as_deref(), val, ttl);
                }
                last_outcome = Some(ToolCallOutcome {
                    tool_call_id: call_id.clone(),
                    tool_name: tool_name.clone(),
                    args: args.clone(),
                    result_json: result_str,
                    decision: "allowed".into(),
                    duration_ms: dur_ms,
                    from_cache: false,
                });
                break;
            }

            Ok(ToolOutput::Streaming(mut rx)) => {
                // Drain the stream and collect
                let mut text = String::new();
                while let Some(chunk) = rx.recv().await {
                    text.push_str(&chunk.delta);
                    if chunk.done { break; }
                }
                let val = json!({ "output": text });
                let result_str = maybe_summarize(&val, &tool_name);
                audit.log_decision_with_context(
                    &tool_name,
                    "allowed",
                    args_str,
                    None,
                    Some(dur_ms),
                    Some(&ctx.session_id),
                    Some(&ctx.turn_id),
                    Some(&call_id),
                );
                last_outcome = Some(ToolCallOutcome {
                    tool_call_id: call_id.clone(),
                    tool_name: tool_name.clone(),
                    args: args.clone(),
                    result_json: result_str,
                    decision: "allowed".into(),
                    duration_ms: dur_ms,
                    from_cache: false,
                });
                break;
            }

            Err(err) if err.is_retryable() && attempt + 1 < retry.max_attempts => {
                audit.log_decision_with_context(
                    &tool_name,
                    "retrying",
                    args_str,
                    Some(err.to_string()),
                    Some(dur_ms),
                    Some(&ctx.session_id),
                    Some(&ctx.turn_id),
                    Some(&call_id),
                );
                last_outcome = Some(ToolCallOutcome {
                    tool_call_id: call_id.clone(),
                    tool_name: tool_name.clone(),
                    args: args.clone(),
                    result_json: err.to_llm_message(),
                    decision: "transient_error".into(),
                    duration_ms: dur_ms,
                    from_cache: false,
                });
                // continue loop
            }

            Err(err) => {
                audit.log_decision_with_context(
                    &tool_name,
                    "error",
                    args_str,
                    Some(err.to_string()),
                    Some(dur_ms),
                    Some(&ctx.session_id),
                    Some(&ctx.turn_id),
                    Some(&call_id),
                );
                let _ = app.emit(
                    "assistant-tool-error",
                    json!({
                        "tool": tool_name,
                        "error": err.to_string(),
                        "session_id": ctx.session_id,
                        "turn_id": ctx.turn_id,
                        "tool_call_id": call_id,
                    }),
                );
                last_outcome = Some(ToolCallOutcome {
                    tool_call_id: call_id.clone(),
                    tool_name: tool_name.clone(),
                    args: args.clone(),
                    result_json: err.to_llm_message(),
                    decision: "error".into(),
                    duration_ms: dur_ms,
                    from_cache: false,
                });
                break;
            }
        }
    }

    last_outcome.unwrap_or_else(|| ToolCallOutcome {
        tool_call_id: call_id,
        tool_name,
        args,
        result_json: "Tool execution failed.".into(),
        decision: "error".into(),
        duration_ms: 0,
        from_cache: false,
    })
}

// ── Streaming accumulator ─────────────────────────────────────────────────────
// Reads SSE lines from llama-server (stream:true), emits tokens to the assistant
// window, and returns the fully assembled message object.

async fn accumulate_stream(
    resp:      reqwest::Response,
    app:       &AppHandle,
    stream_tx: &Option<mpsc::UnboundedSender<String>>,
    cancel:    Arc<AtomicBool>,
) -> Result<(Value, Option<Vec<Value>>), String> {
    let mut text_content  = String::new();
    let mut tool_calls_map: std::collections::HashMap<usize, Value> = Default::default();
    let mut finish_reason = "stop".to_string();

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::SeqCst) {
            return Err("Cancelled".into());
        }
        let bytes = chunk.map_err(|e| format!("stream read: {e}"))?;
        buffer.push_str(&String::from_utf8_lossy(&bytes));

        // Process complete SSE lines
        while let Some(nl) = buffer.find('\n') {
            let line = buffer[..nl].trim().to_string();
            buffer = buffer[nl + 1..].to_string();

            if line.is_empty() || line == "data: [DONE]" { continue; }
            let json_str = line.strip_prefix("data: ").unwrap_or(&line);
            let Ok(delta_obj): Result<Value, _> = serde_json::from_str(json_str) else { continue };

            let choice = &delta_obj["choices"][0];
            finish_reason = choice["finish_reason"].as_str()
                .filter(|s| *s != "null")
                .unwrap_or(&finish_reason)
                .to_string();

            let delta = &choice["delta"];

            // Accumulate text content
            if let Some(tok) = delta.get("content").and_then(|c| c.as_str()) {
                if !tok.is_empty() {
                    text_content.push_str(tok);
                    let _ = app.emit(
                        "token-stream-assistant",
                        tok,
                    );
                    if let Some(ref tx) = stream_tx { let _ = tx.send(tok.to_string()); }
                }
            }

            // Accumulate tool_calls deltas (each has an index)
            if let Some(tcs) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                for tc_delta in tcs {
                    let idx = tc_delta.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let entry = tool_calls_map.entry(idx).or_insert_with(|| json!({
                        "id": "", "type": "function",
                        "function": { "name": "", "arguments": "" }
                    }));
                    if let Some(id) = tc_delta.get("id").and_then(|v| v.as_str()) {
                        entry["id"] = json!(id);
                    }
                    if let Some(name) = tc_delta.pointer("/function/name").and_then(|v| v.as_str()) {
                        let cur = entry["function"]["name"].as_str().unwrap_or("").to_string();
                        entry["function"]["name"] = json!(cur + name);
                    }
                    if let Some(args) = tc_delta.pointer("/function/arguments").and_then(|v| v.as_str()) {
                        let cur = entry["function"]["arguments"].as_str().unwrap_or("").to_string();
                        entry["function"]["arguments"] = json!(cur + args);
                    }
                }
            }
        }
    }

    let tool_calls_vec = if tool_calls_map.is_empty() {
        None
    } else {
        let mut sorted: Vec<_> = tool_calls_map.into_iter().collect();
        sorted.sort_by_key(|(i, _)| *i);
        Some(sorted.into_iter().map(|(_, v)| v).collect())
    };

    let message = if let Some(ref calls) = tool_calls_vec {
        json!({
            "role":       "assistant",
            "content":    text_content,
            "tool_calls": calls,
        })
    } else {
        json!({ "role": "assistant", "content": text_content })
    };

    Ok((message, tool_calls_vec))
}

// ── Context-window protector ──────────────────────────────────────────────────
// If a tool result exceeds the threshold, return a compressed summary notice
// instead of the full JSON. The full result is preserved in the audit log.

fn maybe_summarize(val: &Value, tool_name: &str) -> String {
    let s = serde_json::to_string(val).unwrap_or_default();
    if s.len() <= RESULT_SUMMARIZE_THRESHOLD {
        return s;
    }
    // Try to produce a meaningful summary for common structures
    if let Some(files) = val.get("files").and_then(|v| v.as_array()) {
        return serde_json::to_string(&json!({
            "count": files.len(),
            "files": &files[..files.len().min(10)],
            "_note": format!("Result truncated. Full {} files available on request.", files.len()),
        })).unwrap_or(s);
    }
    if let Some(content) = val.get("content").and_then(|v| v.as_str()) {
        return serde_json::to_string(&json!({
            "content": &content[..content.len().min(1500)],
            "_note": "Content truncated to 1500 chars. Ask for more if needed.",
        })).unwrap_or(s);
    }
    // Generic: keep first 2KB
    format!("{}... [result truncated — full {} bytes available, tool: {tool_name}]",
        &s[..s.len().min(2048)], s.len())
}

// ── Plain-text fallback ───────────────────────────────────────────────────────
// Used when llama-server returns 400/422 for tool schemas (model doesn't support them).

async fn run_plain_turn(
    history:    Vec<Value>,
    profile:    &AssistantProfile,
    orch:       &ModelOrchestrator,
    audit:      &AuditLog,
    app:        &AppHandle,
    cancel:     Arc<AtomicBool>,
    stream_tx:  Option<mpsc::UnboundedSender<String>>,
    session_id: &str,
) -> Result<AssistantTurn, String> {
    use tokio::sync::oneshot;
    use crate::model_orchestrator::InferRequest;

    let (resp_tx, resp_rx) = oneshot::channel();
    let (tok_tx, mut tok_rx) = mpsc::unbounded_channel::<String>();
    let app2 = app.clone();
    let outer_tx = stream_tx.clone();
    tokio::spawn(async move {
        while let Some(tok) = tok_rx.recv().await {
            let _ = app2.emit("token-stream-assistant", &tok);
            if let Some(ref tx) = outer_tx { let _ = tx.send(tok); }
        }
    });

    let req = InferRequest {
        model_id:    profile.model_id.clone(),
        messages:    history,
        max_tokens:  MAX_TOKENS,
        overrides:   None,
        stream_tx:   Some(tok_tx),
        cancel_flag: Some(cancel),
        resp_tx,
        source:      "assistant",
    };
    orch.infer(req)?;

    let (reply, _stats) = resp_rx.await
        .map_err(|_| "inference channel closed".to_string())??;

    let plain_turn_id = {
        use rand::distributions::Alphanumeric;
        use rand::Rng;
        rand::thread_rng().sample_iter(&Alphanumeric).take(16).map(char::from).collect::<String>()
    };
    audit.log_decision_with_context(
        "plain_turn",
        "allowed",
        "{}",
        None,
        None,
        Some(session_id),
        Some(&plain_turn_id),
        None,
    );
    Ok(AssistantTurn { reply, outcomes: vec![], confirm_token: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_injected_tool_triggers_expansion() {
        let injected = vec!["get_datetime".to_string(), "get_system_stats".to_string()];
        assert!(should_expand_for_non_injected("get_weather", &injected));
    }

    #[test]
    fn injected_tool_does_not_trigger_expansion() {
        let injected = vec!["get_datetime".to_string(), "get_system_stats".to_string()];
        assert!(!should_expand_for_non_injected("get_datetime", &injected));
    }

    #[test]
    fn expansion_queue_deduplicates_tool_names() {
        let mut expand = vec!["get_weather".to_string()];
        queue_expansion_tool(&mut expand, "get_weather");
        queue_expansion_tool(&mut expand, "get_datetime");
        assert_eq!(expand, vec!["get_weather".to_string(), "get_datetime".to_string()]);
    }

    #[test]
    fn expansion_retry_blocked_on_last_iteration() {
        let expand = vec!["get_weather".to_string()];
        assert!(should_retry_with_expansion(&expand, MAX_REACT_ITERATIONS - 2));
        assert!(!should_retry_with_expansion(&expand, MAX_REACT_ITERATIONS - 1));
    }

    struct CacheableTestTool;

    #[async_trait::async_trait]
    impl crate::tool_core::Tool for CacheableTestTool {
        fn name(&self) -> &'static str { "cacheable_test" }
        fn description(&self) -> &'static str { "cacheable test tool" }
        fn schema(&self) -> serde_json::Value { serde_json::json!({"type": "object"}) }
        fn policy_hint(&self) -> crate::tool_core::ToolPolicyHint { crate::tool_core::ToolPolicyHint::safe() }
        fn side_effects(&self) -> crate::tool_core::SideEffectProfile { crate::tool_core::SideEffectProfile::Read }
        fn tags(&self) -> &'static [&'static str] { &["test"] }
        fn cache_ttl_secs(&self) -> Option<u64> { Some(120) }
        async fn execute(&self, _args: &serde_json::Value, _ctx: &crate::tool_core::ToolContext) -> crate::tool_core::ToolResult {
            Ok(crate::tool_core::ToolOutput::Complete(serde_json::json!({"ok": true})))
        }
    }

    struct NonCacheableTestTool;

    #[async_trait::async_trait]
    impl crate::tool_core::Tool for NonCacheableTestTool {
        fn name(&self) -> &'static str { "non_cacheable_test" }
        fn description(&self) -> &'static str { "non-cacheable test tool" }
        fn schema(&self) -> serde_json::Value { serde_json::json!({"type": "object"}) }
        fn policy_hint(&self) -> crate::tool_core::ToolPolicyHint { crate::tool_core::ToolPolicyHint::external() }
        fn side_effects(&self) -> crate::tool_core::SideEffectProfile { crate::tool_core::SideEffectProfile::External }
        fn tags(&self) -> &'static [&'static str] { &["test"] }
        fn cache_ttl_secs(&self) -> Option<u64> { None }
        async fn execute(&self, _args: &serde_json::Value, _ctx: &crate::tool_core::ToolContext) -> crate::tool_core::ToolResult {
            Ok(crate::tool_core::ToolOutput::Complete(serde_json::json!({"ok": true})))
        }
    }

    #[test]
    fn cache_enabled_helper_respects_ttl_presence() {
        let cacheable: Arc<dyn crate::tool_core::Tool> = Arc::new(CacheableTestTool);
        let non_cacheable: Arc<dyn crate::tool_core::Tool> = Arc::new(NonCacheableTestTool);

        assert_eq!(cache_enabled_for_tool(&cacheable), Some(120));
        assert_eq!(cache_enabled_for_tool(&non_cacheable), None);
    }
}
