use futures::StreamExt;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use git2::Repository;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::Command;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use sysinfo::System;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;
use walkdir::WalkDir;

use crate::action_parser::handle_agent_response;
use crate::api_server;
use crate::agent_connect::{AgentConnectEvent, AgentConnectSession};
use crate::bootstrap;
use crate::error::BonsaiError;
use crate::cluster_orchestrator::{
    ClusterNode,
    ClusterPolicy,
    ClusterWorkload,
    NodeRuntimeMetrics,
};
use crate::model_orchestrator::InferStats;
use crate::remote::RemoteManager;
use crate::remote_input::RemoteInputEvent;
use crate::task_queue::{InferenceTask, TaskQueueStatus, TaskSource, TaskType};
use crate::tools;
use crate::AppState;

// ─── Path guard ───────────────────────────────────────────────────────────────

/// Returns true if any component of `path` is a parent-directory (`..`).
/// Using `Path::components()` is more precise than `contains("..")` which
/// would incorrectly flag filenames like `foo..bar.txt`.
fn has_parent_dir_component(path: &str) -> bool {
    use std::path::Component;
    std::path::Path::new(path)
        .components()
        .any(|c| c == Component::ParentDir)
}

fn default_canvas_layout() -> Value {
    json!({
        "schema_version": 1,
        "saved_at": Value::Null,
        "viewport": { "x": 0.0, "y": 0.0, "zoom": 1.0 },
        "nodes": [],
        "connections": []
    })
}

// ─── File system ─────────────────────────────────────────────────────────────

#[tauri::command]
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub async fn open_workspace(app_handle: AppHandle) -> Result<String, String> {
    let path = app_handle
        .dialog()
        .file()
        .blocking_pick_folder()
        .map(|p| p.to_string())
        .ok_or_else(|| "No folder selected".to_string())?;
    Ok(path)
}

#[tauri::command]
#[cfg(any(target_os = "android", target_os = "ios"))]
pub async fn open_workspace(_app_handle: AppHandle) -> Result<String, String> {
    Err("Workspace folder picker is not supported on mobile targets".to_string())
}

#[tauri::command]
pub async fn read_file(path: String) -> Result<String, String> {
    if has_parent_dir_component(&path) {
        return Err("Path not allowed: traversal sequences are forbidden".to_string());
    }
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn write_file(path: String, content: String) -> Result<(), String> {
    if has_parent_dir_component(&path) {
        return Err("Path not allowed: traversal sequences are forbidden".to_string());
    }
    let p = std::path::Path::new(&path);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_canvas_layout(workspace_path: String) -> Result<Value, String> {
    if has_parent_dir_component(&workspace_path) {
        return Err("Path not allowed: traversal sequences are forbidden".to_string());
    }

    let bonsai_dir = std::path::Path::new(&workspace_path).join(".bonsai");
    let canvas_path = bonsai_dir.join("canvas.json");
    if !canvas_path.exists() {
        return Ok(json!({
            "layout": default_canvas_layout(),
        }));
    }

    let raw = fs::read_to_string(&canvas_path).map_err(|e| e.to_string())?;
    match serde_json::from_str::<Value>(&raw) {
        Ok(layout) => Ok(json!({ "layout": layout })),
        Err(err) => {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_secs();
            let backup_path = bonsai_dir.join(format!("canvas.corrupt.{}.json", ts));
            fs::rename(&canvas_path, &backup_path).map_err(|e| e.to_string())?;
            Ok(json!({
                "layout": default_canvas_layout(),
                "recovered_corrupt_file": backup_path.to_string_lossy().to_string(),
                "error": err.to_string(),
            }))
        }
    }
}

#[tauri::command]
pub async fn save_canvas_layout(workspace_path: String, layout: Value) -> Result<Value, String> {
    if has_parent_dir_component(&workspace_path) {
        return Err("Path not allowed: traversal sequences are forbidden".to_string());
    }

    let bonsai_dir = std::path::Path::new(&workspace_path).join(".bonsai");
    fs::create_dir_all(&bonsai_dir).map_err(|e| e.to_string())?;

    let canvas_path = bonsai_dir.join("canvas.json");
    let tmp_path = bonsai_dir.join("canvas.json.tmp");
    let mut doc = layout;
    if doc.get("schema_version").is_none() {
        doc["schema_version"] = json!(1);
    }
    let saved_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis();
    doc["saved_at"] = json!(saved_at_ms.to_string());

    let payload = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    {
        let mut temp = fs::File::create(&tmp_path).map_err(|e| e.to_string())?;
        temp.write_all(payload.as_bytes()).map_err(|e| e.to_string())?;
        temp.write_all(b"\n").map_err(|e| e.to_string())?;
        temp.sync_all().map_err(|e| e.to_string())?;
    }

    fs::rename(&tmp_path, &canvas_path).map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true, "path": canvas_path.to_string_lossy().to_string() }))
}

#[tauri::command]
pub async fn create_directory(path: String) -> Result<(), String> {
    if has_parent_dir_component(&path) {
        return Err("Path not allowed: traversal sequences are forbidden".to_string());
    }
    fs::create_dir_all(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_file(path: String) -> Result<(), String> {
    if has_parent_dir_component(&path) {
        return Err("Path not allowed: traversal sequences are forbidden".to_string());
    }
    let p = std::path::Path::new(&path);
    if p.is_dir() {
        fs::remove_dir_all(&path).map_err(|e| e.to_string())
    } else {
        fs::remove_file(&path).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn list_project_files(workspace_path: String) -> Result<Vec<serde_json::Value>, String> {
    let mut entries = Vec::new();
    for entry in WalkDir::new(&workspace_path)
        .follow_links(false)
        .max_depth(8)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            !e.path().components().any(|c| {
                let s = c.as_os_str().to_string_lossy();
                s.starts_with('.') || s == "node_modules" || s == "target"
            })
        })
    {
        let raw_rel = entry
            .path()
            .strip_prefix(&workspace_path)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");
        // strip_prefix on Windows leaves a leading backslash → leading slash after replace
        let rel = raw_rel.trim_start_matches('/').to_string();
        // Skip the workspace root itself (empty rel)
        if rel.is_empty() { continue; }
        entries.push(serde_json::json!({
            "path":   entry.path().to_string_lossy(),
            "rel":    rel,
            "name":   entry.file_name().to_string_lossy(),
            "is_dir": entry.file_type().is_dir(),
        }));
    }
    Ok(entries)
}

// ─── Git ─────────────────────────────────────────────────────────────────────

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn get_git_status(workspace_path: String) -> Result<Vec<serde_json::Value>, String> {
    let repo = Repository::open(&workspace_path).map_err(|e| e.to_string())?;
    let statuses = repo.statuses(None).map_err(|e| e.to_string())?;
    let mut entries = vec![];
    for s in statuses.iter() {
        let status_str = match s.status() {
            git2::Status::CURRENT => "clean",
            s if s.intersects(git2::Status::INDEX_MODIFIED | git2::Status::WT_MODIFIED) => {
                "modified"
            }
            s if s.intersects(git2::Status::INDEX_NEW | git2::Status::WT_NEW) => "added",
            s if s.intersects(git2::Status::INDEX_DELETED | git2::Status::WT_DELETED) => "deleted",
            s if s.intersects(git2::Status::CONFLICTED) => "conflict",
            _ => "unknown",
        };
        entries.push(serde_json::json!({ "path": s.path().unwrap_or(""), "status": status_str }));
    }
    Ok(entries)
}

#[cfg(any(target_os = "android", target_os = "ios"))]
#[tauri::command]
pub async fn get_git_status(_workspace_path: String) -> Result<Vec<serde_json::Value>, String> {
    Err("Git status is not supported on mobile targets".to_string())
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[tauri::command]
pub async fn get_git_branch(workspace_path: String) -> Result<String, String> {
    let repo = Repository::open(&workspace_path).map_err(|e| e.to_string())?;
    let head = repo.head().map_err(|e| e.to_string())?;
    Ok(head.shorthand().unwrap_or("HEAD").to_string())
}

#[cfg(any(target_os = "android", target_os = "ios"))]
#[tauri::command]
pub async fn get_git_branch(_workspace_path: String) -> Result<String, String> {
    Err("Git branch is not supported on mobile targets".to_string())
}

// ─── Chat / AI ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ChatMessagePayload {
    pub role:    String,
    pub content: String,
}

const CHAT_PROMPT_TOKEN_BUDGET: usize = 3200;
const CHAT_PROMPT_TOKEN_BUDGET_REDUCED: usize = 2400;

fn estimate_msg_tokens(msg: &Value) -> usize {
    let role_len = msg
        .get("role")
        .and_then(|v| v.as_str())
        .map(crate::context_builder::estimate_tokens)
        .unwrap_or(0);
    let content_len = msg
        .get("content")
        .and_then(|v| v.as_str())
        .map(crate::context_builder::estimate_tokens)
        .unwrap_or(0);
    role_len + content_len + 10
}

fn estimate_ctx_tokens(ctx: &[Value]) -> usize {
    ctx.iter().map(estimate_msg_tokens).sum()
}

fn set_msg_content(msg: &mut Value, content: String) {
    if let Some(obj) = msg.as_object_mut() {
        obj.insert("content".to_string(), Value::String(content));
    }
}

fn truncate_to_tokens(text: &str, max_tokens: usize) -> String {
    let max_chars = max_tokens.saturating_mul(4);
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let tail: String = text
        .chars()
        .rev()
        .take(max_chars)
        .collect::<Vec<char>>()
        .into_iter()
        .rev()
        .collect();
    format!("[truncated to fit context]\n{}", tail)
}

fn trim_context_to_budget(ctx: &mut Vec<Value>, budget_tokens: usize) -> bool {
    if ctx.is_empty() {
        return false;
    }

    let mut trimmed = false;

    // Keep system prompt at index 0, drop oldest conversational turns first.
    while estimate_ctx_tokens(ctx) > budget_tokens && ctx.len() > 2 {
        ctx.remove(1);
        trimmed = true;
    }

    if estimate_ctx_tokens(ctx) > budget_tokens {
        let without_last = if ctx.len() > 1 {
            estimate_ctx_tokens(&ctx[..ctx.len() - 1])
        } else {
            0
        };
        if let Some(last) = ctx.last_mut() {
            let allowed = budget_tokens.saturating_sub(without_last).saturating_sub(12);
            let current = last
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            set_msg_content(last, truncate_to_tokens(current, allowed));
            trimmed = true;
        }
    }

    if estimate_ctx_tokens(ctx) > budget_tokens {
        let others = if ctx.len() > 1 {
            estimate_ctx_tokens(&ctx[1..])
        } else {
            0
        };
        if let Some(system) = ctx.get_mut(0) {
            let allowed = budget_tokens.saturating_sub(others).saturating_sub(12);
            let current = system
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            set_msg_content(system, truncate_to_tokens(current, allowed));
            trimmed = true;
        }
    }

    trimmed
}

fn is_context_overflow_error(err: &str) -> bool {
    let e = err.to_lowercase();
    e.contains("exceed_context_size_error")
        || e.contains("exceeds the available context size")
        || e.contains("n_ctx")
}

fn is_file_inventory_request(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("list files")
        || t.contains("list all files")
        || t.contains("files in this folder")
        || t.contains("files in this directory")
        || t.contains("show files")
        || t.contains("readme")
        || t.contains("read the file")
}

fn is_greeting_message(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    if t.is_empty() {
        return false;
    }

    let normalized = t
        .trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace())
        .to_string();

    matches!(normalized.as_str(),
        "hi" | "hello" | "hey" | "yo" | "sup" | "good morning" | "good afternoon" | "good evening"
    )
}

fn is_system_info_request(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("how much ram")
        || t.contains("how much memory")
        || t.contains("ram do i have")
        || t.contains("memory do i have")
        || t.contains("system specs")
        || t.contains("computer specs")
        || t.contains("hardware info")
        || t.contains("system info")
        || (t.contains("cpu") && t.contains("gpu"))
}

fn emit_agent_connect_event(
    state: &AppState,
    app_handle: &AppHandle,
    event_type: &str,
    summary: &str,
    details: Value,
) {
    let event = {
        let mut hub = match state.agent_connect.lock() {
            Ok(h) => h,
            Err(_) => return,
        };
        hub.append_to_active(event_type, summary, details)
    };

    if let Some(ev) = event {
        let _ = app_handle.emit("agent-connect-event", &ev);
    }
}

#[tauri::command]
pub async fn agent_connect_start_session(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    goal: Option<String>,
    workspace_path: Option<String>,
) -> Result<AgentConnectSession, String> {
    let session = {
        let mut hub = state
            .agent_connect
            .lock()
            .map_err(|_| "Agent Connect state unavailable".to_string())?;
        hub.start_session(goal.clone(), workspace_path.clone())
    };

    emit_agent_connect_event(
        &state,
        &app_handle,
        "session.started",
        "Agent Connect session started",
        json!({
            "session_id": session.id,
            "goal": goal,
            "workspace_path": workspace_path,
        }),
    );

    Ok(session)
}

#[tauri::command]
pub async fn agent_connect_set_active_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<AgentConnectSession, String> {
    let mut hub = state
        .agent_connect
        .lock()
        .map_err(|_| "Agent Connect state unavailable".to_string())?;
    hub.set_active_session(&session_id)
}

#[tauri::command]
pub async fn agent_connect_get_active_session(
    state: State<'_, AppState>,
) -> Result<Option<AgentConnectSession>, String> {
    let hub = state
        .agent_connect
        .lock()
        .map_err(|_| "Agent Connect state unavailable".to_string())?;
    Ok(hub.get_active_session())
}

#[tauri::command]
pub async fn agent_connect_list_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<AgentConnectSession>, String> {
    let hub = state
        .agent_connect
        .lock()
        .map_err(|_| "Agent Connect state unavailable".to_string())?;
    Ok(hub.list_sessions())
}

#[tauri::command]
pub async fn agent_connect_get_timeline(
    state: State<'_, AppState>,
    session_id: Option<String>,
    after_seq: Option<u64>,
    limit: Option<usize>,
) -> Result<Vec<AgentConnectEvent>, String> {
    let hub = state
        .agent_connect
        .lock()
        .map_err(|_| "Agent Connect state unavailable".to_string())?;
    Ok(hub.get_timeline(session_id.as_deref(), after_seq, limit))
}

#[tauri::command]
pub async fn agent_connect_end_session(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    session_id: Option<String>,
    status: Option<String>,
) -> Result<AgentConnectSession, String> {
    let session = {
        let mut hub = state
            .agent_connect
            .lock()
            .map_err(|_| "Agent Connect state unavailable".to_string())?;
        hub.end_session(session_id.as_deref(), status.clone())?
    };

    let _ = app_handle.emit(
        "agent-connect-session-ended",
        json!({
            "session_id": session.id,
            "status": session.status,
        }),
    );

    Ok(session)
}

#[derive(serde::Serialize)]
pub struct ChatResponse {
    pub content:        String,
    pub stats:          InferStats,
    /// true when the response was paused for HITL tool approval
    pub action_handled: bool,
    /// Tools automatically executed (no HITL) during this turn
    pub tools_used:     Vec<String>,
}

/// Single inference call — streams tokens via "token-stream" event.
async fn run_inference(
    task_queue:   &crate::task_queue::TaskQueue,
    app_handle:   &AppHandle,
    messages:     Vec<Value>,
    cancel_flag:  Option<Arc<std::sync::atomic::AtomicBool>>,
) -> Result<(String, InferStats), String> {
    let (stream_tx, mut stream_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(tok) = stream_rx.recv().await {
            let _ = handle.emit("token-stream", &tok);
        }
    });
    task_queue.submit(InferenceTask {
        task_type: TaskType::UserChat,
        source: TaskSource::Workspace,
        model_id: None,
        messages,
        max_tokens: 4096,
        overrides: None,
        stream_tx: Some(stream_tx),
        cancel_flag,
        estimated_tokens: 4096,
        estimated_ram_mb: 2048,
    }).await
}

#[tauri::command]
pub async fn submit_chat(
    app_handle:     AppHandle,
    state:          State<'_, AppState>,
    messages:       Vec<ChatMessagePayload>,
    workspace_path: Option<String>,
    enabled_tools:  Option<Vec<String>>,
) -> Result<ChatResponse, BonsaiError> {
    state.chat_cancel.store(false, Ordering::Relaxed);

    let last_user_text = messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();

    emit_agent_connect_event(
        &state,
        &app_handle,
        "chat.submitted",
        "User submitted chat message",
        json!({
            "workspace_path": workspace_path,
            "message_preview": last_user_text.chars().take(220).collect::<String>(),
            "message_count": messages.len(),
        }),
    );

    let mut tools = tools::all_tools(workspace_path.as_deref());
    if let Some(enabled) = enabled_tools {
        let allow: std::collections::HashSet<String> = enabled.into_iter().collect();
        tools.retain(|t| allow.contains(&t.name));
    }

    let mut sys_prompt = tools::system_prompt(&tools, workspace_path.as_deref());
    if is_file_inventory_request(&last_user_text) {
            sys_prompt.push_str(
                "\n\n## Immediate instruction for this request\n\
                 - For this turn, execute tools directly (do not reply with a capabilities list).\n\
                 - If user asks to list files, call list_all_files first.\n\
                 - If user asks to read a file and filename is known (for example README), locate it with list_all_files and then call read_file.\n"
            );
    }
    let greeting_only = is_greeting_message(&last_user_text);

    if is_system_info_request(&last_user_text) {
        sys_prompt.push_str(
            "\n\n## Immediate instruction for this request\n\
             - The user is asking for machine/system facts (for example RAM).\n\
             - You MUST call run_command before answering.\n\
             - Output ONLY this tool call first (no prose):\n\
               <tool_call>{\"tool\":\"run_command\",\"args\":{\"command\":\"specs\"}}</tool_call>\n"
        );
    }
    if greeting_only {
        sys_prompt.push_str(
            "\n\n## Immediate instruction for this request\n\
             - The user sent a greeting/salutation.\n\
             - Do not call tools for this turn.\n\
             - Reply briefly and conversationally, then ask one helpful follow-up question.\n"
        );
    }

    // Build initial context list (system + conversation history)
    let mut ctx: Vec<Value> = vec![json!({"role": "system", "content": sys_prompt})];
    for m in &messages {
        ctx.push(json!({"role": m.role, "content": m.content}));
    }
    let was_trimmed = trim_context_to_budget(&mut ctx, CHAT_PROMPT_TOKEN_BUDGET);
    if was_trimmed {
        emit_agent_connect_event(
            &state,
            &app_handle,
            "chat.context_trimmed",
            "Context was trimmed to fit model limit",
            json!({
                "budget_tokens": CHAT_PROMPT_TOKEN_BUDGET,
                "estimated_tokens": estimate_ctx_tokens(&ctx),
            }),
        );
    }

    let mut final_content  = String::new();
    let mut final_stats    = InferStats::default();
    let mut action_handled = false;
    let mut tools_used     = Vec::<String>::new();
    let mut last_auto_tool_name = String::new();
    let mut last_auto_tool_output = String::new();
    let mut last_auto_tool_sig = String::new();
    let mut repeated_auto_tool_count = 0usize;
    let mut malformed_retry_used = false;
    let mut system_info_retry_used = false;
    let mut greeting_retry_used = false;
    const  MAX_TURNS: usize = 8;
    let mut loop_limit_reached = true;

    for _turn in 0..MAX_TURNS {
        trim_context_to_budget(&mut ctx, CHAT_PROMPT_TOKEN_BUDGET);
        let (raw, stats) = match run_inference(
            &state.task_queue,
            &app_handle,
            ctx.clone(),
            Some(state.chat_cancel.clone()),
        ).await {
            Ok(v) => v,
            Err(e) => {
                if is_context_overflow_error(&e) {
                    let trimmed_more = trim_context_to_budget(&mut ctx, CHAT_PROMPT_TOKEN_BUDGET_REDUCED);
                    if trimmed_more {
                        emit_agent_connect_event(
                            &state,
                            &app_handle,
                            "chat.context_trimmed",
                            "Context trimmed again after overflow",
                            json!({
                                "budget_tokens": CHAT_PROMPT_TOKEN_BUDGET_REDUCED,
                                "estimated_tokens": estimate_ctx_tokens(&ctx),
                            }),
                        );
                        continue;
                    }
                }
                return Err(e.into());
            }
        };
        final_stats       = stats;
        let response      = strip_think_tags(&raw);
        let parsed        = tools::parse_tool_calls(&response);
        let calls         = parsed.calls;

        if calls.is_empty() {
            if parsed.malformed_count > 0 {
                emit_agent_connect_event(
                    &state,
                    &app_handle,
                    "tool.parse_error",
                    "Model emitted malformed tool call payload",
                    json!({
                        "malformed_count": parsed.malformed_count,
                        "raw_response": response.chars().take(1200).collect::<String>(),
                    }),
                );
                if malformed_retry_used {
                    final_content = "Tool call JSON is malformed. Please retry your request.".to_string();
                    loop_limit_reached = false;
                    break;
                }
                malformed_retry_used = true;
                ctx.push(json!({"role": "assistant", "content": &response}));
                ctx.push(json!({"role": "user", "content": "<tool_result>Error: malformed JSON in tool_call. Please reformat.</tool_result>"}));
                continue;
            }

            if is_system_info_request(&last_user_text) {
                let run_command_available = tools.iter().any(|t| t.name == "run_command");
                if !run_command_available {
                    final_content = "I need the run_command tool enabled to retrieve machine specs. Please enable command execution tools and retry.".to_string();
                    loop_limit_reached = false;
                    break;
                }

                if !system_info_retry_used {
                    system_info_retry_used = true;
                    ctx.push(json!({"role": "assistant", "content": &response}));
                    ctx.push(json!({
                        "role": "user",
                        "content": "Tool required for this request. Call run_command with command 'specs' before answering. Return only a <tool_call> block."
                    }));
                    continue;
                }

                // Deterministic fallback: trigger approval flow directly so system facts are gathered.
                let fallback_args = json!({ "command": "specs" });
                let payload = json!({
                    "type":          "tool_approval",
                    "tool":          "run_command",
                    "args":          fallback_args,
                    "description":   "Run command: specs",
                    "rationale":     "System information request requires factual command output.",
                    "paths_affected": [],
                    "action":        json!({"tool": "run_command", "args": {"command": "specs"}}),
                    "raw_response":  &response,
                    "ctx_snapshot":  &ctx,
                });
                let _ = app_handle.emit("permission-request", payload);
                emit_agent_connect_event(
                    &state,
                    &app_handle,
                    "hitl.requested",
                    "Tool approval required (system-info fallback)",
                    json!({
                        "tool": "run_command",
                        "args": {"command": "specs"},
                        "source": "system_info_fallback",
                    }),
                );

                final_content = "I need to run a local system command to answer this accurately. Please approve the request.".to_string();
                action_handled = true;
                loop_limit_reached = false;
                break;
            }

            // No tool calls — this is the final prose response.
            final_content = tools::strip_tool_calls(&response);
            loop_limit_reached = false;
            break;
        }

        if greeting_only {
            emit_agent_connect_event(
                &state,
                &app_handle,
                "tool.prevented",
                "Tool use prevented for greeting-only message",
                json!({
                    "tool": calls[0].tool,
                    "reason": "greeting_only",
                }),
            );

            if !greeting_retry_used {
                greeting_retry_used = true;
                ctx.push(json!({"role": "assistant", "content": &response}));
                ctx.push(json!({
                    "role": "user",
                    "content": "No tools are needed for this greeting. Reply conversationally and ask what the user wants to do next."
                }));
                continue;
            }

            final_content = "Hello! I am ready to help. What would you like to work on right now?".to_string();
            loop_limit_reached = false;
            break;
        }

        // Process the first tool call in this response.
        let call = &calls[0];
        match tools.iter().find(|t| t.name == call.tool) {
            None => {
                // Unknown tool — inject an error result and loop back
                ctx.push(json!({"role": "assistant", "content": &response}));
                ctx.push(json!({"role": "user",
                    "content": format!("<tool_result>\nError: unknown tool `{}`\n</tool_result>", call.tool)}));
            }
            Some(tool_def) if tool_def.requires_approval => {
                // Needs HITL approval — pause here and emit a permission card.
                // Include a ctx_snapshot and raw_response so the frontend can
                // reconstruct the full conversation after the user approves.
                let mut payload = json!({
                    "type":          "tool_approval",
                    "tool":          &call.tool,
                    "args":          &call.args,
                    "description":   tool_human_description(&call.tool, &call.args),
                    "rationale":     format!("The model wants to run `{}` on your device.", &call.tool),
                    "paths_affected": paths_from_args(&call.args),
                    "action":        json!({"tool": &call.tool, "args": &call.args}),
                    "raw_response":  &response,
                    "ctx_snapshot":  &ctx,
                });

                if call.tool == "write_file" {
                    if let Some((file_path, unified_diff)) = write_file_diff_preview(&call.args) {
                        payload["file_path"] = json!(file_path);
                        payload["unified_diff"] = json!(unified_diff);
                    }
                }

                let _ = app_handle.emit("permission-request", payload);
                emit_agent_connect_event(
                    &state,
                    &app_handle,
                    "hitl.requested",
                    "Tool approval required",
                    json!({
                        "tool": call.tool,
                        "args": call.args,
                    }),
                );
                final_content  = tools::strip_tool_calls(&response);
                action_handled = true;
                loop_limit_reached = false;
                break;
            }
            Some(tool_def) => {
                let call_sig = format!(
                    "{}:{}",
                    call.tool,
                    serde_json::to_string(&call.args).unwrap_or_default()
                );
                if call_sig == last_auto_tool_sig {
                    repeated_auto_tool_count += 1;
                } else {
                    repeated_auto_tool_count = 0;
                    last_auto_tool_sig = call_sig;
                }

                if repeated_auto_tool_count >= 2 {
                    final_content = finalize_tool_only_response(&last_auto_tool_name, &last_auto_tool_output);
                    emit_agent_connect_event(
                        &state,
                        &app_handle,
                        "tool.loop_prevented",
                        "Prevented repeated identical tool calls",
                        json!({
                            "tool": call.tool,
                            "repeat_count": repeated_auto_tool_count,
                        }),
                    );
                    loop_limit_reached = false;
                    break;
                }

                // Safe tool — execute automatically and loop back to inference.
                let output = if tool_def.is_custom {
                    let sp = tool_def.script_path.as_deref().ok_or("Missing script path")?;
                    tools::execute_custom(sp, &call.args).await
                        .unwrap_or_else(|e| format!("Error: {e}"))
                } else {
                    tools::execute_built_in(&call.tool, &call.args, workspace_path.as_deref()).await
                        .unwrap_or_else(|e| format!("Error: {e}"))
                };

                tools_used.push(call.tool.clone());
                last_auto_tool_name = call.tool.clone();
                last_auto_tool_output = output.clone();
                let _ = app_handle.emit("tool-used", json!({
                    "tool":   &call.tool,
                    "output": &output,
                }));

                emit_agent_connect_event(
                    &state,
                    &app_handle,
                    "tool.executed",
                    "Tool executed automatically",
                    json!({
                        "tool": call.tool,
                        "output_preview": output.chars().take(300).collect::<String>(),
                    }),
                );

                // Append assistant turn + tool result, then run inference again.
                ctx.push(json!({"role": "assistant", "content": &response}));
                ctx.push(json!({"role": "user",
                    "content": format!("<tool_result>\n{output}\n</tool_result>")}));
            }
        }

        if action_handled { break; }
    }

    if loop_limit_reached && !action_handled {
        let mut summary = "Tool loop limit reached after 8 turns. Try asking again with a narrower request.".to_string();
        if !last_auto_tool_output.trim().is_empty() {
            summary.push_str("\n\nLatest tool result:\n");
            summary.push_str(&truncate_chars(last_auto_tool_output.trim(), 1600));
        }
        final_content = summary;
    }

    if !action_handled && final_content.trim().is_empty() {
        if !last_auto_tool_output.trim().is_empty() {
            final_content = finalize_tool_only_response(&last_auto_tool_name, &last_auto_tool_output);
        } else {
            final_content = "I completed tool execution, but no final natural-language response was produced. Please ask me to summarize the result.".to_string();
        }
    }

    let _ = app_handle.emit("token-speed", final_stats.tokens_per_second as u32);

    emit_agent_connect_event(
        &state,
        &app_handle,
        "chat.completed",
        "Chat turn completed",
        json!({
            "action_handled": action_handled,
            "tools_used": tools_used,
            "completion_tokens": final_stats.completion_tokens,
            "tokens_per_second": final_stats.tokens_per_second,
        }),
    );

    Ok(ChatResponse { content: final_content, stats: final_stats, action_handled, tools_used })
}

#[tauri::command]
pub async fn generate_inline_completion(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    file_path: String,
    language: String,
    before_cursor: String,
    after_cursor: String,
) -> Result<String, String> {
    if before_cursor.trim().is_empty() {
        return Ok(String::new());
    }

    let system_prompt =
        "You are an inline code completion engine. Return only the text that should appear immediately after the cursor. \
Do not include markdown fences, explanations, or repeated context. Keep completions short and practical.";

    let user_prompt = format!(
        "File: {file_path}\nLanguage: {language}\n\nCode before cursor:\n```{language}\n{before_cursor}\n```\n\nCode after cursor:\n```{language}\n{after_cursor}\n```\n\nReturn only the continuation text to insert at cursor.",
    );

    let messages = vec![
        json!({"role": "system", "content": system_prompt}),
        json!({"role": "user", "content": user_prompt}),
    ];

    let (raw, _stats) = run_inference(&state.task_queue, &app_handle, messages, None).await?;
    let mut completion = strip_think_tags(&raw);
    completion = tools::strip_tool_calls(&completion);

    if completion.trim_start().starts_with("```") {
        completion = completion
            .trim()
            .trim_start_matches("```")
            .trim()
            .to_string();
        if let Some(end) = completion.rfind("```") {
            completion = completion[..end].trim().to_string();
        }
    }

    let mut lines: Vec<&str> = completion.lines().collect();
    if lines.len() > 8 {
        lines.truncate(8);
    }
    let mut trimmed = lines.join("\n");
    if trimmed.len() > 400 {
        trimmed = trimmed.chars().take(400).collect::<String>();
    }

    Ok(trimmed)
}

#[tauri::command]
pub async fn list_available_chat_tools(
    workspace_path: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let tools = tools::all_tools(workspace_path.as_deref());
    Ok(tools
        .into_iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "requires_approval": t.requires_approval,
                "is_custom": t.is_custom,
                "args": t.args,
            })
        })
        .collect())
}

#[tauri::command]
pub async fn stop_chat_generation(state: State<'_, AppState>) -> Result<(), String> {
    state.chat_cancel.store(true, Ordering::Relaxed);
    Ok(())
}

fn tool_human_description(tool: &str, args: &Value) -> String {
    match tool {
        "read_file"    => format!("Read file: {}",        args["path"].as_str().unwrap_or("?")),
        "write_file"   => format!("Write file: {}",       args["path"].as_str().unwrap_or("?")),
        "edit_file"    => format!("Edit file: {}",        args["path"].as_str().unwrap_or("?")),
        "delete_file"  => format!("Delete: {}",           args["path"].as_str().unwrap_or("?")),
        "create_dir"   => format!("Create directory: {}", args["path"].as_str().unwrap_or("?")),
        "run_command"  => format!("Run command: {}",      args["command"].as_str().unwrap_or("?")),
        "search_files" => format!("Search '{}' in {}",
            args["query"].as_str().unwrap_or("?"),
            args["path"].as_str().unwrap_or("?")),
        "grep_files"   => format!("Regex search '{}' in {}",
            args["pattern"].as_str().unwrap_or("?"),
            args["path"].as_str().unwrap_or("?")),
        "list_files"   => format!("List directory: {}",   args["path"].as_str().unwrap_or("?")),
        "list_all_files" => format!("List all files in: {}", args["path"].as_str().unwrap_or("(workspace)")),
        _              => format!("Execute tool: {tool}"),
    }
}

fn paths_from_args(args: &Value) -> Vec<String> {
    let mut v = vec![];
    if let Some(p) = args["path"].as_str()    { v.push(p.to_string()); }
    if let Some(p) = args["command"].as_str() { v.push(p.to_string()); }
    v
}

fn write_file_diff_preview(args: &Value) -> Option<(String, String)> {
    let path = args["path"].as_str()?.to_string();
    let new_content = args["content"].as_str()?;
    let current = fs::read_to_string(&path).unwrap_or_default();
    let patch = diffy::create_patch(&current, new_content).to_string();
    Some((path, patch))
}

fn strip_think_tags(text: &str) -> String {
    let open = "<think>";
    let close = "</think>";
    if let Some(start) = text.find(open) {
        if let Some(end) = text[start..].find(close) {
            let after = &text[start + end + close.len()..];
            return after.trim_start().to_string();
        }
    }
    text.to_string()
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let kept: String = text.chars().take(max_chars).collect();
    format!("{}\n\n... (truncated)", kept)
}

fn finalize_tool_only_response(tool_name: &str, output: &str) -> String {
    let safe_tool = if tool_name.trim().is_empty() { "tool" } else { tool_name };
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return format!("I ran `{}` but it returned no output.", safe_tool);
    }
    format!(
        "I used `{}` and got this result:\n\n{}",
        safe_tool,
        truncate_chars(trimmed, 6000)
    )
}

fn tool_name_from_action(action: &Value) -> String {
    action
        .get("tool")
        .and_then(|v| v.as_str())
        .unwrap_or("tool")
        .to_string()
}

fn tool_denied_message(tool: &str) -> String {
    format!("Tool execution denied for `{tool}`.")
}

fn build_resume_continuation_payloads(
    ctx_snapshot: &[Value],
    raw_response: &str,
    tool_output: &str,
) -> Vec<ChatMessagePayload> {
    let mut continuation_payloads = Vec::<ChatMessagePayload>::new();
    for msg in ctx_snapshot {
        let Some(role) = msg.get("role").and_then(|v| v.as_str()) else { continue };
        if role == "system" {
            continue;
        }
        let Some(content) = msg.get("content").and_then(|v| v.as_str()) else { continue };
        continuation_payloads.push(ChatMessagePayload {
            role: role.to_string(),
            content: content.to_string(),
        });
    }

    continuation_payloads.push(ChatMessagePayload {
        role: "assistant".to_string(),
        content: raw_response.to_string(),
    });
    continuation_payloads.push(ChatMessagePayload {
        role: "user".to_string(),
        content: format!("<tool_result>\n{}\n</tool_result>", tool_output),
    });

    continuation_payloads
}

#[tauri::command]
pub async fn execute_tool_call(
    app_handle: AppHandle,
    action: Value,
    workspace_path: Option<String>,
) -> Result<String, String> {
    let tool = action["tool"]
        .as_str()
        .ok_or_else(|| "Missing tool name".to_string())?;
    let args = action["args"].clone();
    let tool_defs = tools::all_tools(workspace_path.as_deref());
    let tool_def = tool_defs
        .into_iter()
        .find(|t| t.name == tool)
        .ok_or_else(|| format!("Unknown tool: {}", tool))?;

    let result = if tool_def.is_custom {
        let script_path = tool_def
            .script_path
            .as_deref()
            .ok_or_else(|| "Missing custom script path".to_string())?;
        tools::execute_custom(script_path, &args).await
    } else {
        tools::execute_built_in(tool, &args, workspace_path.as_deref()).await
    };

    match result {
        Ok(output) => {
            if tool == "run_command" {
                let cmd = args["command"].as_str().unwrap_or("(unknown command)");
                let _ = app_handle.emit("show-terminal", json!({ "source": "agent_tool" }));
                let _ = app_handle.emit(
                    "terminal-output",
                    json!({
                        "session_id": "agent-tool",
                        "text": format!("$ {}\n{}\n", cmd, output),
                    }),
                );
            }
            let _ = app_handle.emit(
                "agent-response",
                json!({
                    "type": "tool_result",
                    "tool": tool,
                    "output": output,
                }),
            );
            Ok(output)
        }
        Err(err) => Err(err),
    }
}

#[tauri::command]
pub async fn resume_tool_call(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    ctx_snapshot: Vec<Value>,
    raw_response: String,
    action: Value,
    approved: bool,
    workspace_path: Option<String>,
    enabled_tools: Option<Vec<String>>,
) -> Result<ChatResponse, BonsaiError> {
    let tool = tool_name_from_action(&action);

    if !approved {
        emit_agent_connect_event(
            &state,
            &app_handle,
            "hitl.denied",
            "Tool approval denied",
            json!({
                "tool": tool,
            }),
        );
        let _ = app_handle.emit("permission-resolved", json!({ "tool": tool, "granted": false }));

        return Ok(ChatResponse {
            content: tool_denied_message(&tool),
            stats: InferStats::default(),
            action_handled: false,
            tools_used: vec![],
        });
    }

    emit_agent_connect_event(
        &state,
        &app_handle,
        "hitl.approved",
        "Tool approval granted",
        json!({
            "tool": tool,
        }),
    );
    let _ = app_handle.emit("permission-resolved", json!({ "tool": tool, "granted": true }));

    let tool_output = execute_tool_call(app_handle.clone(), action.clone(), workspace_path.clone()).await?;

    let continuation_payloads =
        build_resume_continuation_payloads(&ctx_snapshot, &raw_response, &tool_output);

    let mut response = submit_chat(
        app_handle.clone(),
        state.clone(),
        continuation_payloads,
        workspace_path,
        enabled_tools,
    ).await?;

    if response.tools_used.is_empty() {
        response.tools_used.push(tool);
    }

    emit_agent_connect_event(
        &state,
        &app_handle,
        "hitl.resumed",
        "Chat resumed after tool approval",
        json!({
            "action_handled": response.action_handled,
            "tools_used": response.tools_used,
        }),
    );

    Ok(response)
}

#[tauri::command]
pub async fn list_chat_sessions(state: State<'_, AppState>) -> Result<Value, String> {
    let sessions = state.chat_sessions.list_sessions().await.map_err(|e| e.to_string())?;
    Ok(json!(sessions))
}

#[tauri::command]
pub async fn list_chat_sessions_detailed(
    state: State<'_, AppState>,
    include_deleted: Option<bool>,
) -> Result<Value, String> {
    let sessions = state
        .chat_sessions
        .list_sessions_detailed(include_deleted.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())?;
    Ok(json!(sessions))
}

#[tauri::command]
pub async fn save_chat_session(
    state: State<'_, AppState>,
    session_id: Option<String>,
    title: String,
    workspace_path: Option<String>,
    messages: Vec<Value>,
) -> Result<Value, String> {
    let id = state
        .chat_sessions
        .save_session(session_id, &title, workspace_path.as_deref(), &messages)
        .await
        .map_err(|e| e.to_string())?;
    Ok(json!({ "id": id }))
}

#[tauri::command]
pub async fn load_chat_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Value, String> {
    let session = state
        .chat_sessions
        .load_session(&session_id)
        .await
        .map_err(|e| e.to_string())?;
    serde_json::to_value(&session).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_chat_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    state
        .chat_sessions
        .delete_session(&session_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_chat_session(
    state: State<'_, AppState>,
    session_id: String,
    new_title: String,
) -> Result<(), String> {
    state
        .chat_sessions
        .rename_session(&session_id, &new_title)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn duplicate_chat_session(
    state: State<'_, AppState>,
    session_id: String,
    title: Option<String>,
) -> Result<serde_json::Value, String> {
    let new_id = state
        .chat_sessions
        .duplicate_session(&session_id, title.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "id": new_id }))
}

#[tauri::command]
pub async fn update_chat_session_meta(
    state: State<'_, AppState>,
    session_id: String,
    title: Option<String>,
    tags: Option<Vec<String>>,
    is_locked: Option<bool>,
    is_favorite: Option<bool>,
    is_deleted: Option<bool>,
) -> Result<(), String> {
    state
        .chat_sessions
        .update_session_meta(
            &session_id,
            title.as_deref(),
            tags,
            is_locked,
            is_favorite,
            is_deleted,
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_chat_session_groups(
    state: State<'_, AppState>,
    include_deleted: Option<bool>,
) -> Result<Value, String> {
    let groups = state
        .chat_sessions
        .list_groups(include_deleted.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())?;
    Ok(json!(groups))
}

#[tauri::command]
pub async fn create_chat_session_group(
    state: State<'_, AppState>,
    title: String,
) -> Result<Value, String> {
    let id = state
        .chat_sessions
        .create_group(&title)
        .await
        .map_err(|e| e.to_string())?;
    Ok(json!({ "id": id }))
}

#[tauri::command]
pub async fn update_chat_session_group_meta(
    state: State<'_, AppState>,
    group_id: String,
    title: Option<String>,
    tags: Option<Vec<String>>,
    is_locked: Option<bool>,
    is_favorite: Option<bool>,
    is_deleted: Option<bool>,
) -> Result<(), String> {
    state
        .chat_sessions
        .update_group_meta(
            &group_id,
            title.as_deref(),
            tags,
            is_locked,
            is_favorite,
            is_deleted,
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn link_chat_to_session_group(
    state: State<'_, AppState>,
    group_id: String,
    chat_id: String,
) -> Result<(), String> {
    state
        .chat_sessions
        .link_chat_to_group(&chat_id, &group_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn unlink_chat_from_session_group(
    state: State<'_, AppState>,
    group_id: String,
    chat_id: String,
) -> Result<(), String> {
    state
        .chat_sessions
        .unlink_chat_from_group(&chat_id, &group_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_group_chats(
    state: State<'_, AppState>,
    group_id: String,
) -> Result<Value, String> {
    let chats = state
        .chat_sessions
        .list_group_chats(&group_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(json!(chats))
}

// ─── Voice transcription ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn voice_transcribe(state: State<'_, AppState>) -> Result<String, String> {
    state.voice_cancel.store(false, Ordering::Relaxed);
    let cancel_flag = state.voice_cancel.clone();

    // cpal::Stream is !Send, so isolate the entire recording session inside
    // spawn_blocking where non-Send types are safe.
    let audio_data = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        use hound::{SampleFormat as HoundFormat, WavSpec, WavWriter};
        use std::io::Cursor;

        let host   = cpal::default_host();
        let device = host.default_input_device().ok_or("No audio input device found")?;
        let cfg    = device.default_input_config().map_err(|e| e.to_string())?;
        let channels    = cfg.channels();
        let sample_rate = cfg.sample_rate().0;

        let spec = WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: HoundFormat::Int,
        };

        let stop_clone = cancel_flag.clone();
        let pcm_buf: Arc<StdMutex<Vec<i16>>> = Arc::new(StdMutex::new(Vec::new()));
        let pcm_clone = pcm_buf.clone();

        let stream = device
            .build_input_stream(
                &cfg.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if stop_clone.load(std::sync::atomic::Ordering::Relaxed) {
                        return;
                    }
                    let mut buf = pcm_clone.lock().unwrap();
                    for &s in data {
                        buf.push((s.clamp(-1.0, 1.0) * 32767.0) as i16);
                    }
                },
                |err| tracing::error!(error=%err, "Audio input error"),
                None,
            )
            .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        let started = std::time::Instant::now();
        while started.elapsed() < std::time::Duration::from_secs(5) {
            if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
            drop(stream);
            return Err("Voice capture cancelled by user".to_string());
        }

        drop(stream);

        let samples = pcm_buf.lock().unwrap().clone();
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = WavWriter::new(&mut cursor, spec).map_err(|e| e.to_string())?;
            for s in &samples {
                writer.write_sample(*s).map_err(|e| e.to_string())?;
            }
            writer.finalize().map_err(|e| e.to_string())?;
        }
        Ok(cursor.into_inner())
    })
    .await
    .map_err(|e| e.to_string())??;

    state.whisper.transcribe(audio_data).await
}

#[tauri::command]
pub async fn stop_voice_capture(state: State<'_, AppState>) -> Result<(), String> {
    state.voice_cancel.store(true, Ordering::Relaxed);
    Ok(())
}

// ─── Project scaffolding ──────────────────────────────────────────────────────

#[tauri::command]
pub async fn create_project_from_template(
    template_id: String,
    project_name: String,
) -> Result<String, String> {
    let base = std::env::current_dir().map_err(|e| e.to_string())?;
    let proj = base.join(&project_name);
    fs::create_dir_all(&proj).map_err(|e| e.to_string())?;
    fs::write(
        proj.join("README.md"),
        format!("# {project_name}\n\nCreated from template: `{template_id}`\n"),
    )
    .map_err(|e| e.to_string())?;
    Ok(proj.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn ai_scaffold_project(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    project_path: String,
    template_id: String,
    user_prompt: String,
) -> Result<String, String> {
    let full_prompt = format!(
        "Scaffold a complete {template_id} project at path `{project_path}`. \
         User request: {user_prompt}. \
         Respond ONLY with a single valid JSON object matching the AgentAction schema \
         (type: file_create | file_edit | message | ask_permission). \
         No markdown, no explanation — pure JSON."
    );

    let (raw, _stats) = state
        .task_queue
        .submit(InferenceTask {
            task_type: TaskType::BackgroundTask,
            source: TaskSource::Workspace,
            model_id: None,
            messages: vec![
                json!({"role": "system", "content": "Scaffold a Bonsai project."}),
                json!({"role": "user", "content": full_prompt}),
            ],
            max_tokens: 4096,
            overrides: None,
            stream_tx: None,
            cancel_flag: None,
            estimated_tokens: 4096,
            estimated_ram_mb: 2048,
        })
        .await
        .map_err(|e| format!("Scaffold cancelled: {e}"))?;
    handle_agent_response(&app_handle, raw).await?;
    Ok("Scaffolding complete".to_string())
}

#[tauri::command]
pub async fn ai_code_review(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    file_path: String,
    content: String,
) -> Result<String, String> {
    let system = "You are a senior code reviewer. Return concrete findings ordered by severity, include exact evidence from the provided file, and end with a brief fix summary.";
    let user = format!(
        "Review this file and report real issues only. If no issues, say so explicitly.\n\nPath: {file_path}\n\n```\n{}\n```",
        truncate_chars(&content, 24_000)
    );
    let messages = vec![
        json!({"role": "system", "content": system}),
        json!({"role": "user", "content": user}),
    ];

    let (raw, _stats) = run_inference(&state.task_queue, &app_handle, messages, None).await?;
    let mut review = strip_think_tags(&raw);
    review = tools::strip_tool_calls(&review);
    let trimmed = review.trim();

    if trimmed.is_empty() {
        return Ok("Code review completed, but the model returned an empty response. Please retry.".to_string());
    }

    Ok(trimmed.to_string())
}

// ─── Terminal ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn run_terminal_command(command: String, app_handle: AppHandle) -> Result<(), String> {
    use tauri_plugin_shell::ShellExt;
    #[cfg(target_os = "windows")]
    let (sh, flag) = ("cmd", "/C");
    #[cfg(not(target_os = "windows"))]
    let (sh, flag) = ("sh", "-c");

    let (mut rx, _child) = app_handle
        .shell()
        .command(sh)
        .args([flag, &command])
        .spawn()
        .map_err(|e| e.to_string())?;

    while let Some(ev) = rx.recv().await {
        use tauri_plugin_shell::process::CommandEvent;
        let text = match ev {
            CommandEvent::Stdout(b)   => String::from_utf8_lossy(&b).into_owned(),
            CommandEvent::Stderr(b)   => String::from_utf8_lossy(&b).into_owned(),
            CommandEvent::Error(e)    => format!("error: {e}"),
            CommandEvent::Terminated(_) => break,
            _ => continue,
        };
        let _ = app_handle.emit(
            "terminal-output",
            json!({
                "session_id": "agent-tool",
                "text": text,
            }),
        );
    }
    Ok(())
}

#[tauri::command]
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub async fn spawn_pty_terminal(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    session_id: Option<String>,
) -> Result<(), String> {
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};

    let session_id = session_id.unwrap_or_else(|| "default".to_string());

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())?;

    let cmd = CommandBuilder::new(if cfg!(target_os = "windows") { "cmd.exe" } else { "bash" });
    let _child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;

    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
    let master = pair.master;

    {
        let mut sessions = state.pty_sessions.lock().await;
        sessions.insert(session_id.clone(), crate::PtySession { writer, master });
    }

    let handle = app_handle.clone();
    let sid = session_id.clone();
    tokio::task::spawn_blocking(move || {
        let mut buf = [0u8; 1024];
        loop {
            use std::io::Read;
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = handle.emit(
                        "pty-output",
                        json!({
                            "session_id": sid,
                            "text": text,
                        }),
                    );
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
#[cfg(any(target_os = "android", target_os = "ios"))]
pub async fn spawn_pty_terminal(
    _app_handle: AppHandle,
    _state: State<'_, AppState>,
    _session_id: Option<String>,
) -> Result<(), String> {
    Err("PTY terminal is not supported on mobile targets".to_string())
}

#[tauri::command]
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub async fn send_to_pty(input: String, state: State<'_, AppState>) -> Result<(), String> {
    use std::io::Write;
    let session_id = "default".to_string();
    let mut sessions = state.pty_sessions.lock().await;
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| format!("No PTY session available ({session_id})"))?;
    session
        .writer
        .write_all(input.as_bytes())
        .map_err(|e| e.to_string())?;
    session.writer.write_all(b"\r").map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
#[cfg(any(target_os = "android", target_os = "ios"))]
pub async fn send_to_pty(_input: String, _state: State<'_, AppState>) -> Result<(), String> {
    Err("PTY terminal is not supported on mobile targets".to_string())
}

#[tauri::command]
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub async fn send_to_pty_session(
    session_id: String,
    input: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use std::io::Write;
    let mut sessions = state.pty_sessions.lock().await;
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| format!("No PTY session available ({session_id})"))?;
    session
        .writer
        .write_all(input.as_bytes())
        .map_err(|e| e.to_string())?;
    session.writer.write_all(b"\r").map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
#[cfg(any(target_os = "android", target_os = "ios"))]
pub async fn send_to_pty_session(
    _session_id: String,
    _input: String,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    Err("PTY terminal is not supported on mobile targets".to_string())
}

#[tauri::command]
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub async fn resize_pty(rows: u16, cols: u16, state: State<'_, AppState>) -> Result<(), String> {
    use portable_pty::PtySize;
    let session_id = "default".to_string();
    let mut sessions = state.pty_sessions.lock().await;
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| format!("No PTY session available ({session_id})"))?;
    session
        .master
        .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
#[cfg(any(target_os = "android", target_os = "ios"))]
pub async fn resize_pty(_rows: u16, _cols: u16, _state: State<'_, AppState>) -> Result<(), String> {
    Err("PTY terminal is not supported on mobile targets".to_string())
}

#[tauri::command]
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub async fn resize_pty_session(
    session_id: String,
    rows: u16,
    cols: u16,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use portable_pty::PtySize;
    let mut sessions = state.pty_sessions.lock().await;
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| format!("No PTY session available ({session_id})"))?;
    session
        .master
        .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
#[cfg(any(target_os = "android", target_os = "ios"))]
pub async fn resize_pty_session(
    _session_id: String,
    _rows: u16,
    _cols: u16,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    Err("PTY terminal is not supported on mobile targets".to_string())
}

#[tauri::command]
pub async fn close_pty_session(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut sessions = state.pty_sessions.lock().await;
    sessions.remove(&session_id);
    Ok(())
}

// ─── Diff hunks ───────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn accept_diff_hunk(
    file_path: String,
    hunk_index: usize,
    diff: String,
) -> Result<(), String> {
    let original = fs::read_to_string(&file_path).map_err(|e| e.to_string())?;

    let lines: Vec<&str> = diff.lines().collect();
    let hunk_starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.starts_with("@@"))
        .map(|(i, _)| i)
        .collect();

    if hunk_index >= hunk_starts.len() {
        return Err(format!(
            "Hunk index {hunk_index} out of range (total: {})",
            hunk_starts.len()
        ));
    }

    let header: Vec<&str> = lines
        .iter()
        .take_while(|l| !l.starts_with("@@"))
        .cloned()
        .collect();

    let hunk_start = hunk_starts[hunk_index];
    let hunk_end   = hunk_starts.get(hunk_index + 1).copied().unwrap_or(lines.len());
    let hunk_lines = &lines[hunk_start..hunk_end];

    let single_diff = format!("{}\n{}\n", header.join("\n"), hunk_lines.join("\n"));

    let patch =
        diffy::Patch::from_str(&single_diff).map_err(|e| format!("Patch parse error: {e}"))?;
    let new_content =
        diffy::apply(&original, &patch).map_err(|e| format!("Patch apply error: {e}"))?;

    fs::write(&file_path, new_content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reject_diff_hunk(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    file_path: String,
    hunk_index: usize,
) -> Result<(), String> {
    // Reject is an explicit user action; file content is intentionally left unchanged.
    // We persist/emit it so UI and audits can reason about completed decisions.
    fs::metadata(&file_path).map_err(|e| format!("Cannot reject hunk for missing file: {e}"))?;

    let payload = json!({
        "file_path": file_path,
        "hunk_index": hunk_index,
    });
    let _ = state.wal.log_event("diff_hunk_rejected", payload.clone()).await;
    let _ = app_handle.emit("diff-hunk-rejected", payload);
    Ok(())
}

#[tauri::command]
pub async fn create_unified_diff(file_path: String, new_content: String) -> Result<String, String> {
    let current = fs::read_to_string(&file_path).unwrap_or_default();
    Ok(diffy::create_patch(&current, &new_content).to_string())
}

// ─── Models ──────────────────────────────────────────────────────────────────

/// Returns every GGUF model the registry found, serialized for the frontend.
#[tauri::command]
pub async fn list_models_registry(state: State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    let models = state.orchestrator.list_models().await;
    Ok(models
        .iter()
        .map(|m| serde_json::json!({
            "id":              m.id,
            "name":            m.name,
            "path":            m.path.to_string_lossy(),
            "architecture":    m.architecture,
            "parameter_count": m.parameter_count,
            "context_length":  m.context_length,
            "quant":           m.quant_label,
            "ram_required_mb": m.ram_required_mb,
            "ram_label":       m.ram_label(),
            "valid":           m.valid,
        }))
        .collect())
}

/// Legacy stub kept for frontend compatibility; now delegates to the registry.
#[tauri::command]
pub async fn list_available_models(state: State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    list_models_registry(state).await
}

/// Load a specific model by registry ID into the orchestrator.
#[tauri::command]
pub async fn load_model(
    model_id: String,
    state: State<'_, AppState>,
) -> Result<(), BonsaiError> {
    let rx = state.orchestrator.load(model_id);
    rx.await.map_err(|_| BonsaiError::Orchestrator("Orchestrator offline".to_string()))?.map_err(BonsaiError::Orchestrator)
}

/// Unload a specific slot by index.
#[tauri::command]
pub async fn unload_slot(slot: usize, state: State<'_, AppState>) -> Result<(), String> {
    state.orchestrator.unload(slot);
    Ok(())
}

/// Switch the active model (loads it; kicks off LRU eviction if needed).
#[tauri::command]
pub async fn switch_model(
    model_id: String,
    state: State<'_, AppState>,
) -> Result<String, BonsaiError> {
    let rx = state.orchestrator.load(model_id.clone());
    rx.await
        .map_err(|_| BonsaiError::Orchestrator("Orchestrator offline".to_string()))?
        .map_err(BonsaiError::Orchestrator)?;

    let models = state.orchestrator.list_models().await;
    let model_name = models
        .into_iter()
        .find(|m| m.id == model_id)
        .map(|m| m.name)
        .unwrap_or_else(|| model_id.clone());

    Ok(format!("Model {model_name} is now active"))
}

/// Snapshot of every slot's state + queue depth + system RAM.
#[tauri::command]
pub async fn get_orchestrator_status(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let s = state.orchestrator.status().await;
    Ok(serde_json::to_value(s).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn get_task_queue_status(state: State<'_, AppState>) -> Result<TaskQueueStatus, String> {
    Ok(state.task_queue.status().await)
}

// ─── Cluster Orchestrator ───────────────────────────────────────────────────

#[tauri::command]
pub async fn cluster_list_nodes(state: State<'_, AppState>) -> Result<Vec<ClusterNode>, String> {
    if !crate::features::FeatureFlags::is_enabled("cluster_orchestrator") {
        return Err("Cluster orchestrator feature is disabled".into());
    }
    let cluster = state.cluster_orchestrator.lock().await;
    Ok(cluster.list_nodes())
}

#[tauri::command]
pub async fn cluster_upsert_node(
    node: ClusterNode,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    if !crate::features::FeatureFlags::is_enabled("cluster_orchestrator") {
        return Err("Cluster orchestrator feature is disabled".into());
    }
    let mut cluster = state.cluster_orchestrator.lock().await;
    cluster.upsert_node(node.clone());
    Ok(serde_json::json!({
        "ok": true,
        "node_id": node.node_id,
    }))
}

#[tauri::command]
pub async fn cluster_remove_node(
    node_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    if !crate::features::FeatureFlags::is_enabled("cluster_orchestrator") {
        return Err("Cluster orchestrator feature is disabled".into());
    }
    let mut cluster = state.cluster_orchestrator.lock().await;
    let removed = cluster.remove_node(node_id.trim());
    Ok(serde_json::json!({
        "ok": removed,
        "node_id": node_id,
    }))
}

#[tauri::command]
pub async fn cluster_update_node_metrics(
    node_id: String,
    metrics: NodeRuntimeMetrics,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    if !crate::features::FeatureFlags::is_enabled("cluster_orchestrator") {
        return Err("Cluster orchestrator feature is disabled".into());
    }
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as u64;

    let mut cluster = state.cluster_orchestrator.lock().await;
    let updated = cluster.update_node_metrics(node_id.trim(), metrics, now_ms);
    Ok(serde_json::json!({
        "ok": updated,
        "node_id": node_id,
        "last_seen_ms": now_ms,
    }))
}

#[tauri::command]
pub async fn cluster_set_policy(
    policy: ClusterPolicy,
    state: State<'_, AppState>,
) -> Result<ClusterPolicy, String> {
    if !crate::features::FeatureFlags::is_enabled("cluster_orchestrator") {
        return Err("Cluster orchestrator feature is disabled".into());
    }
    let mut cluster = state.cluster_orchestrator.lock().await;
    cluster.set_policy(policy);
    Ok(cluster.policy().clone())
}

#[tauri::command]
pub async fn cluster_get_policy(state: State<'_, AppState>) -> Result<ClusterPolicy, String> {
    if !crate::features::FeatureFlags::is_enabled("cluster_orchestrator") {
        return Err("Cluster orchestrator feature is disabled".into());
    }
    let cluster = state.cluster_orchestrator.lock().await;
    Ok(cluster.policy().clone())
}

#[tauri::command]
pub async fn cluster_plan_workload(
    workload: ClusterWorkload,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    if !crate::features::FeatureFlags::is_enabled("cluster_orchestrator") {
        return Err("Cluster orchestrator feature is disabled".into());
    }
    let cluster = state.cluster_orchestrator.lock().await;
    let plan = cluster.plan_workload(&workload);
    Ok(serde_json::to_value(plan).map_err(|e| e.to_string())?)
}

struct GpuInfo {
    name:    String,
    backend: String,
}

/// Collect GPU names from the OS and classify the inference backend.
/// Returns one GpuInfo per detected GPU (discrete or integrated).
fn detect_gpus() -> Vec<GpuInfo> {
    let raw_names = collect_raw_gpu_names();
    raw_names.into_iter().map(|name| {
        let lower = name.to_lowercase();
        let backend = if lower.contains("nvidia") {
            "CUDA".to_string()
        } else if lower.contains("amd") || lower.contains("radeon") {
            // ROCm on Linux; Vulkan/DirectML on Windows
            if cfg!(target_os = "linux") { "ROCm".to_string() } else { "Vulkan / DirectML".to_string() }
        } else if lower.contains("intel") {
            // Intel Xe / Arc discrete → SYCL; UHD / Iris = iGPU → OpenCL / DirectML
            if lower.contains("arc") || lower.contains("xe") {
                "SYCL / DirectML".to_string()
            } else {
                "iGPU / OpenCL".to_string()
            }
        } else if lower.contains("apple") || lower.contains("m1") || lower.contains("m2") || lower.contains("m3") || lower.contains("m4") {
            "Metal".to_string()
        } else {
            "CPU".to_string()
        };
        GpuInfo { name, backend }
    }).collect()
}

fn collect_raw_gpu_names() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        let args = ["path", "win32_VideoController", "get", "name"];
        let mut wmic_cmd = Command::new("wmic");
        wmic_cmd.args(&args);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            wmic_cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        if let Ok(output) = wmic_cmd.output() {
            let names: Vec<String> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .skip(1)
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            if !names.is_empty() {
                return names;
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("lspci").output() {
            let names: Vec<String> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| {
                    let lower = line.to_lowercase();
                    if lower.contains("vga compatible controller") || lower.contains("3d controller") || lower.contains("display controller") {
                        // Strip the PCI address prefix
                        Some(line.splitn(2, ':').nth(1).unwrap_or(line).trim().to_string())
                    } else {
                        None
                    }
                })
                .collect();
            if !names.is_empty() {
                return names;
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        return vec!["Apple Silicon / Metal".to_string()];
    }

    #[allow(unreachable_code)] { vec![] }
}

#[tauri::command]
pub async fn get_api_port(app_handle: AppHandle) -> Result<u16, String> {
    let config = crate::config::load_config(&app_handle)?;
    Ok(config.api_port)
}

#[tauri::command]
pub async fn get_buddy_api_port(state: State<'_, AppState>) -> Result<u16, String> {
    Ok(state.buddy_api_port)
}

#[tauri::command]
pub async fn get_api_config(app_handle: AppHandle) -> Result<serde_json::Value, String> {
    let config = crate::config::load_config(&app_handle)?;
    Ok(serde_json::json!({
        "api_host": config.api_host,
        "api_port": config.api_port,
    }))
}

#[tauri::command]
pub async fn set_api_config(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    api_host: String,
    api_port: u16,
) -> Result<serde_json::Value, String> {
    let old_config = crate::config::load_config(&app_handle)?;

    // Apply API host/port changes immediately by replacing the running server.
    let remote_manager = app_handle.state::<Arc<RemoteManager>>().inner().clone();
    let mut api_guard = state.api_server.lock().await;

    if let Some(mut running) = api_guard.take() {
        running.stop().await;
    }

    let started = api_server::start(
        state.orchestrator.clone(),
        remote_manager,
        state.ws_router.clone(),
        state.pair_token.clone(),
        api_host.clone(),
        api_port,
        app_handle.clone(),
    )
    .await;

    let new_server = match started {
        Ok(s) => s,
        Err(e) => {
            // Try to restore previous config runtime so the app keeps working.
            let rollback_remote = app_handle.state::<Arc<RemoteManager>>().inner().clone();
            if let Ok(restored) = api_server::start(
                state.orchestrator.clone(),
                rollback_remote,
                state.ws_router.clone(),
                state.pair_token.clone(),
                old_config.api_host.clone(),
                old_config.api_port,
                app_handle.clone(),
            )
            .await
            {
                *api_guard = Some(restored);
            }
            return Err(format!("Failed to restart API server: {e}"));
        }
    };

    let mut config = old_config;
    config.api_host = new_server.host.clone();
    config.api_port = new_server.port;
    let config = crate::config::save_config(&app_handle, &config)?;

    *api_guard = Some(new_server);

    Ok(serde_json::json!({
        "api_host": config.api_host,
        "api_port": config.api_port,
    }))
}

#[tauri::command]
pub async fn get_current_session_state(app_handle: AppHandle) -> Result<serde_json::Value, String> {
    let config = crate::config::load_config(&app_handle)?;
    Ok(serde_json::json!({
        "current_session_id": config.current_session_id,
        "current_session_title": config.current_session_title,
    }))
}

#[tauri::command]
pub async fn set_current_session_state(
    app_handle: AppHandle,
    session_id: Option<String>,
    title: Option<String>,
) -> Result<serde_json::Value, String> {
    let mut config = crate::config::load_config(&app_handle)?;
    config.current_session_id = session_id;
    config.current_session_title = title;
    let config = crate::config::save_config(&app_handle, &config)?;
    Ok(serde_json::json!({
        "current_session_id": config.current_session_id,
        "current_session_title": config.current_session_title,
    }))
}

#[tauri::command]
pub async fn start_remote_session(
    remote_manager: State<'_, Arc<RemoteManager>>,
) -> Result<serde_json::Value, String> {
    let session = remote_manager.start_session().await?;
    Ok(serde_json::json!({
        "session_id": session.id,
        "state": session.state,
    }))
}

#[tauri::command]
pub async fn stop_remote_session(
    remote_manager: State<'_, Arc<RemoteManager>>,
) -> Result<(), String> {
    remote_manager.stop_session().await
}

#[tauri::command]
pub async fn send_remote_input(
    remote_manager: State<'_, Arc<RemoteManager>>,
    event: RemoteInputEvent,
) -> Result<serde_json::Value, String> {
    remote_manager.submit_input(event).await?;
    Ok(serde_json::json!({ "status": "accepted" }))
}

#[tauri::command]
pub async fn get_hardware_info() -> Result<serde_json::Value, String> {
    let mut sys = System::new_all();
    sys.refresh_all();
    let ram_gb   = sys.total_memory() / 1024 / 1024 / 1024;
    let avail_gb = sys.available_memory() / 1024 / 1024 / 1024;

    let gpus = detect_gpus();
    let (gpu_names, backends): (Vec<_>, Vec<_>) = if gpus.is_empty() {
        (vec!["None detected".to_string()], vec!["CPU".to_string()])
    } else {
        gpus.iter().map(|g| (g.name.clone(), g.backend.clone())).unzip()
    };
    // De-duplicate backends (e.g. two NVIDIA GPUs → one "CUDA" entry)
    let mut unique_backends: Vec<String> = vec![];
    for b in &backends {
        if !unique_backends.contains(b) { unique_backends.push(b.clone()); }
    }

    Ok(serde_json::json!({
        "ram_total_gb":     ram_gb,
        "ram_available_gb": avail_gb,
        "cpu_count":        sys.cpus().len(),
        "backend":          unique_backends.join(" / "),
        "gpu_names":        gpu_names,
    }))
}

#[tauri::command]
pub async fn prompt_gguf_import(app_handle: AppHandle) -> Result<String, String> {
    let path = app_handle
        .dialog()
        .file()
        .add_filter("GGUF Model", &["gguf"])
        .blocking_pick_file()
        .map(|p| p.to_string())
        .ok_or_else(|| "No file selected".to_string())?;
    Ok(path)
}

// ─── Bootstrap ───────────────────────────────────────────────────────────────

/// Returns the current bootstrap status (which binaries/models are present).
#[tauri::command]
pub async fn check_bootstrap_status(app_handle: AppHandle) -> Result<serde_json::Value, String> {
    let s = bootstrap::check_status(&app_handle);
    Ok(serde_json::json!({
        "llama_ready":   s.llama_ready,
        "whisper_ready": s.whisper_ready,
        "model_ready":   s.model_ready,
        "all_ready":     s.all_ready(),
    }))
}

/// Manually trigger the bootstrap flow (idempotent — skips anything already present).
#[tauri::command]
pub async fn run_bootstrap(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    // Reset any previous cancellation before starting a fresh run
    state.bootstrap_cancel.store(false, Ordering::Relaxed);

    let orch   = state.orchestrator.clone();
    let cancel = state.bootstrap_cancel.clone();
    let bh     = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        match bootstrap::run(bh.clone(), cancel).await {
            Ok(()) => {
                orch.refresh_registry();
                let _ = bh.emit("bootstrap-complete", ());
            }
            Err(e) => {
                tracing::error!(error=%e, "[bootstrap] run_bootstrap error");
                let _ = bh.emit("bootstrap-error", e.to_string());
            }
        }
    });
    Ok(())
}

/// Cancel any in-progress bootstrap download.
#[tauri::command]
pub async fn cancel_bootstrap(state: State<'_, AppState>) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    state.bootstrap_cancel.store(true, Ordering::Relaxed);
    Ok(())
}

// ─── Download ────────────────────────────────────────────────────────────────

async fn download_to_file(
    app_handle: &AppHandle,
    url: &str,
    file_name: &str,
    event_tag: &str,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}: {url}", resp.status()));
    }
    let total = resp.content_length().unwrap_or(0);

    let app_data = {
        use tauri::Manager;
        app_handle
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
    };
    let models_dir = app_data.join("models");
    fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;
    let save_path = models_dir.join(file_name);

    let mut file       = fs::File::create(&save_path).map_err(|e| e.to_string())?;
    let mut downloaded = 0u64;
    let mut stream     = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        use std::io::Write;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        let pct = if total > 0 { downloaded * 100 / total } else { 0 };
        let _ = app_handle.emit(
            event_tag,
            serde_json::json!({ "progress": pct, "downloaded": downloaded, "total": total }),
        );
    }

    Ok(save_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn download_gguf_model(
    app_handle: AppHandle,
    url: String,
    file_name: String,
) -> Result<String, String> {
    download_to_file(&app_handle, &url, &file_name, "download-progress").await
}

#[tauri::command]
pub async fn download_whisper_model(app_handle: AppHandle) -> Result<String, String> {
    download_to_file(
        &app_handle,
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
        "ggml-base.en.bin",
        "download-progress",
    )
    .await
}

// ─── Connection / pairing commands ───────────────────────────────────────────

/// Returns the 8-char alphanumeric pairing token displayed in Settings.
#[tauri::command]
pub async fn get_pair_token(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.pair_token.clone())
}

/// Returns the first non-loopback LAN IPv4 address of this machine.
#[tauri::command]
pub async fn get_local_ip() -> Result<String, String> {
    local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .map_err(|e| e.to_string())
}

/// Generates an SVG QR code encoding
/// `bonsai://connect?ip=<local_ip>&port=<api_port>&token=<pair_token>`.
#[tauri::command]
pub async fn generate_pair_qr(state: State<'_, AppState>) -> Result<String, String> {
    use qrcode::QrCode;
    use qrcode::render::svg;

    let ip   = local_ip_address::local_ip().map(|i| i.to_string()).unwrap_or_else(|_| "127.0.0.1".into());
    let data = format!(
        "bonsai://connect?ip={}&port={}&token={}",
        ip,
        crate::config::DEFAULT_API_PORT,
        state.pair_token
    );
    let code = QrCode::new(data.as_bytes()).map_err(|e| e.to_string())?;
    let svg  = code.render::<svg::Color>()
        .min_dimensions(200, 200)
        .build();
    Ok(svg)
}

/// Broadcasts an arbitrary JSON payload to all connected WebSocket clients.
/// Useful for pushing chat token streams to the Android app.
#[tauri::command]
pub async fn ws_broadcast(
    state: State<'_, AppState>,
    payload: Value,
) -> Result<(), String> {
    use axum::extract::ws::Message;
    let txt = payload.to_string();
    state.ws_router.broadcast(Message::Text(txt));
    Ok(())
}

/// Returns the number of active WebSocket clients (Android + VSCode extensions).
#[tauri::command]
pub async fn ws_client_count(state: State<'_, AppState>) -> Result<usize, String> {
    Ok(state.ws_router.client_count())
}

fn resolve_adb_executable() -> (String, Vec<String>) {
    let mut candidates = Vec::<String>::new();

    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        candidates.push(format!(
            "{}\\Android\\Sdk\\platform-tools\\adb.exe",
            local_app_data
        ));
    }
    if let Ok(android_home) = std::env::var("ANDROID_HOME") {
        candidates.push(format!("{}\\platform-tools\\adb.exe", android_home));
    }
    if let Ok(android_sdk_root) = std::env::var("ANDROID_SDK_ROOT") {
        candidates.push(format!("{}\\platform-tools\\adb.exe", android_sdk_root));
    }

    for c in &candidates {
        if std::path::Path::new(c).exists() {
            return (c.clone(), candidates);
        }
    }

    ("adb".to_string(), candidates)
}

const DEFAULT_ADB_TIMEOUT_MS: u64 = 120_000;
const LONG_ADB_TIMEOUT_MS: u64 = 300_000;

fn mobile_command_cancel_requested() -> &'static AtomicBool {
    static CANCEL_REQUESTED: OnceLock<AtomicBool> = OnceLock::new();
    CANCEL_REQUESTED.get_or_init(|| AtomicBool::new(false))
}

fn run_command_with_timeout(
    mut cmd: Command,
    timeout_ms: u64,
    label: &str,
) -> Result<std::process::Output, String> {
    use std::thread;
    use std::time::{Duration, Instant};

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start {label}: {e}"))?;

    let start = Instant::now();
    let poll = Duration::from_millis(100);

    loop {
        if mobile_command_cancel_requested().load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("{label} canceled by request."));
        }

        if let Some(_) = child.try_wait().map_err(|e| format!("Failed to poll {label}: {e}"))? {
            return child
                .wait_with_output()
                .map_err(|e| format!("Failed to collect {label} output: {e}"));
        }

        if start.elapsed() >= Duration::from_millis(timeout_ms.max(1_000)) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "{label} timed out after {} ms. The process was terminated.",
                timeout_ms.max(1_000)
            ));
        }

        thread::sleep(poll);
    }
}

fn adb_run_with_timeout(args: &[String], timeout_ms: u64) -> Result<serde_json::Value, String> {
    mobile_command_cancel_requested().store(false, Ordering::Relaxed);

    let (adb_executable, candidates) = resolve_adb_executable();
    let mut cmd = Command::new(&adb_executable);
    cmd.args(args);

    let output = run_command_with_timeout(cmd, timeout_ms, "adb command").map_err(|e| {
        if e.contains("canceled by request") || e.contains("timed out") {
            return e;
        }

        format!(
            "Failed to execute adb via '{}'. Ensure Android Platform Tools are installed. Candidate locations checked: {}. {}",
            adb_executable,
            candidates.join(", "),
            e
        )
    })?;

    let status_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    Ok(serde_json::json!({
        "ok": output.status.success(),
        "status": status_code,
        "stdout": stdout,
        "stderr": stderr,
        "args": args,
        "adb_executable": adb_executable,
    }))
}

fn adb_run(args: &[String]) -> Result<serde_json::Value, String> {
    adb_run_with_timeout(args, DEFAULT_ADB_TIMEOUT_MS)
}

fn resolve_executable_from_path(name: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let out = Command::new("where").arg(name).output().ok()?;
        if !out.status.success() {
            return None;
        }

        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let candidate = line.trim();
            if candidate.is_empty() {
                continue;
            }
            if std::path::Path::new(candidate).exists() {
                return Some(candidate.to_string());
            }
        }
        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        let out = Command::new("which").arg(name).output().ok()?;
        if !out.status.success() {
            return None;
        }

        let candidate = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if candidate.is_empty() {
            None
        } else {
            Some(candidate)
        }
    }
}

#[tauri::command]
pub async fn android_mobile_cancel_pending_operations() -> Result<serde_json::Value, String> {
    mobile_command_cancel_requested().store(true, Ordering::Relaxed);
    Ok(json!({ "ok": true, "message": "Cancel requested for pending mobile commands." }))
}

fn adb_assert_ok(result: &serde_json::Value, label: &str) -> Result<(), String> {
    let ok = result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    if ok {
        return Ok(());
    }

    let status = result.get("status").and_then(|v| v.as_i64()).unwrap_or(-1);
    let stderr = result.get("stderr").and_then(|v| v.as_str()).unwrap_or("");
    let stdout = result.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
    Err(format!("{label} failed (exit {status}). stderr: {stderr}. stdout: {stdout}"))
}

#[derive(Clone)]
struct MobileRecordingSession {
    pid: u32,
    remote_path: String,
}

fn mobile_view_sessions() -> &'static StdMutex<HashMap<String, u32>> {
    static SESSIONS: OnceLock<StdMutex<HashMap<String, u32>>> = OnceLock::new();
    SESSIONS.get_or_init(|| StdMutex::new(HashMap::new()))
}

fn mobile_recording_sessions() -> &'static StdMutex<HashMap<String, MobileRecordingSession>> {
    static RECORDINGS: OnceLock<StdMutex<HashMap<String, MobileRecordingSession>>> = OnceLock::new();
    RECORDINGS.get_or_init(|| StdMutex::new(HashMap::new()))
}

fn kill_process_tree(pid: u32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let out = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output()
            .map_err(|e| format!("Failed to run taskkill: {e}"))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            return Err(format!("taskkill failed for pid {pid}. stderr: {stderr}. stdout: {stdout}"));
        }
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let out = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output()
            .map_err(|e| format!("Failed to run kill: {e}"))?;
        if out.status.success() {
            return Ok(());
        }

        let out_force = Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .output()
            .map_err(|e| format!("Failed to run kill -KILL: {e}"))?;
        if !out_force.status.success() {
            let stderr = String::from_utf8_lossy(&out_force.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&out_force.stdout).trim().to_string();
            return Err(format!("kill failed for pid {pid}. stderr: {stderr}. stdout: {stdout}"));
        }
        Ok(())
    }
}

fn resolve_scrcpy_executable() -> (String, Vec<String>) {
    let mut candidates = Vec::<String>::new();

    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            candidates.push(format!("{}\\Programs\\scrcpy\\scrcpy.exe", local_app_data));
            candidates.push(format!("{}\\Programs\\scrcpy-win64\\scrcpy.exe", local_app_data));
            candidates.push(format!("{}\\Microsoft\\WinGet\\Links\\scrcpy.exe", local_app_data));
        }
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            candidates.push(format!("{}\\scrcpy\\scrcpy.exe", program_files));
        }
        if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
            candidates.push(format!("{}\\scrcpy\\scrcpy.exe", program_files_x86));
        }
        if let Ok(program_data) = std::env::var("ProgramData") {
            candidates.push(format!("{}\\chocolatey\\bin\\scrcpy.exe", program_data));
        }
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            candidates.push(format!("{}\\scoop\\apps\\scrcpy\\current\\scrcpy.exe", user_profile));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        candidates.push("/usr/bin/scrcpy".to_string());
        candidates.push("/usr/local/bin/scrcpy".to_string());
    }

    for c in &candidates {
        if std::path::Path::new(c).exists() {
            return (c.clone(), candidates);
        }
    }

    if let Some(from_path) = resolve_executable_from_path("scrcpy") {
        candidates.push(from_path.clone());
        return (from_path, candidates);
    }

    ("scrcpy".to_string(), candidates)
}

fn ensure_mobile_artifact_dir(app_handle: &AppHandle, category: &str) -> Result<std::path::PathBuf, String> {
    use tauri::Manager as _;

    let base = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("mobile-view").join(category);
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create artifact dir {}: {e}", dir.to_string_lossy()))?;
    Ok(dir)
}

/// Inspect Mobile View runtime capabilities and active sessions.
#[tauri::command]
pub async fn android_mobile_view_status() -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let (adb_executable, adb_candidates) = resolve_adb_executable();
        let (scrcpy_executable, scrcpy_candidates) = resolve_scrcpy_executable();
        let scrcpy_exists = std::path::Path::new(&scrcpy_executable).exists();

        let active_views = {
            let sessions = mobile_view_sessions()
                .lock()
                .map_err(|_| "mobile view session lock poisoned".to_string())?;
            sessions
                .iter()
                .map(|(serial, pid)| json!({ "serial": serial, "pid": pid }))
                .collect::<Vec<Value>>()
        };

        let active_recordings = {
            let sessions = mobile_recording_sessions()
                .lock()
                .map_err(|_| "mobile recording session lock poisoned".to_string())?;
            sessions
                .iter()
                .map(|(serial, rec)| json!({ "serial": serial, "pid": rec.pid, "remote_path": rec.remote_path }))
                .collect::<Vec<Value>>()
        };

        Ok(json!({
            "adb_executable": adb_executable,
            "adb_candidates": adb_candidates,
            "scrcpy_executable": scrcpy_executable,
            "scrcpy_candidates": scrcpy_candidates,
            "scrcpy_available": scrcpy_exists,
            "active_views": active_views,
            "active_recordings": active_recordings,
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Start a scrcpy live control session for the selected device.
#[tauri::command]
pub async fn android_mobile_view_start(
    serial: String,
    max_size: Option<u32>,
    bitrate_mbps: Option<u32>,
    fullscreen: Option<bool>,
    stay_awake: Option<bool>,
    turn_screen_off: Option<bool>,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let serial = serial.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }

        let (scrcpy_executable, candidates) = resolve_scrcpy_executable();

        let mut cmd = Command::new(&scrcpy_executable);
        cmd.args(["-s", &serial]);
        cmd.arg("--window-title").arg(format!("Bonsai Mobile View - {serial}"));

        if let Some(size) = max_size {
            if size >= 240 {
                cmd.arg("--max-size").arg(size.to_string());
            }
        }

        if let Some(bitrate) = bitrate_mbps {
            if bitrate > 0 {
                cmd.arg("--video-bit-rate").arg(format!("{bitrate}M"));
            }
        }

        if fullscreen.unwrap_or(false) {
            cmd.arg("--fullscreen");
        }
        if stay_awake.unwrap_or(true) {
            cmd.arg("--stay-awake");
        }
        if turn_screen_off.unwrap_or(false) {
            cmd.arg("--turn-screen-off");
        }

        let child = cmd.spawn().map_err(|e| {
            format!(
                "Failed to start scrcpy via '{}'. Ensure scrcpy is installed. Candidate locations checked: {}. {}",
                scrcpy_executable,
                candidates.join(", "),
                e,
            )
        })?;

        let pid = child.id();

        let mut sessions = mobile_view_sessions()
            .lock()
            .map_err(|_| "mobile view session lock poisoned".to_string())?;

        if let Some(old_pid) = sessions.insert(serial.clone(), pid) {
            let _ = kill_process_tree(old_pid);
        }

        Ok(json!({
            "ok": true,
            "serial": serial,
            "pid": pid,
            "scrcpy_executable": scrcpy_executable,
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Stop a running scrcpy live control session.
#[tauri::command]
pub async fn android_mobile_view_stop(serial: String) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let serial = serial.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }

        let pid = {
            let mut sessions = mobile_view_sessions()
                .lock()
                .map_err(|_| "mobile view session lock poisoned".to_string())?;
            sessions.remove(&serial)
        };

        let Some(pid) = pid else {
            return Ok(json!({
                "ok": true,
                "serial": serial,
                "stopped": false,
                "message": "No running Mobile View session for this serial.",
            }));
        };

        kill_process_tree(pid)?;

        Ok(json!({
            "ok": true,
            "serial": serial,
            "stopped": true,
            "pid": pid,
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Capture a screenshot from the connected Android device.
#[tauri::command]
pub async fn android_mobile_take_screenshot(
    app_handle: AppHandle,
    serial: String,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        use std::time::{SystemTime, UNIX_EPOCH};

        let serial = serial.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis();

        let output_dir = ensure_mobile_artifact_dir(&app_handle, "screenshots")?;
        let output_path = output_dir.join(format!("{serial}-{timestamp}.png"));

        let (adb_executable, candidates) = resolve_adb_executable();
        let out = Command::new(&adb_executable)
            .args(["-s", &serial, "exec-out", "screencap", "-p"])
            .output()
            .map_err(|e| {
                format!(
                    "Failed to run adb screenshot via '{}'. Candidate locations checked: {}. {}",
                    adb_executable,
                    candidates.join(", "),
                    e,
                )
            })?;

        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            return Err(format!("adb exec-out screencap failed. stderr: {stderr}. stdout: {stdout}"));
        }

        fs::write(&output_path, &out.stdout)
            .map_err(|e| format!("Failed writing screenshot {}: {e}", output_path.to_string_lossy()))?;

        Ok(json!({
            "ok": true,
            "serial": serial,
            "path": output_path.to_string_lossy().to_string(),
            "size_bytes": out.stdout.len(),
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Start device-side screen recording (adb shell screenrecord).
#[tauri::command]
pub async fn android_mobile_start_recording(
    serial: String,
    bitrate_mbps: Option<u32>,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        use std::time::{SystemTime, UNIX_EPOCH};

        let serial = serial.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }

        {
            let sessions = mobile_recording_sessions()
                .lock()
                .map_err(|_| "mobile recording session lock poisoned".to_string())?;
            if let Some(existing) = sessions.get(&serial) {
                return Err(format!("Recording already active for {serial} (pid {}). Stop it first.", existing.pid));
            }
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis();
        let remote_path = format!("/sdcard/Movies/bonsai-recording-{timestamp}.mp4");

        let (adb_executable, candidates) = resolve_adb_executable();
        let mut cmd = Command::new(&adb_executable);
        cmd.args(["-s", &serial, "shell", "screenrecord"]);

        if let Some(rate) = bitrate_mbps {
            if rate > 0 {
                cmd.arg("--bit-rate").arg((rate * 1_000_000).to_string());
            }
        }

        cmd.arg(&remote_path);

        let child = cmd.spawn().map_err(|e| {
            format!(
                "Failed to start adb screenrecord via '{}'. Candidate locations checked: {}. {}",
                adb_executable,
                candidates.join(", "),
                e,
            )
        })?;
        let pid = child.id();

        let mut sessions = mobile_recording_sessions()
            .lock()
            .map_err(|_| "mobile recording session lock poisoned".to_string())?;
        sessions.insert(
            serial.clone(),
            MobileRecordingSession {
                pid,
                remote_path: remote_path.clone(),
            },
        );

        Ok(json!({
            "ok": true,
            "serial": serial,
            "pid": pid,
            "remote_path": remote_path,
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Stop recording, pull the video file to desktop, and clean up remote artifact.
#[tauri::command]
pub async fn android_mobile_stop_recording(
    app_handle: AppHandle,
    serial: String,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        use std::time::{SystemTime, UNIX_EPOCH};

        let serial = serial.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }

        let rec = {
            let mut sessions = mobile_recording_sessions()
                .lock()
                .map_err(|_| "mobile recording session lock poisoned".to_string())?;
            sessions.remove(&serial)
        };

        let Some(rec) = rec else {
            return Err(format!("No active recording found for {serial}"));
        };

        // Stop adb screenrecord process first.
        let _ = kill_process_tree(rec.pid);

        let output_dir = ensure_mobile_artifact_dir(&app_handle, "recordings")?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis();
        let output_path = output_dir.join(format!("{serial}-{timestamp}.mp4"));

        let pull_args = vec![
            "-s".to_string(),
            serial.clone(),
            "pull".to_string(),
            rec.remote_path.clone(),
            output_path.to_string_lossy().to_string(),
        ];
        let pull = adb_run_with_timeout(&pull_args, LONG_ADB_TIMEOUT_MS)?;
        adb_assert_ok(&pull, "adb pull screenrecord")?;

        let _ = adb_run_with_timeout(&vec![
            "-s".to_string(),
            serial.clone(),
            "shell".to_string(),
            "rm".to_string(),
            rec.remote_path.clone(),
        ], LONG_ADB_TIMEOUT_MS);

        Ok(json!({
            "ok": true,
            "serial": serial,
            "path": output_path.to_string_lossy().to_string(),
            "remote_path": rec.remote_path,
            "pid": rec.pid,
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Launch the default camera app in photo or video capture mode.
#[tauri::command]
pub async fn android_mobile_launch_camera(
    serial: String,
    video_mode: Option<bool>,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let serial = serial.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }

        let action = if video_mode.unwrap_or(false) {
            "android.media.action.VIDEO_CAPTURE"
        } else {
            "android.media.action.IMAGE_CAPTURE"
        };

        let args = vec![
            "-s".to_string(),
            serial,
            "shell".to_string(),
            "am".to_string(),
            "start".to_string(),
            "-a".to_string(),
            action.to_string(),
        ];
        let out = adb_run(&args)?;
        adb_assert_ok(&out, "adb launch camera")?;
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Send Android key events (home/back/recent/volume/power, etc.).
#[tauri::command]
pub async fn android_mobile_send_key(serial: String, key_code: i32) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let serial = serial.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }
        if key_code <= 0 {
            return Err("key_code must be > 0".to_string());
        }

        let args = vec![
            "-s".to_string(),
            serial,
            "shell".to_string(),
            "input".to_string(),
            "keyevent".to_string(),
            key_code.to_string(),
        ];

        let out = adb_run(&args)?;
        adb_assert_ok(&out, "adb input keyevent")?;
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Send text input to device.
#[tauri::command]
pub async fn android_mobile_send_text(serial: String, text: String) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let serial = serial.trim().to_string();
        let text = text.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }
        if text.is_empty() {
            return Err("text cannot be empty".to_string());
        }

        let escaped = text
            .replace(" ", "%s")
            .replace("&", "\\&")
            .replace("|", "\\|")
            .replace("<", "\\<")
            .replace(">", "\\>")
            .replace(";", "\\;");

        let args = vec![
            "-s".to_string(),
            serial,
            "shell".to_string(),
            "input".to_string(),
            "text".to_string(),
            escaped,
        ];

        let out = adb_run(&args)?;
        adb_assert_ok(&out, "adb input text")?;
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Send tap coordinates.
#[tauri::command]
pub async fn android_mobile_tap(serial: String, x: u32, y: u32) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let serial = serial.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }

        let args = vec![
            "-s".to_string(),
            serial,
            "shell".to_string(),
            "input".to_string(),
            "tap".to_string(),
            x.to_string(),
            y.to_string(),
        ];

        let out = adb_run(&args)?;
        adb_assert_ok(&out, "adb input tap")?;
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Send swipe gesture coordinates.
#[tauri::command]
pub async fn android_mobile_swipe(
    serial: String,
    x1: u32,
    y1: u32,
    x2: u32,
    y2: u32,
    duration_ms: Option<u32>,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let serial = serial.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }

        let mut args = vec![
            "-s".to_string(),
            serial,
            "shell".to_string(),
            "input".to_string(),
            "swipe".to_string(),
            x1.to_string(),
            y1.to_string(),
            x2.to_string(),
            y2.to_string(),
        ];
        if let Some(ms) = duration_ms {
            args.push(ms.to_string());
        }

        let out = adb_run(&args)?;
        adb_assert_ok(&out, "adb input swipe")?;
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Read display metrics from the selected Android device.
#[tauri::command]
pub async fn android_mobile_get_display_info(serial: String) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let serial = serial.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }

        let size = adb_run(&vec![
            "-s".to_string(),
            serial.clone(),
            "shell".to_string(),
            "wm".to_string(),
            "size".to_string(),
        ])?;
        adb_assert_ok(&size, "adb shell wm size")?;

        let density = adb_run(&vec![
            "-s".to_string(),
            serial.clone(),
            "shell".to_string(),
            "wm".to_string(),
            "density".to_string(),
        ])?;
        adb_assert_ok(&density, "adb shell wm density")?;

        let input = adb_run(&vec![
            "-s".to_string(),
            serial.clone(),
            "shell".to_string(),
            "dumpsys".to_string(),
            "input".to_string(),
        ])?;
        adb_assert_ok(&input, "adb shell dumpsys input")?;

        let size_stdout = size.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
        let density_stdout = density.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
        let input_stdout = input.get("stdout").and_then(|v| v.as_str()).unwrap_or("");

        let mut width: Option<u32> = None;
        let mut height: Option<u32> = None;
        for line in size_stdout.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("Physical size:") {
                let parts: Vec<&str> = rest.trim().split('x').collect();
                if parts.len() == 2 {
                    width = parts[0].trim().parse::<u32>().ok();
                    height = parts[1].trim().parse::<u32>().ok();
                }
            }
        }

        let mut dpi: Option<u32> = None;
        for line in density_stdout.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("Physical density:") {
                dpi = rest.trim().parse::<u32>().ok();
            }
        }

        let mut surface_orientation: Option<u32> = None;
        for line in input_stdout.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("SurfaceOrientation:") {
                surface_orientation = rest.trim().parse::<u32>().ok();
                break;
            }
        }

        Ok(json!({
            "ok": true,
            "serial": serial,
            "width": width,
            "height": height,
            "density_dpi": dpi,
            "surface_orientation": surface_orientation,
            "wm_size_raw": size_stdout,
            "wm_density_raw": density_stdout,
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Lock or unlock orientation for stable Mobile View sessions.
/// Allowed modes: portrait, landscape, unlock.
#[tauri::command]
pub async fn android_mobile_set_orientation(
    serial: String,
    mode: String,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let serial = serial.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }

        let mode = mode.trim().to_lowercase();
        match mode.as_str() {
            "portrait" => {
                let rotate_off = adb_run(&vec![
                    "-s".to_string(), serial.clone(), "shell".to_string(),
                    "settings".to_string(), "put".to_string(), "system".to_string(),
                    "accelerometer_rotation".to_string(), "0".to_string(),
                ])?;
                adb_assert_ok(&rotate_off, "adb settings put accelerometer_rotation 0")?;

                let user_rotation = adb_run(&vec![
                    "-s".to_string(), serial.clone(), "shell".to_string(),
                    "settings".to_string(), "put".to_string(), "system".to_string(),
                    "user_rotation".to_string(), "0".to_string(),
                ])?;
                adb_assert_ok(&user_rotation, "adb settings put user_rotation 0")?;
            }
            "landscape" => {
                let rotate_off = adb_run(&vec![
                    "-s".to_string(), serial.clone(), "shell".to_string(),
                    "settings".to_string(), "put".to_string(), "system".to_string(),
                    "accelerometer_rotation".to_string(), "0".to_string(),
                ])?;
                adb_assert_ok(&rotate_off, "adb settings put accelerometer_rotation 0")?;

                let user_rotation = adb_run(&vec![
                    "-s".to_string(), serial.clone(), "shell".to_string(),
                    "settings".to_string(), "put".to_string(), "system".to_string(),
                    "user_rotation".to_string(), "1".to_string(),
                ])?;
                adb_assert_ok(&user_rotation, "adb settings put user_rotation 1")?;
            }
            "unlock" => {
                let rotate_on = adb_run(&vec![
                    "-s".to_string(), serial.clone(), "shell".to_string(),
                    "settings".to_string(), "put".to_string(), "system".to_string(),
                    "accelerometer_rotation".to_string(), "1".to_string(),
                ])?;
                adb_assert_ok(&rotate_on, "adb settings put accelerometer_rotation 1")?;
            }
            _ => {
                return Err("mode must be one of: portrait, landscape, unlock".to_string());
            }
        }

        Ok(json!({
            "ok": true,
            "serial": serial,
            "mode": mode,
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Launch Bonsai Workspace on the selected Android device.
#[tauri::command]
pub async fn android_mobile_launch_bonsai(serial: String) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let serial = serial.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }

        let start = adb_run(&vec![
            "-s".to_string(),
            serial.clone(),
            "shell".to_string(),
            "am".to_string(),
            "start".to_string(),
            "-W".to_string(),
            "-n".to_string(),
            "com.bonsai.workspace/.MainActivity".to_string(),
        ])?;
        adb_assert_ok(&start, "adb am start Bonsai")?;

        Ok(json!({
            "ok": true,
            "serial": serial,
            "activity": "com.bonsai.workspace/.MainActivity",
            "result": start,
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Prepare a uniform mobile runtime session (wake/unlock + reverse ports + launch app).
#[tauri::command]
pub async fn android_mobile_prepare_uniform_runtime(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    remote_manager: State<'_, Arc<RemoteManager>>,
    serial: String,
    api_port: Option<u16>,
    ws_port: Option<u16>,
    start_remote_surface: Option<bool>,
) -> Result<serde_json::Value, String> {
    let remote_surface = start_remote_surface.unwrap_or(true);

    let pair_token = state.pair_token.clone();
    let pair_token_for_launch = pair_token.clone();
    let config = crate::config::load_config(&app_handle).unwrap_or_default();
    let configured_api_host = config.api_host;
    let configured_api_port = config.api_port;

    let remote_session = if remote_surface {
        Some(remote_manager.start_session().await?)
    } else {
        None
    };

    let remote_session_id = remote_session
        .as_ref()
        .map(|s| s.id.clone())
        .unwrap_or_default();

    let out = tokio::task::spawn_blocking(move || {
        let serial = serial.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }

        let api = api_port.unwrap_or(11369);
        let ws = ws_port.unwrap_or(11371);

        let wake = adb_run(&vec![
            "-s".to_string(), serial.clone(), "shell".to_string(),
            "input".to_string(), "keyevent".to_string(), "224".to_string(),
        ])?;
        adb_assert_ok(&wake, "adb keyevent 224")?;

        let unlock = adb_run(&vec![
            "-s".to_string(), serial.clone(), "shell".to_string(),
            "input".to_string(), "keyevent".to_string(), "82".to_string(),
        ])?;
        adb_assert_ok(&unlock, "adb keyevent 82")?;

        let reverse_api = adb_run(&vec![
            "-s".to_string(), serial.clone(), "reverse".to_string(),
            format!("tcp:{api}"), format!("tcp:{api}"),
        ])?;
        adb_assert_ok(&reverse_api, "adb reverse api")?;

        let reverse_ws = adb_run(&vec![
            "-s".to_string(), serial.clone(), "reverse".to_string(),
            format!("tcp:{ws}"), format!("tcp:{ws}"),
        ])?;
        adb_assert_ok(&reverse_ws, "adb reverse ws")?;

        let launch = adb_run(&vec![
            "-s".to_string(), serial.clone(), "shell".to_string(),
            "am".to_string(), "start".to_string(), "-W".to_string(),
            "-n".to_string(), "com.bonsai.workspace/.MainActivity".to_string(),
        ])?;
        adb_assert_ok(&launch, "adb launch bonsai")?;

        let remote_launch = if remote_surface {
            let launch_remote = adb_run(&vec![
                "-s".to_string(), serial.clone(), "shell".to_string(),
                "am".to_string(), "start".to_string(), "-W".to_string(),
                "-n".to_string(), "com.bonsai.workspace/.RemoteSurfaceEntryActivity".to_string(),
                "--es".to_string(), "desktop_host".to_string(), "127.0.0.1".to_string(),
                "--ei".to_string(), "desktop_port".to_string(), api.to_string(),
                "--es".to_string(), "pair_token".to_string(), pair_token_for_launch.clone(),
                "--es".to_string(), "session_id".to_string(), remote_session_id.clone(),
            ])?;
            adb_assert_ok(&launch_remote, "adb launch remote surface activity")?;
            Some(launch_remote)
        } else {
            None
        };

        Ok(json!({
            "ok": true,
            "serial": serial,
            "api_port": api,
            "ws_port": ws,
            "remote_surface_enabled": remote_surface,
            "steps": {
                "wake": wake,
                "unlock": unlock,
                "reverse_api": reverse_api,
                "reverse_ws": reverse_ws,
                "launch": launch,
                "launch_remote_surface": remote_launch,
            }
        }))
    })
    .await
    .map_err(|e| e.to_string())??;

    if !remote_surface {
        return Ok(out);
    }

    let session_id = remote_session
        .as_ref()
        .map(|s| s.id.clone())
        .unwrap_or_default();
    let api_base = format!("http://{}:{}", configured_api_host, configured_api_port);

    Ok(json!({
        "ok": out.get("ok").and_then(|v| v.as_bool()).unwrap_or(false),
        "serial": out.get("serial").cloned().unwrap_or(json!("")),
        "api_port": out.get("api_port").cloned().unwrap_or(json!(configured_api_port)),
        "ws_port": out.get("ws_port").cloned().unwrap_or(json!(configured_api_port)),
        "remote_surface_enabled": true,
        "remote_surface": {
            "session_id": session_id,
            "pair_token": pair_token,
            "desktop_api_base": api_base,
            "frame_url": format!("http://{}:{}/remote/surface/frame", configured_api_host, configured_api_port),
            "input_url": format!("http://{}:{}/remote/surface/input", configured_api_host, configured_api_port),
            "stop_url": format!("http://{}:{}/remote/surface/session/stop", configured_api_host, configured_api_port)
        },
        "steps": out.get("steps").cloned().unwrap_or(json!({}))
    }))
}

/// List Android devices visible through ADB (USB or WiFi debugging).
#[tauri::command]
pub async fn android_usb_list_devices() -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let args = vec!["devices".to_string(), "-l".to_string()];
        let result = adb_run(&args)?;
        adb_assert_ok(&result, "adb devices")?;

        let stdout = result
            .get("stdout")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let mut devices = Vec::new();
        for line in stdout.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }

            let serial = parts[0].to_string();
            let state = parts[1].to_string();
            let mut model = String::new();
            let mut device = String::new();
            let mut transport_id = String::new();

            for part in parts.iter().skip(2) {
                if let Some(rest) = part.strip_prefix("model:") {
                    model = rest.to_string();
                } else if let Some(rest) = part.strip_prefix("device:") {
                    device = rest.to_string();
                } else if let Some(rest) = part.strip_prefix("transport_id:") {
                    transport_id = rest.to_string();
                }
            }

            devices.push(serde_json::json!({
                "serial": serial,
                "state": state,
                "model": model,
                "device": device,
                "transport_id": transport_id,
                "raw": line,
            }));
        }

        Ok(serde_json::json!({
            "devices": devices,
            "raw": stdout,
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Report which adb executable Bonsai is currently resolving.
#[tauri::command]
pub async fn android_usb_get_adb_info() -> Result<serde_json::Value, String> {
    let (adb_executable, candidates) = resolve_adb_executable();
    Ok(serde_json::json!({
        "adb_executable": adb_executable,
        "candidates": candidates,
    }))
}

/// Run an arbitrary ADB shell command on a selected Android device.
#[tauri::command]
pub async fn android_usb_shell(serial: String, shell_command: String) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let serial = serial.trim().to_string();
        let shell_command = shell_command.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }
        if shell_command.is_empty() {
            return Err("shell_command cannot be empty".to_string());
        }

        let args = vec![
            "-s".to_string(),
            serial,
            "shell".to_string(),
            shell_command,
        ];
        let result = adb_run(&args)?;
        adb_assert_ok(&result, "adb shell")?;
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Install an APK onto a selected Android device.
#[tauri::command]
pub async fn android_usb_install_apk(
    serial: String,
    apk_path: String,
    replace: Option<bool>,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let serial = serial.trim().to_string();
        let apk_path = apk_path.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }
        if apk_path.is_empty() {
            return Err("apk_path cannot be empty".to_string());
        }
        if !std::path::Path::new(&apk_path).exists() {
            return Err(format!("APK not found: {apk_path}"));
        }

        let mut args = vec!["-s".to_string(), serial, "install".to_string()];
        if replace.unwrap_or(true) {
            args.push("-r".to_string());
        }
        args.push(apk_path);

        let result = adb_run(&args)?;
        adb_assert_ok(&result, "adb install")?;
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Launch an Android app by package name (and optional activity).
#[tauri::command]
pub async fn android_usb_launch_app(
    serial: String,
    package_name: String,
    activity: Option<String>,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let serial = serial.trim().to_string();
        let package_name = package_name.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }
        if package_name.is_empty() {
            return Err("package_name cannot be empty".to_string());
        }

        let args = if let Some(activity_name) = activity {
            let activity_name = activity_name.trim().to_string();
            if activity_name.is_empty() {
                vec![
                    "-s".to_string(),
                    serial,
                    "shell".to_string(),
                    "monkey".to_string(),
                    "-p".to_string(),
                    package_name,
                    "-c".to_string(),
                    "android.intent.category.LAUNCHER".to_string(),
                    "1".to_string(),
                ]
            } else {
                vec![
                    "-s".to_string(),
                    serial,
                    "shell".to_string(),
                    "am".to_string(),
                    "start".to_string(),
                    "-n".to_string(),
                    format!("{package_name}/{activity_name}"),
                ]
            }
        } else {
            vec![
                "-s".to_string(),
                serial,
                "shell".to_string(),
                "monkey".to_string(),
                "-p".to_string(),
                package_name,
                "-c".to_string(),
                "android.intent.category.LAUNCHER".to_string(),
                "1".to_string(),
            ]
        };

        let result = adb_run(&args)?;
        adb_assert_ok(&result, "adb launch app")?;
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Configure adb reverse for desktop API access over USB.
#[tauri::command]
pub async fn android_usb_reverse(
    serial: String,
    host_port: Option<u16>,
    device_port: Option<u16>,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let serial = serial.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }

        let host_port = host_port.unwrap_or(crate::config::DEFAULT_API_PORT);
        let device_port = device_port.unwrap_or(crate::config::DEFAULT_API_PORT);
        let args = vec![
            "-s".to_string(),
            serial,
            "reverse".to_string(),
            format!("tcp:{host_port}"),
            format!("tcp:{device_port}"),
        ];

        let result = adb_run(&args)?;
        adb_assert_ok(&result, "adb reverse")?;
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Remove all adb reverse mappings for a selected device.
#[tauri::command]
pub async fn android_usb_reverse_clear(serial: String) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let serial = serial.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }

        let args = vec![
            "-s".to_string(),
            serial,
            "reverse".to_string(),
            "--remove-all".to_string(),
        ];
        let result = adb_run(&args)?;
        adb_assert_ok(&result, "adb reverse --remove-all")?;
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Enable WiFi debugging mode (adb tcpip) while connected over USB.
#[tauri::command]
pub async fn android_usb_enable_wifi_debug(
    serial: String,
    port: Option<u16>,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let serial = serial.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }

        let port = port.unwrap_or(5555);
        let args = vec![
            "-s".to_string(),
            serial,
            "tcpip".to_string(),
            port.to_string(),
        ];
        let result = adb_run(&args)?;
        adb_assert_ok(&result, "adb tcpip")?;
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Connect to an Android device over WiFi debugging.
#[tauri::command]
pub async fn android_usb_connect_wifi(
    host: String,
    port: Option<u16>,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let host = host.trim().to_string();
        if host.is_empty() {
            return Err("host cannot be empty".to_string());
        }

        let target = if host.contains(':') {
            host
        } else {
            format!("{}:{}", host, port.unwrap_or(5555))
        };
        let args = vec!["connect".to_string(), target];
        let result = adb_run(&args)?;
        adb_assert_ok(&result, "adb connect")?;
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Disconnect one (or all) WiFi adb sessions.
#[tauri::command]
pub async fn android_usb_disconnect_wifi(
    host: Option<String>,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let mut args = vec!["disconnect".to_string()];
        if let Some(host) = host {
            let host = host.trim().to_string();
            if !host.is_empty() {
                args.push(host);
            }
        }
        let result = adb_run(&args)?;
        adb_assert_ok(&result, "adb disconnect")?;
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn usb_regression_evidence_path(app_handle: &AppHandle) -> Result<std::path::PathBuf, String> {
    use tauri::Manager;
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
    Ok(app_data_dir.join("android-usb-regression-evidence.jsonl"))
}

/// Run an end-to-end Android USB regression workflow and persist an evidence record.
#[tauri::command]
pub async fn android_usb_run_regression(
    app_handle: AppHandle,
    serial: String,
    api_port: Option<u16>,
    package_name: Option<String>,
    activity: Option<String>,
    wifi_host: Option<String>,
    wifi_port: Option<u16>,
    strict_require_app: Option<bool>,
    apk_path: Option<String>,
    enable_bootstrap: Option<bool>,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        use std::time::{SystemTime, UNIX_EPOCH};

        let serial = serial.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }

        let api_port = api_port.unwrap_or(crate::config::DEFAULT_API_PORT);
        let wifi_port = wifi_port.unwrap_or(5555);
        let pkg = package_name.unwrap_or_else(|| "com.bonsai.workspace".to_string());
        let activity_name = activity.unwrap_or_default();
        let wifi_host = wifi_host.unwrap_or_default().trim().to_string();
        let strict = strict_require_app.unwrap_or(false);
        let resolved_apk = apk_path.map(|s| s.trim().to_string()).unwrap_or_default();
        let _enable_bootstrap = enable_bootstrap.unwrap_or(false);

        let mut steps = Vec::<serde_json::Value>::new();

        let run_step = |label: &str, args: Vec<String>, steps: &mut Vec<serde_json::Value>| -> Result<serde_json::Value, String> {
            let out = adb_run(&args)?;
            let ok = out.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            steps.push(serde_json::json!({
                "label": label,
                "ok": ok,
                "result": out,
            }));
            Ok(steps.last().cloned().unwrap_or_else(|| serde_json::json!({})))
        };

        let mut all_ok = true;

        // 1) Verify device appears.
        let devices = run_step(
            "adb devices -l",
            vec!["devices".to_string(), "-l".to_string()],
            &mut steps,
        )?;
        if !devices.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            all_ok = false;
        }

        // 2) Query model.
        let model = run_step(
            "adb shell getprop ro.product.model",
            vec![
                "-s".to_string(),
                serial.clone(),
                "shell".to_string(),
                "getprop ro.product.model".to_string(),
            ],
            &mut steps,
        )?;
        if !model.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            all_ok = false;
        }

        // 3) Configure reverse mapping.
        let reverse = run_step(
            "adb reverse tcp:api tcp:api",
            vec![
                "-s".to_string(),
                serial.clone(),
                "reverse".to_string(),
                format!("tcp:{api_port}"),
                format!("tcp:{api_port}"),
            ],
            &mut steps,
        )?;
        if !reverse.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            all_ok = false;
        }

        let reverse_list = run_step(
            "adb reverse --list",
            vec![
                "-s".to_string(),
                serial.clone(),
                "reverse".to_string(),
                "--list".to_string(),
            ],
            &mut steps,
        )?;
        if !reverse_list.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            all_ok = false;
        }

        // 4) Launch app (optional but enabled by default package).
        let launch_args = if activity_name.trim().is_empty() {
            vec![
                "-s".to_string(),
                serial.clone(),
                "shell".to_string(),
                "monkey".to_string(),
                "-p".to_string(),
                pkg.clone(),
                "-c".to_string(),
                "android.intent.category.LAUNCHER".to_string(),
                "1".to_string(),
            ]
        } else {
            vec![
                "-s".to_string(),
                serial.clone(),
                "shell".to_string(),
                "am".to_string(),
                "start".to_string(),
                "-n".to_string(),
                format!("{}/{}", pkg, activity_name.trim()),
            ]
        };
        let launch = run_step("launch app", launch_args, &mut steps)?;
        if !launch.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            all_ok = false;
        }

        // 5) Optional USB -> WiFi bridge validation.
        if !wifi_host.is_empty() {
            let tcpip = run_step(
                "adb tcpip",
                vec![
                    "-s".to_string(),
                    serial.clone(),
                    "tcpip".to_string(),
                    wifi_port.to_string(),
                ],
                &mut steps,
            )?;
            if !tcpip.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                all_ok = false;
            }

            let connect = run_step(
                "adb connect",
                vec!["connect".to_string(), format!("{}:{}", wifi_host, wifi_port)],
                &mut steps,
            )?;
            if !connect.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                all_ok = false;
            }
        }

        let ts_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis() as u64;

        let record = serde_json::json!({
            "schema_version": 2,
            "ts_ms": ts_ms,
            "serial": serial,
            "api_port": api_port,
            "wifi_host": if wifi_host.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(wifi_host.clone()) },
            "wifi_port": wifi_port,
            "package_name": pkg,
            "activity": if activity_name.trim().is_empty() { serde_json::Value::Null } else { serde_json::Value::String(activity_name.trim().to_string()) },
            "strict_require_app": strict,
            "resolved_apk_path": if resolved_apk.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(resolved_apk.clone()) },
            "ok": all_ok,
            "steps": steps,
            "platform": std::env::consts::OS,
        });

        let path = usb_regression_evidence_path(&app_handle)?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        writeln!(file, "{}", record).map_err(|e| e.to_string())?;

        Ok(serde_json::json!({
            "path": path.to_string_lossy(),
            "record": record,
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

// ─── USB Lab Runtime System ───────────────────────────────────────────────────
// Steps 1-4: Readiness, APK resolver, install/launch orchestrator, bootstrap.

/// Step 1: Device Readiness State Machine.
///
/// Returns a structured readiness status for a single device so the UI can
/// guide the operator with a deterministic next-action.
#[tauri::command]
pub async fn android_usb_get_device_readiness(
    serial: String,
    api_port: Option<u16>,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        use std::time::Instant;

        let serial = serial.trim().to_string();
        let api_port = api_port.unwrap_or(crate::config::DEFAULT_API_PORT);
        let (adb_executable, _candidates) = resolve_adb_executable();
        let adb_present = std::path::Path::new(&adb_executable).exists()
            || which_adb_on_path();

        // Helper: run a quick adb command, return (ok, stdout, stderr).
        let quick = |args: Vec<String>| -> (bool, String, String) {
            match adb_run(&args) {
                Ok(v) => (
                    v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false),
                    v.get("stdout").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                    v.get("stderr").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                ),
                Err(_) => (false, String::new(), String::new()),
            }
        };

        // --- adb present? ---
        if !adb_present {
            return Ok(serde_json::json!({
                "serial": serial,
                "adb_executable": adb_executable,
                "connected": false,
                "authorized": false,
                "model": null,
                "reverse_api_active": false,
                "api_port": api_port,
                "status": "disconnected",
                "next_action": "Install Android Platform Tools (adb) and ensure it is on PATH or ANDROID_HOME is set.",
            }));
        }

        // --- Is device visible? ---
        let (devices_ok, devices_out, _) =
            quick(vec!["devices".to_string(), "-l".to_string()]);

        let mut connected = false;
        let mut authorized = false;
        let mut state_str = String::new();

        if devices_ok {
            for line in devices_out.lines().skip(1) {
                let line = line.trim();
                if line.is_empty() { continue; }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 2 { continue; }
                if parts[0] == serial {
                    connected = true;
                    state_str = parts[1].to_string();
                    authorized = state_str == "device";
                    break;
                }
            }
            // If serial is empty pick first "device" state entry.
            if serial.is_empty() {
                for line in devices_out.lines().skip(1) {
                    let line = line.trim();
                    if line.is_empty() { continue; }
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 && parts[1] == "device" {
                        connected = true;
                        authorized = true;
                        state_str = "device".to_string();
                        break;
                    }
                }
            }
        }

        if !connected {
            return Ok(serde_json::json!({
                "serial": serial,
                "adb_executable": adb_executable,
                "connected": false,
                "authorized": false,
                "model": null,
                "reverse_api_active": false,
                "api_port": api_port,
                "status": "disconnected",
                "next_action": "Connect Android device over USB and enable USB debugging.",
            }));
        }

        if !authorized {
            return Ok(serde_json::json!({
                "serial": serial,
                "adb_executable": adb_executable,
                "connected": true,
                "authorized": false,
                "model": null,
                "reverse_api_active": false,
                "api_port": api_port,
                "status": "unauthorized",
                "next_action": format!("Tap 'Allow USB debugging' on the device screen (state: {state_str})."),
            }));
        }

        // --- Model ---
        let _t0 = Instant::now();
        let (model_ok, model_out, _) = quick(vec![
            "-s".to_string(), serial.clone(),
            "shell".to_string(), "getprop".to_string(), "ro.product.model".to_string(),
        ]);
        let model_str = if model_ok && !model_out.is_empty() {
            model_out.trim().to_string()
        } else {
            String::new()
        };

        // --- Reverse mapping active? ---
        let (rev_ok, rev_out, _) = quick(vec![
            "-s".to_string(), serial.clone(),
            "reverse".to_string(), "--list".to_string(),
        ]);
        let needle = format!("tcp:{api_port}");
        let reverse_api_active = rev_ok && rev_out.contains(&needle);

        let status = if reverse_api_active { "ready" } else { "online" };
        let next_action = if reverse_api_active {
            "Device is ready. Run Install & Launch or Full Validation.".to_string()
        } else {
            "Click 'Bootstrap Connection' to establish reverse port mapping.".to_string()
        };

        Ok(serde_json::json!({
            "serial": serial,
            "adb_executable": adb_executable,
            "connected": true,
            "authorized": true,
            "model": if model_str.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(model_str) },
            "reverse_api_active": reverse_api_active,
            "api_port": api_port,
            "status": status,
            "next_action": next_action,
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

fn which_adb_on_path() -> bool {
    Command::new("adb")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Step 2a: APK Artifact Resolver.
///
/// Resolution order:
/// 1. explicit_path if provided and exists
/// 2. Known Tauri/Gradle output candidates under the workspace
/// 3. Error with candidate list so the UI can guide the operator.
#[tauri::command]
pub async fn android_usb_resolve_apk(
    app_handle: AppHandle,
    explicit_path: Option<String>,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        // 1. Explicit path wins.
        if let Some(ref p) = explicit_path {
            let p = p.trim();
            if !p.is_empty() {
                let path = std::path::Path::new(p);
                if path.exists() {
                    return apk_metadata(p);
                }
                return Err(format!("Explicit APK path not found: {p}"));
            }
        }

        // 2. Known output candidates relative to the app data directory.
        use tauri::Manager as _;
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?;

        // Walk up to workspace root (app_data_dir is typically inside AppData; walk up to find bonsai src-tauri).
        let workspace_root: std::path::PathBuf = app_data_dir
            .ancestors()
            .find(|p: &&std::path::Path| p.join("src-tauri").exists())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| app_data_dir.clone());

        let candidates: Vec<std::path::PathBuf> = vec![
            // Tauri mobile build output.
            workspace_root.join("src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk"),
            workspace_root.join("src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk"),
            workspace_root.join("src-tauri/gen/android/app/build/outputs/apk/arm64-v8a/release/app-arm64-v8a-release-unsigned.apk"),
            workspace_root.join("src-tauri/gen/android/app/build/outputs/apk/arm64-v8a/debug/app-arm64-v8a-debug.apk"),
            // Generic gradle outputs.
            workspace_root.join("app/build/outputs/apk/release/app-release.apk"),
            workspace_root.join("app/build/outputs/apk/debug/app-debug.apk"),
        ];

        let candidate_strs: Vec<String> = candidates
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        for candidate in &candidates {
            if candidate.exists() {
                let s = candidate.to_string_lossy().to_string();
                return apk_metadata(&s);
            }
        }

        Err(format!(
            "No APK found. Checked candidates: {}. Provide ANDROID_APK_PATH or build the project first.",
            candidate_strs.join("; ")
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Extract metadata from an APK file (package name, version, etc.) using aapt if available.
fn apk_metadata(path: &str) -> Result<serde_json::Value, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("APK stat error: {e}"))?;
    let size_bytes = meta.len();

    // Try aapt2 then aapt for richer metadata.
    let mut package = String::new();
    let mut version_name = String::new();
    let mut version_code = String::new();

    for tool in &["aapt2", "aapt"] {
        let result = Command::new(tool)
            .args(["dump", "badging", path])
            .output();
        if let Ok(out) = result {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines() {
                    if line.starts_with("package:") {
                        for part in line.split_whitespace() {
                            if let Some(v) = part.strip_prefix("name='") {
                                package = v.trim_end_matches('\'').to_string();
                            } else if let Some(v) = part.strip_prefix("versionName='") {
                                version_name = v.trim_end_matches('\'').to_string();
                            } else if let Some(v) = part.strip_prefix("versionCode='") {
                                version_code = v.trim_end_matches('\'').to_string();
                            }
                        }
                        break;
                    }
                }
                break;
            }
        }
    }

    Ok(serde_json::json!({
        "path": path,
        "size_bytes": size_bytes,
        "package": if package.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(package) },
        "version_name": if version_name.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(version_name) },
        "version_code": if version_code.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(version_code) },
    }))
}

/// Step 2b: Install and Launch Orchestrator.
///
/// Single command that installs an APK, verifies it, and launches the app.
/// Returns a per-step result array so the UI can show exactly where it failed.
#[tauri::command]
pub async fn android_usb_install_and_launch(
    serial: String,
    apk_path: String,
    package_name: Option<String>,
    activity: Option<String>,
    strict_require_app: Option<bool>,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        use std::time::Instant;

        let serial = serial.trim().to_string();
        let apk_path = apk_path.trim().to_string();
        let pkg = package_name
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "com.bonsai.workspace".to_string());
        let activity_name = activity.map(|s| s.trim().to_string()).unwrap_or_default();
        let strict = strict_require_app.unwrap_or(false);

        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }
        if apk_path.is_empty() {
            return Err("apk_path cannot be empty".to_string());
        }
        if !std::path::Path::new(&apk_path).exists() {
            return Err(format!("APK not found: {apk_path}"));
        }

        let mut steps: Vec<serde_json::Value> = Vec::new();
        let mut overall_ok = true;

        macro_rules! run_step {
            ($label:expr, $args:expr) => {{
                let t0 = Instant::now();
                let result = adb_run(&$args);
                let duration_ms = t0.elapsed().as_millis() as u64;
                match result {
                    Ok(v) => {
                        let ok = v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false);
                        let stdout = v.get("stdout").and_then(|x| x.as_str()).unwrap_or("").to_string();
                        let stderr = v.get("stderr").and_then(|x| x.as_str()).unwrap_or("").to_string();
                        let step = serde_json::json!({
                            "label": $label,
                            "ok": ok,
                            "stdout": stdout,
                            "stderr": stderr,
                            "duration_ms": duration_ms,
                            "fatal": false,
                        });
                        steps.push(step.clone());
                        if !ok { overall_ok = false; }
                        step
                    }
                    Err(e) => {
                        let step = serde_json::json!({
                            "label": $label,
                            "ok": false,
                            "stdout": "",
                            "stderr": e.clone(),
                            "duration_ms": duration_ms,
                            "fatal": true,
                        });
                        steps.push(step.clone());
                        overall_ok = false;
                        step
                    }
                }
            }};
        }

        // 1. Install APK.
        run_step!("adb install -r", vec![
            "-s".to_string(), serial.clone(),
            "install".to_string(), "-r".to_string(),
            apk_path.clone(),
        ]);

        // 2. Verify package installed.
        let pkg_check = run_step!("pm path", vec![
            "-s".to_string(), serial.clone(),
            "shell".to_string(), "pm".to_string(), "path".to_string(), pkg.clone(),
        ]);
        let pkg_installed = pkg_check.get("stdout")
            .and_then(|v| v.as_str())
            .map(|s| s.contains("package:"))
            .unwrap_or(false);

        if !pkg_installed {
            let hint = if strict {
                "Package not found after install (strict mode); check APK compatibility."
            } else {
                "Package not found after install; continuing in non-strict mode."
            };
            steps.last_mut().map(|s| {
                s.as_object_mut().map(|o| {
                    o.insert("hint".to_string(), serde_json::Value::String(hint.to_string()));
                    if strict { o.insert("ok".to_string(), serde_json::Value::Bool(false)); }
                });
            });
            if strict { overall_ok = false; }
        }

        // 3. Launch app.
        if pkg_installed || !strict {
            let launch_args = if !activity_name.is_empty() {
                vec![
                    "-s".to_string(), serial.clone(),
                    "shell".to_string(), "am".to_string(), "start".to_string(),
                    "-n".to_string(), format!("{pkg}/{activity_name}"),
                ]
            } else {
                vec![
                    "-s".to_string(), serial.clone(),
                    "shell".to_string(), "monkey".to_string(),
                    "-p".to_string(), pkg.clone(),
                    "-c".to_string(), "android.intent.category.LAUNCHER".to_string(),
                    "1".to_string(),
                ]
            };
            run_step!("launch app", launch_args);

            // 4. Verify process started.
            let ps_step = run_step!("pidof app process", vec![
                "-s".to_string(), serial.clone(),
                "shell".to_string(), "pidof".to_string(), pkg.clone(),
            ]);
            let pid_ok = ps_step.get("stdout")
                .and_then(|v| v.as_str())
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            if !pid_ok && strict {
                overall_ok = false;
            }
        } else {
            steps.push(serde_json::json!({
                "label": "launch app",
                "ok": true,
                "stdout": "",
                "stderr": "",
                "duration_ms": 0,
                "fatal": false,
                "skipped": true,
                "reason": format!("Package {pkg} not installed; strict_require_app=false so skipping launch."),
            }));
        }

        Ok(serde_json::json!({
            "serial": serial,
            "apk_path": apk_path,
            "package_name": pkg,
            "strict_require_app": strict,
            "ok": overall_ok,
            "steps": steps,
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Step 3: Connection Bootstrap Command.
///
/// Ensures reverse port mapping is active and optionally enables WiFi bridge.
/// Returns a deterministic step list with no hidden side effects.
#[tauri::command]
pub async fn android_usb_bootstrap_connection(
    serial: String,
    api_port: Option<u16>,
    ws_port: Option<u16>,
    wifi_host: Option<String>,
    wifi_port: Option<u16>,
    enable_wifi_bridge: Option<bool>,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        use std::time::Instant;

        let serial = serial.trim().to_string();
        if serial.is_empty() {
            return Err("serial cannot be empty".to_string());
        }

        let api_port = api_port.unwrap_or(crate::config::DEFAULT_API_PORT);
        let ws_port = ws_port.unwrap_or(api_port); // WS shares the API port path (/ws route)
        let wifi_port_val = wifi_port.unwrap_or(5555);
        let wifi_host_val = wifi_host.unwrap_or_default().trim().to_string();
        let bridge = enable_wifi_bridge.unwrap_or(false);

        let mut steps: Vec<serde_json::Value> = Vec::new();
        let mut overall_ok = true;

        macro_rules! step {
            ($label:expr, $args:expr) => {{
                let t0 = Instant::now();
                let r = adb_run(&$args);
                let duration_ms = t0.elapsed().as_millis() as u64;
                let (ok, stdout, stderr) = match r {
                    Ok(v) => (
                        v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false),
                        v.get("stdout").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                        v.get("stderr").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                    ),
                    Err(e) => (false, String::new(), e),
                };
                if !ok { overall_ok = false; }
                steps.push(serde_json::json!({
                    "label": $label,
                    "ok": ok,
                    "stdout": stdout,
                    "stderr": stderr,
                    "duration_ms": duration_ms,
                }));
                ok
            }};
        }

        // 1. Reverse API port.
        step!("adb reverse tcp:api", vec![
            "-s".to_string(), serial.clone(),
            "reverse".to_string(),
            format!("tcp:{api_port}"),
            format!("tcp:{api_port}"),
        ]);

        // 2. If ws_port differs from api_port also map it.
        if ws_port != api_port {
            step!("adb reverse tcp:ws", vec![
                "-s".to_string(), serial.clone(),
                "reverse".to_string(),
                format!("tcp:{ws_port}"),
                format!("tcp:{ws_port}"),
            ]);
        }

        // 3. Verify reverse list contains both mappings.
        let rev_ok = step!("adb reverse --list", vec![
            "-s".to_string(), serial.clone(),
            "reverse".to_string(), "--list".to_string(),
        ]);

        // Check stdout of the --list step for expected mapping.
        if let Some(list_step) = steps.last() {
            let stdout = list_step.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
            let needle = format!("tcp:{api_port}");
            if rev_ok && !stdout.contains(&needle) {
                overall_ok = false;
                // Amend the step to reflect the mapping check failure.
                let idx = steps.len() - 1;
                if let Some(s) = steps.get_mut(idx) {
                    if let Some(o) = s.as_object_mut() {
                        o.insert("ok".to_string(), serde_json::Value::Bool(false));
                        o.insert("hint".to_string(), serde_json::Value::String(
                            format!("Reverse --list did not contain tcp:{api_port}")
                        ));
                    }
                }
            }
        }

        // 4. Optional WiFi bridge.
        if bridge && !wifi_host_val.is_empty() {
            step!("adb tcpip", vec![
                "-s".to_string(), serial.clone(),
                "tcpip".to_string(), wifi_port_val.to_string(),
            ]);
            step!("adb connect wifi", vec![
                "connect".to_string(),
                format!("{}:{}", wifi_host_val, wifi_port_val),
            ]);
        }

        Ok(serde_json::json!({
            "serial": serial,
            "api_port": api_port,
            "ws_port": ws_port,
            "wifi_bridge_enabled": bridge,
            "ok": overall_ok,
            "steps": steps,
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Compatibility command retained for legacy invoke surfaces.
///
/// Mobile QR scanning is handled in the frontend via
/// `@tauri-apps/plugin-barcode-scanner` (see SettingsPanel scan action).
/// Desktop builds return a clear guidance error.
#[tauri::command]
pub async fn scan_qr() -> Result<String, String> {
    Err("scan_qr is only available on Android builds (barcode scanner plugin)".to_string())
}

/// Persist desktop connection details for Android auto-reconnect.
#[tauri::command]
pub async fn save_desktop_connection(
    app_handle: AppHandle,
    ip: String,
    token: String,
) -> Result<serde_json::Value, String> {
    let ip = ip.trim().to_string();
    let token = token.trim().to_string();

    if ip.is_empty() {
        return Err("ip cannot be empty".to_string());
    }
    if token.is_empty() {
        return Err("token cannot be empty".to_string());
    }

    let mut config = crate::config::load_config(&app_handle)?;
    config.desktop_connection_ip = Some(ip.clone());
    let config = crate::config::save_config(&app_handle, &config)?;

    // Store the token in the OS keychain instead of the config file
    let secrets = crate::secrets_store::SecretsStore::new();
    secrets.store(crate::secrets_store::ACCOUNT_DESKTOP_CONNECTION_TOKEN, &token)?;

    Ok(serde_json::json!({
        "ip": config.desktop_connection_ip,
        "token": token,
    }))
}

/// Load persisted desktop connection details for Android auto-reconnect.
#[tauri::command]
pub async fn load_desktop_connection(app_handle: AppHandle) -> Result<Option<serde_json::Value>, String> {
    let config = crate::config::load_config(&app_handle)?;
    
    // Retrieve the token from the OS keychain
    let secrets = crate::secrets_store::SecretsStore::new();
    let token = secrets.get(crate::secrets_store::ACCOUNT_DESKTOP_CONNECTION_TOKEN)?
        .and_then(|t| if t.is_empty() { None } else { Some(t) });
    
    match (config.desktop_connection_ip, token) {
        (Some(ip), Some(token)) => Ok(Some(serde_json::json!({
            "ip": ip,
            "token": token,
        }))),
        _ => Ok(None),
    }
}

fn mobile_pairing_evidence_path(app_handle: &AppHandle) -> Result<std::path::PathBuf, String> {
    use tauri::Manager;
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
    Ok(app_data_dir.join("mobile-pairing-evidence.jsonl"))
}

/// Append a mobile pairing verification evidence record to app data storage.
#[tauri::command]
pub async fn record_mobile_pairing_evidence(
    app_handle: AppHandle,
    source: String,
    ip: String,
    verified: bool,
    detail: String,
    ws_url: Option<String>,
    elapsed_ms: Option<u64>,
    scanned_payload: Option<String>,
    token_hint: Option<String>,
) -> Result<serde_json::Value, String> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let path = mobile_pairing_evidence_path(&app_handle)?;
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as u64;

    let record = serde_json::json!({
        "ts_ms": now_ms,
        "source": source,
        "ip": ip,
        "verified": verified,
        "detail": detail,
        "ws_url": ws_url,
        "elapsed_ms": elapsed_ms,
        "scanned_payload": scanned_payload,
        "token_hint": token_hint,
        "platform": std::env::consts::OS,
    });

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    writeln!(file, "{}", record).map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "path": path.to_string_lossy(),
        "record": record,
    }))
}

/// Read recent mobile pairing verification evidence records.
#[tauri::command]
pub async fn get_mobile_pairing_evidence(
    app_handle: AppHandle,
    limit: Option<usize>,
) -> Result<serde_json::Value, String> {
    let path = mobile_pairing_evidence_path(&app_handle)?;
    if !path.exists() {
        return Ok(serde_json::json!({
            "path": path.to_string_lossy(),
            "items": [],
        }));
    }

    let file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let take = limit.unwrap_or(20).max(1).min(200);
    let mut lines = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        lines.push(line);
    }

    let start = lines.len().saturating_sub(take);
    let mut items = Vec::new();
    for raw in lines.into_iter().skip(start) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
            items.push(value);
        }
    }

    Ok(serde_json::json!({
        "path": path.to_string_lossy(),
        "items": items,
    }))
}

/// Browse LAN for `_bonsai._tcp.local.` services.
#[tauri::command]
pub async fn browse_bonsai_services() -> Result<Vec<serde_json::Value>, String> {
    use mdns_sd::{ServiceDaemon, ServiceEvent};
    use std::time::{Duration, Instant};

    let mdns = ServiceDaemon::new().map_err(|e| e.to_string())?;
    let receiver = mdns
        .browse("_bonsai._tcp.local.")
        .map_err(|e| e.to_string())?;

    tokio::task::spawn_blocking(move || {
        let deadline = Instant::now() + Duration::from_millis(1500);
        let mut out = Vec::new();

        while Instant::now() < deadline {
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    out.push(serde_json::json!({
                        "name": info.get_fullname(),
                        "host": info.get_hostname(),
                        "ip": info.get_addresses().iter().next().map(|x| x.to_string()).unwrap_or_default(),
                        "port": info.get_port(),
                    }));
                }
                Ok(_) => {}
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }

        out
    })
    .await
    .map_err(|e| e.to_string())
}

// ─── Multi-agent swarm commands ──────────────────────────────────────────────

use crate::agent_store::{AgentConfig, Persona, ResolvedAgent};
use crate::swarm_orchestrator::{SwarmRequest, SwarmResult, SwarmRuntimeSettings};

#[derive(serde::Serialize)]
pub struct AgentResourceCost {
    pub agent_id:        String,
    pub slot_index:      i64,
    pub model_id:        Option<String>,
    pub ram_required_mb: u64,
}

#[derive(serde::Serialize)]
pub struct SwarmResourceEstimate {
    pub total_ram_required_mb: u64,
    pub shared_ram_required_mb: u64,
    pub free_ram_mb:           u64,
    pub fits:                  bool,
    pub per_agent:             Vec<AgentResourceCost>,
}

#[tauri::command]
pub async fn list_personas(state: State<'_, AppState>) -> Result<Vec<Persona>, String> {
    state.agent_store.list_personas().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upsert_persona(state: State<'_, AppState>, persona: Persona) -> Result<Persona, String> {
    state.agent_store.upsert_persona(persona).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_persona(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.agent_store.delete_persona(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_agent_configs(state: State<'_, AppState>) -> Result<Vec<ResolvedAgent>, String> {
    state.agent_store.resolve_agents(&state.orchestrator).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upsert_agent_config(state: State<'_, AppState>, config: AgentConfig) -> Result<AgentConfig, String> {
    state.agent_store.upsert_agent(config).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_agent_config(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.agent_store.delete_agent(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn estimate_swarm_resources(state: State<'_, AppState>) -> Result<SwarmResourceEstimate, String> {
    if !crate::features::FeatureFlags::is_enabled("swarm") {
        return Err("Swarm feature is disabled".into());
    }
    let resolved = state.agent_store.resolve_agents(&state.orchestrator).await.map_err(|e| e.to_string())?;
    let enabled: Vec<&ResolvedAgent> = resolved.iter().filter(|a| a.config.enabled).collect();

    // Shared-model baseline (same model ID counted once).
    let mut seen_models = std::collections::HashSet::new();
    let mut shared_ram: u64 = 0;
    for a in &enabled {
        if let Some(mid) = &a.effective_model_id {
            if seen_models.insert(mid.clone()) {
                shared_ram += a.ram_required_mb;
            }
        }
    }

    // Per-agent configured RAM is what the UI presents as "RAM estimate".
    // This avoids confusing drops when an agent switches to a larger model
    // that is already used by another agent.
    let per_agent_ram: u64 = enabled.iter().map(|a| a.ram_required_mb).sum();
    let kv_overhead: u64 = 256 * enabled.len() as u64;
    let total = per_agent_ram + kv_overhead;

    let mut sys = sysinfo::System::new_all();
    sys.refresh_memory();
    let free_ram_mb = sys.available_memory() / 1024 / 1024;
    let fits = total <= (free_ram_mb as f64 * 0.85) as u64;

    let per_agent = enabled.iter().map(|a| AgentResourceCost {
        agent_id:        a.config.id.clone(),
        slot_index:      a.config.slot_index,
        model_id:        a.effective_model_id.clone(),
        ram_required_mb: a.ram_required_mb,
    }).collect();

    Ok(SwarmResourceEstimate {
        total_ram_required_mb: total,
        shared_ram_required_mb: shared_ram + kv_overhead,
        free_ram_mb,
        fits,
        per_agent,
    })
}

#[derive(serde::Serialize)]
pub struct SwarmChatResponse {
    pub run_id:         String,
    pub final_content:  String,
    pub leader_plan:    Option<serde_json::Value>,
    pub agent_results:  Vec<crate::swarm_orchestrator::AgentOutput>,
    pub stats:          InferStats,
    pub action_handled: bool,
    pub tools_used:     Vec<String>,
}

#[tauri::command]
pub async fn submit_swarm_chat(
    app_handle:     AppHandle,
    state:          State<'_, AppState>,
    messages:       Vec<ChatMessagePayload>,
    workspace_path: Option<String>,
    enabled_tools:  Option<Vec<String>>,
    swarm_settings: Option<SwarmRuntimeSettings>,
) -> Result<SwarmChatResponse, BonsaiError> {
    if !crate::features::FeatureFlags::is_enabled("swarm") {
        return Err(BonsaiError::Config("Swarm feature is disabled".into()));
    }
    let resolved = state.agent_store.resolve_agents(&state.orchestrator).await.map_err(|e| BonsaiError::Orchestrator(e.to_string()))?;
    let leader = resolved
        .iter()
        .find(|a| a.config.slot_index == 0)
        .cloned()
        .ok_or_else(|| "Leader agent (slot 0) is missing".to_string())?;

    let mut enabled_workers: Vec<ResolvedAgent> = resolved
        .into_iter()
        .filter(|a| a.config.slot_index != 0 && a.config.enabled)
        .collect();
    enabled_workers.sort_by_key(|a| a.config.slot_index);

    let mut enabled: Vec<ResolvedAgent> = Vec::with_capacity(1 + enabled_workers.len());
    enabled.push(leader);
    enabled.extend(enabled_workers);

    if enabled.len() <= 1 {
        // Single-agent fallback: just call submit_chat logic
        state.chat_cancel.store(false, Ordering::Relaxed);
        let result = submit_chat(app_handle, state, messages, workspace_path, enabled_tools).await?;
        return Ok(SwarmChatResponse {
            run_id:        "single".to_string(),
            final_content: result.content,
            leader_plan:   None,
            agent_results: vec![],
            stats:         result.stats,
            action_handled: result.action_handled,
            tools_used:    result.tools_used,
        });
    }

    // Resource gate
    let estimate = estimate_swarm_resources(state.clone()).await?;
    if !estimate.fits {
        return Err(BonsaiError::Orchestrator(format!(
            "Not enough RAM: need {} MB, only {} MB available. Disable an agent or choose a smaller model.",
            estimate.total_ram_required_mb, estimate.free_ram_mb
        )));
    }

    let user_prompt = messages.iter().rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();

    let run_id: String = {
        use rand::distributions::Alphanumeric;
        use rand::Rng;
        rand::thread_rng().sample_iter(&Alphanumeric).take(12).map(char::from).collect()
    };

    // Build cancel flags (one per slot, indexed by slot_index)
    let max_slot = enabled.iter().map(|a| a.config.slot_index).max().unwrap_or(0) as usize;
    let mut cancel_flags: Vec<Arc<std::sync::atomic::AtomicBool>> = (0..=max_slot)
        .map(|_| Arc::new(std::sync::atomic::AtomicBool::new(false)))
        .collect();
    // Ensure length matches agent slot indices
    while cancel_flags.len() <= max_slot {
        cancel_flags.push(Arc::new(std::sync::atomic::AtomicBool::new(false)));
    }

    {
        let mut cancels = state.swarm_cancels.lock().map_err(|_| "lock poisoned")?;
        cancels.insert(run_id.clone(), cancel_flags.clone());
    }

    let global_cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (resp_tx, resp_rx) = oneshot::channel();

    state.swarm_orchestrator.submit(SwarmRequest {
        run_id:         run_id.clone(),
        session_id:     None,
        user_prompt,
        workspace_path,
        enabled_tools,
        runtime_settings: swarm_settings.unwrap_or_default(),
        agents:         enabled,
        cancel_flags,
        global_cancel,
        resp_tx,
        app_handle,
    })?;

    let result: SwarmResult = resp_rx.await
        .map_err(|_| "Swarm request cancelled".to_string())?
        .map_err(|e| e)?;

    {
        let mut cancels = state.swarm_cancels.lock().map_err(|_| "lock poisoned")?;
        cancels.remove(&run_id);
    }

    Ok(SwarmChatResponse {
        run_id,
        final_content:  result.final_response,
        leader_plan:    result.leader_plan,
        agent_results:  result.agent_results,
        stats:          result.stats,
        action_handled: false,
        tools_used:     vec![],
    })
}

#[tauri::command]
pub async fn cancel_swarm(state: State<'_, AppState>, run_id: String) -> Result<(), String> {
    if !crate::features::FeatureFlags::is_enabled("swarm") {
        return Err("Swarm feature is disabled".into());
    }
    let cancels = state.swarm_cancels.lock().map_err(|_| "lock poisoned")?;
    if let Some(flags) = cancels.get(&run_id) {
        for f in flags { f.store(true, Ordering::Relaxed); }
    }
    Ok(())
}

#[tauri::command]
pub async fn get_swarm_metrics() -> Result<Vec<crate::swarm_orchestrator::SwarmRunRecord>, String> {
    if !crate::features::FeatureFlags::is_enabled("swarm") {
        return Err("Swarm feature is disabled".into());
    }
    Ok(crate::swarm_orchestrator::recent_swarm_runs())
}

#[tauri::command]
pub async fn cancel_agent(state: State<'_, AppState>, run_id: String, slot: usize) -> Result<(), String> {
    let cancels = state.swarm_cancels.lock().map_err(|_| "lock poisoned")?;
    if let Some(flags) = cancels.get(&run_id) {
        if let Some(f) = flags.get(slot) { f.store(true, Ordering::Relaxed); }
    }
    Ok(())
}

// ─── Messaging bot integration ───────────────────────────────────────────────

const BOT_ADMIN_PORT: u16 = 11666;
// bonsai-bot stores its admin token under its own keyring service, separate from
// the workspace's "bonsai-assistant" service used by SecretsStore.
const BOT_KEYRING_SERVICE: &str = "bonsai-bot";

fn bot_admin_token() -> String {
    keyring::Entry::new(BOT_KEYRING_SERVICE, "bot_admin_token")
        .ok()
        .and_then(|e| e.get_password().ok())
        .unwrap_or_default()
}

async fn fetch_from_bot_path(path: &str, token: &str) -> Result<(Value, u16), String> {
    let client = reqwest::Client::new();
    // Prefer a persisted port file written by the bot when available.
    fn read_persisted_bot_port() -> Option<u16> {
        // First try the OS config dir: {config_dir}/bonsai/bonsai-bot-port.json
        if let Some(cfg) = dirs::config_dir() {
            let path = cfg.join("bonsai").join("bonsai-bot-port.json");
            if path.exists() {
                if let Ok(s) = std::fs::read_to_string(&path) {
                    if let Ok(v) = serde_json::from_str::<Value>(&s) {
                        if let Some(p) = v.get("port").and_then(|n| n.as_u64()) {
                            return Some(p as u16);
                        }
                    }
                }
            }
        }

        // Fallback to local workspace file if present
        let local = std::path::Path::new("bonsai-bot-port.json");
        if local.exists() {
            if let Ok(s) = std::fs::read_to_string(local) {
                if let Ok(v) = serde_json::from_str::<Value>(&s) {
                    if let Some(p) = v.get("port").and_then(|n| n.as_u64()) {
                        return Some(p as u16);
                    }
                }
            }
        }
        None
    }

    if let Some(persisted) = read_persisted_bot_port() {
        let url = format!("http://127.0.0.1:{persisted}/{}", path.trim_start_matches('/'));
        if let Ok(resp) = client
            .get(&url)
            .bearer_auth(token)
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(v) = resp.json::<Value>().await {
                    return Ok((v, persisted));
                }
            }
        }
        // If persisted port failed, fall through to probing the default range.
    }

    for p in BOT_ADMIN_PORT..BOT_ADMIN_PORT.saturating_add(5) {
        let url = format!("http://127.0.0.1:{p}/{}", path.trim_start_matches('/'));
        match client
            .get(&url)
            .bearer_auth(token)
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<Value>().await {
                        Ok(v) => return Ok((v, p)),
                        Err(e) => return Err(e.to_string()),
                    }
                } else {
                    // try next port
                    continue;
                }
            }
            Err(_) => continue,
        }
    }
    Err("Bot server not running".to_string())
}

/// Read a persisted bonsai-bot port file (if present) from the OS config dir or
/// the local workspace, returning the port number when available.
#[tauri::command]
pub fn read_persisted_bot_port() -> Result<Option<u16>, String> {
    // First try the OS config dir: {config_dir}/bonsai/bonsai-bot-port.json
    if let Some(cfg) = dirs::config_dir() {
        let path = cfg.join("bonsai").join("bonsai-bot-port.json");
        if path.exists() {
            if let Ok(s) = std::fs::read_to_string(&path) {
                if let Ok(v) = serde_json::from_str::<Value>(&s) {
                    if let Some(p) = v.get("port").and_then(|n| n.as_u64()) {
                        return Ok(Some(p as u16));
                    }
                }
            }
        }
    }

    // Fallback to local workspace file if present
    let local = std::path::Path::new("bonsai-bot-port.json");
    if local.exists() {
        if let Ok(s) = std::fs::read_to_string(local) {
            if let Ok(v) = serde_json::from_str::<Value>(&s) {
                if let Some(p) = v.get("port").and_then(|n| n.as_u64()) {
                    return Ok(Some(p as u16));
                }
            }
        }
    }
    Ok(None)
}

/// Execute the reclaim-listener PowerShell script and return its output.
#[tauri::command]
pub fn run_reclaim_listener(
    ports: Option<Vec<u16>>,
    force_kill: Option<bool>,
    use_handle: Option<bool>,
) -> Result<String, String> {
    let script = std::path::Path::new("scripts").join("reclaim-listener.ps1");
    if !script.exists() {
        return Err(format!("reclaim script not found: {}", script.display()));
    }
    let mut cmd = std::process::Command::new("powershell");
    cmd.arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(script);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    if let Some(ports) = ports {
        let port_arg = ports.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(",");
        cmd.arg("-Ports").arg(port_arg);
    }
    if force_kill.unwrap_or(false) {
        cmd.arg("-ForceKill");
    }
    if use_handle.unwrap_or(false) {
        cmd.arg("-UseHandle");
    }
    match cmd.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Ok(format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr))
        }
        Err(e) => Err(e.to_string()),
    }
}

fn bot_config_path(app_handle: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("bonsai-bot-config.json"))
}

fn read_bot_cfg(path: &std::path::Path) -> Value {
    if path.exists() {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(json!({}))
    } else {
        json!({})
    }
}

fn write_bot_cfg(path: &std::path::Path, cfg: &Value) -> Result<(), String> {
    fs::write(path, serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

/// Fetch live status from the bot admin API.
/// Returns the raw JSON status object, or an error string if the bot is not running.
#[tauri::command]
pub async fn get_bot_server_status(_state: State<'_, AppState>) -> Result<Value, String> {
    if !crate::features::FeatureFlags::is_enabled("bot") {
        return Err("Bot feature is disabled".into());
    }
    let token = bot_admin_token();
    let (v, _p) = fetch_from_bot_path("status", &token).await?;
    Ok(v)
}

/// Fetch live metrics counters from the bot admin API.
#[tauri::command]
pub async fn get_bot_metrics(_state: State<'_, AppState>) -> Result<Value, String> {
    if !crate::features::FeatureFlags::is_enabled("bot") {
        return Err("Bot feature is disabled".into());
    }
    let token = bot_admin_token();
    let (v, _p) = fetch_from_bot_path("metrics", &token).await?;
    Ok(v)
}

/// Save Discord bot configuration: store token in keychain, persist non-secret settings to
/// the bot config file, then signal a reload if the bot is running.
#[tauri::command]
pub async fn save_discord_bot_config(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    token: String,
    allowed_guild_ids:   Vec<String>,
    allowed_channel_ids: Vec<String>,
    allowed_user_ids:    Vec<String>,
) -> Result<(), String> {
    if !crate::features::FeatureFlags::is_enabled("bot") {
        return Err("Bot feature is disabled".into());
    }
    if !token.is_empty() {
        state.secrets_store.store("discord_token", &token)
            .map_err(|e| e.to_string())?;
    }
    let path = bot_config_path(&app_handle)?;
    let mut cfg = read_bot_cfg(&path);
    cfg["discord"]["allowed_guild_ids"]   = json!(allowed_guild_ids);
    cfg["discord"]["allowed_channel_ids"] = json!(allowed_channel_ids);
    cfg["discord"]["allowed_user_ids"]    = json!(allowed_user_ids);
    cfg["discord"]["enabled"] = json!(true);
    write_bot_cfg(&path, &cfg)?;
    let _ = bot_reload(&state).await;
    Ok(())
}

/// Save Telegram bot configuration.
#[tauri::command]
pub async fn save_telegram_bot_config(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    token: String,
    allowed_chat_ids: Vec<i64>,
) -> Result<(), String> {
    if !crate::features::FeatureFlags::is_enabled("bot") {
        return Err("Bot feature is disabled".into());
    }
    if !token.is_empty() {
        state.secrets_store.store("telegram_token", &token)
            .map_err(|e| e.to_string())?;
    }
    let path = bot_config_path(&app_handle)?;
    let mut cfg = read_bot_cfg(&path);
    cfg["telegram"]["allowed_chat_ids"] = json!(allowed_chat_ids);
    cfg["telegram"]["enabled"] = json!(true);
    write_bot_cfg(&path, &cfg)?;
    let _ = bot_reload(&state).await;
    Ok(())
}

/// Save Matrix bot configuration.
#[tauri::command]
pub async fn save_matrix_bot_config(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    password:       String,
    homeserver_url: String,
    username:       String,
    allowed_rooms:  Vec<String>,
    allowed_users:  Vec<String>,
) -> Result<(), String> {
    if !crate::features::FeatureFlags::is_enabled("bot") {
        return Err("Bot feature is disabled".into());
    }
    if !password.is_empty() {
        state.secrets_store.store("matrix_password", &password)
            .map_err(|e| e.to_string())?;
    }
    let path = bot_config_path(&app_handle)?;
    let mut cfg = read_bot_cfg(&path);
    cfg["matrix"]["homeserver_url"] = json!(homeserver_url);
    cfg["matrix"]["username"]       = json!(username);
    cfg["matrix"]["allowed_rooms"]  = json!(allowed_rooms);
    cfg["matrix"]["allowed_users"]  = json!(allowed_users);
    cfg["matrix"]["enabled"] = json!(true);
    write_bot_cfg(&path, &cfg)?;
    let _ = bot_reload(&state).await;
    Ok(())
}

/// Save Email bot configuration.
#[tauri::command]
pub async fn save_email_bot_config(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    imap_password:      String,
    smtp_password:      String,
    imap_host:          String,
    imap_port:          u16,
    imap_username:      String,
    smtp_host:          String,
    smtp_username:      String,
    smtp_from:          String,
    subject_prefix:     String,
    allowed_from_addrs: Vec<String>,
) -> Result<(), String> {
    if !crate::features::FeatureFlags::is_enabled("bot") {
        return Err("Bot feature is disabled".into());
    }
    if !imap_password.is_empty() {
        state.secrets_store.store("email_imap_password", &imap_password)
            .map_err(|e| e.to_string())?;
    }
    if !smtp_password.is_empty() {
        state.secrets_store.store("email_smtp_password", &smtp_password)
            .map_err(|e| e.to_string())?;
    }
    let path = bot_config_path(&app_handle)?;
    let mut cfg = read_bot_cfg(&path);
    cfg["email"]["imap_host"]          = json!(imap_host);
    cfg["email"]["imap_port"]          = json!(imap_port);
    cfg["email"]["imap_username"]      = json!(imap_username);
    cfg["email"]["smtp_host"]          = json!(smtp_host);
    cfg["email"]["smtp_username"]      = json!(smtp_username);
    cfg["email"]["smtp_from"]          = json!(smtp_from);
    cfg["email"]["subject_prefix"]     = json!(subject_prefix);
    cfg["email"]["allowed_from_addrs"] = json!(allowed_from_addrs);
    cfg["email"]["enabled"] = json!(true);
    write_bot_cfg(&path, &cfg)?;
    let _ = bot_reload(&state).await;
    Ok(())
}

/// Test a platform's current connection state by querying the bot admin API status.
#[tauri::command]
pub async fn test_bot_platform(
    state: State<'_, AppState>,
    platform: String,
) -> Result<Value, String> {
    if !crate::features::FeatureFlags::is_enabled("bot") {
        return Err("Bot feature is disabled".into());
    }
    let status = get_bot_server_status(state).await?;
    Ok(status.get("platforms")
        .and_then(|p| p.get(&platform))
        .cloned()
        .unwrap_or(json!({"connected": false, "error": "Platform not found"})))
}

/// Reveal the Matrix key backup passphrase.
/// Requires the caller to supply the current `bot_admin_token` as proof of authorization.
/// Emits an audit event and returns the passphrase for one-time display.
#[tauri::command]
pub async fn get_matrix_key_backup_passphrase(
    state: State<'_, AppState>,
    admin_token_proof: String,
) -> Result<Option<String>, String> {
    let stored_token = bot_admin_token();
    if stored_token.is_empty() || admin_token_proof != stored_token {
        return Err("Unauthorized: invalid admin token proof".to_string());
    }
    state.audit_log.log(crate::assistant_audit_log::AuditEvent {
        ts:           std::time::SystemTime::now()
                          .duration_since(std::time::UNIX_EPOCH)
                          .unwrap_or_default()
                          .as_secs() as i64,
        tool:         "matrix_key_backup_passphrase_reveal".to_string(),
        decision:     "allowed".to_string(),
        args_hash:    String::new(),
        error:        None,
        duration_ms:  None,
        session_id:   None,
        turn_id:      None,
        tool_call_id: None,
    });
    let passphrase = keyring::Entry::new(BOT_KEYRING_SERVICE, "matrix_key_backup_pass")
        .ok()
        .and_then(|e| e.get_password().ok());
    Ok(passphrase)
}

/// Internal helper: POST /config/reload to the bot admin API (best-effort).
async fn bot_reload(_state: &State<'_, AppState>) -> Result<(), ()> {
    let token = bot_admin_token();
    let _ = fetch_from_bot_path("config/reload", &token).await;
    Ok(())
}

// ─── Model Data ──────────────────────────────────────────────────────────────

use crate::inference_mode::InferenceMode;
use crate::model_data::{GenerateModelDataInput, ModelData, ModelDataSummary};
use crate::model_data_generator::ModelDataGenerator;

/// List all model data entries (summaries — lighter than full records).
#[tauri::command]
pub async fn list_model_data(state: State<'_, AppState>) -> Result<Vec<ModelDataSummary>, String> {
    state.model_data_store
        .list_summaries()
        .await
        .map_err(|e| e.to_string())
}

/// Get the full ModelData record for a single model.
#[tauri::command]
pub async fn get_model_data(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<ModelData>, String> {
    state.model_data_store
        .get(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Create or update a ModelData record. Returns the saved ID.
#[tauri::command]
pub async fn save_model_data(
    state: State<'_, AppState>,
    mut data: ModelData,
) -> Result<String, String> {
    data.touch();
    state.model_data_store
        .save(&data)
        .await
        .map_err(|e| e.to_string())?;
    Ok(data.id)
}

/// Delete a ModelData record.
#[tauri::command]
pub async fn delete_model_data(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.model_data_store
        .delete(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Search model data by keyword.
#[tauri::command]
pub async fn search_model_data(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<ModelDataSummary>, String> {
    let results = state.model_data_store
        .search(&query)
        .await
        .map_err(|e| e.to_string())?;
    Ok(results.iter().map(ModelDataSummary::from).collect())
}

/// Return models ranked best-first for a given skill/tool ID.
#[tauri::command]
pub async fn rank_models_for_skill(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<Vec<ModelDataSummary>, String> {
    let results = state.model_data_store
        .rank_for_skill(&skill_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(results.iter().map(ModelDataSummary::from).collect())
}

/// Auto-generate a ModelData draft using the built-in knowledge base + LLM.
/// The result is NOT automatically saved — the frontend shows it for review first.
#[tauri::command]
pub async fn generate_model_data(
    state: State<'_, AppState>,
    input: GenerateModelDataInput,
) -> Result<ModelData, String> {
    let generator = ModelDataGenerator::new(state.orchestrator.clone());
    match input {
        GenerateModelDataInput::FromRegistry { registry_id } => {
            let models = state.orchestrator.list_models().await;
            let info   = models.iter()
                .find(|m| m.id == registry_id)
                .ok_or_else(|| format!("registry model '{registry_id}' not found"))?;
            generator.from_registry_info(info).await.map_err(|e| e.to_string())
        }
        GenerateModelDataInput::FromProvider { provider, model_id, base_url } => {
            generator
                .from_provider(&provider, &model_id, base_url.as_deref())
                .await
                .map_err(|e| e.to_string())
        }
    }
}

/// Ensure every local GGUF in the registry has a ModelData entry.
/// Skips models that already have an entry. Returns the count of new entries created.
#[tauri::command]
pub async fn sync_registry_to_model_data(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let models = state.orchestrator.list_models().await;
    let default_mode = crate::config::load_config(&app_handle)
        .map(|c| c.default_inference_mode)
        .unwrap_or_default();
    state.model_data_store
        .sync_from_registry(&models, &default_mode)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_default_inference_mode(app_handle: AppHandle) -> Result<InferenceMode, String> {
    let cfg = crate::config::load_config(&app_handle)?;
    Ok(cfg.default_inference_mode)
}

#[tauri::command]
pub async fn set_default_inference_mode(
    app_handle: AppHandle,
    mode: InferenceMode,
) -> Result<InferenceMode, String> {
    let mut cfg = crate::config::load_config(&app_handle)?;
    cfg.default_inference_mode = mode.clone();
    crate::config::save_config(&app_handle, &cfg)?;
    Ok(mode)
}

#[tauri::command]
pub async fn get_inference_mode(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> Result<InferenceMode, String> {
    if let Some(data) = state
        .model_data_store
        .find_by_registry_id(&model_id)
        .await
        .map_err(|e| e.to_string())?
    {
        return Ok(data.inference_mode);
    }
    let cfg = crate::config::load_config(&app_handle)?;
    Ok(cfg.default_inference_mode)
}

#[tauri::command]
pub async fn set_inference_mode(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    model_id: String,
    mode: InferenceMode,
) -> Result<InferenceMode, String> {
    let mut data = if let Some(existing) = state
        .model_data_store
        .find_by_registry_id(&model_id)
        .await
        .map_err(|e| e.to_string())?
    {
        existing
    } else {
        let models = state.orchestrator.list_models().await;
        let info = models
            .iter()
            .find(|m| m.id == model_id)
            .ok_or_else(|| format!("model '{model_id}' not found in registry"))?;
        ModelData::from_registry_with_mode(info, mode.clone())
    };

    data.inference_mode = mode.clone();
    data.touch();
    state
        .model_data_store
        .save(&data)
        .await
        .map_err(|e| e.to_string())?;

    state
        .orchestrator
        .set_inference_mode(model_id.clone(), mode.clone());

    if state.orchestrator.is_model_loaded(&model_id).await {
        state.orchestrator.unload_model(&model_id).await;
        let rx = state.orchestrator.load(model_id.clone());
        rx.await.map_err(|_| "Orchestrator offline".to_string())??;
    }

    let _ = app_handle.emit(
        "model-inference-mode-updated",
        serde_json::json!({
            "model_id": model_id,
            "mode": mode,
        }),
    );

    Ok(data.inference_mode)
}

#[tauri::command]
pub async fn apply_inference_mode_to_all(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    mode: InferenceMode,
) -> Result<usize, String> {
    let mut cfg = crate::config::load_config(&app_handle)?;
    cfg.default_inference_mode = mode.clone();
    crate::config::save_config(&app_handle, &cfg)?;

    let models = state.orchestrator.list_models().await;
    state
        .model_data_store
        .sync_from_registry(&models, &mode)
        .await
        .map_err(|e| e.to_string())?;

    let mut updated = 0usize;
    for m in models {
        let mut data = match state
            .model_data_store
            .find_by_registry_id(&m.id)
            .await
            .map_err(|e| e.to_string())?
        {
            Some(d) => d,
            None => ModelData::from_registry_with_mode(&m, mode.clone()),
        };
        data.inference_mode = mode.clone();
        data.touch();
        state
            .model_data_store
            .save(&data)
            .await
            .map_err(|e| e.to_string())?;
        state
            .orchestrator
            .set_inference_mode(m.id.clone(), mode.clone());
        updated += 1;
    }

    Ok(updated)
}

// ─── Model directories ────────────────────────────────────────────────────────

/// List all configured model directories (bootstrap dir + extra dirs).
#[tauri::command]
pub async fn list_model_directories(app_handle: AppHandle) -> Result<Vec<String>, String> {
    let cfg = crate::config::load_config(&app_handle).map_err(|e| e.to_string())?;
    let bootstrap = crate::bootstrap::models_dir(&app_handle).display().to_string();
    let mut dirs = vec![bootstrap];
    dirs.extend(cfg.extra_model_dirs);
    Ok(dirs)
}

/// Add a directory to scan for .gguf models. Refreshes the registry immediately.
#[tauri::command]
pub async fn add_model_directory(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let canonical = std::path::Path::new(&path)
        .canonicalize()
        .map_err(|e| format!("Cannot access '{path}': {e}"))?;
    let canonical_str = canonical.display().to_string();

    let mut cfg = crate::config::load_config(&app_handle).map_err(|e| e.to_string())?;
    if !cfg.extra_model_dirs.contains(&canonical_str) {
        cfg.extra_model_dirs.push(canonical_str);
        crate::config::save_config(&app_handle, &cfg).map_err(|e| e.to_string())?;
    }

    state.orchestrator.refresh_registry();
    Ok(())
}

/// Remove an extra model directory. Does not remove the bootstrap directory.
#[tauri::command]
pub async fn remove_model_directory(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let mut cfg = crate::config::load_config(&app_handle).map_err(|e| e.to_string())?;
    cfg.extra_model_dirs.retain(|d| d != &path);
    crate::config::save_config(&app_handle, &cfg).map_err(|e| e.to_string())?;
    state.orchestrator.refresh_registry();
    Ok(())
}

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{
        build_resume_continuation_payloads,
        finalize_tool_only_response,
        has_parent_dir_component,
        is_file_inventory_request,
        tool_denied_message,
        tool_name_from_action,
    };
    use serde_json::json;

    #[test]
    fn traversal_blocked() {
        assert!(has_parent_dir_component("../../etc/passwd"));
        assert!(has_parent_dir_component("../sibling"));
        assert!(has_parent_dir_component("a/b/../../secret"));
        assert!(has_parent_dir_component("..\\windows\\system32"));
    }

    #[test]
    fn safe_paths_allowed() {
        assert!(!has_parent_dir_component("src/main.rs"));
        assert!(!has_parent_dir_component("src\\main.rs"));
        assert!(!has_parent_dir_component("foo..bar.txt"));
        assert!(!has_parent_dir_component("."));
        assert!(!has_parent_dir_component("models/ggml-base.en.bin"));
    }

    #[test]
    fn file_inventory_requests_detected() {
        assert!(is_file_inventory_request("List files in this folder"));
        assert!(is_file_inventory_request("Please list all files"));
        assert!(is_file_inventory_request("Show files"));
        assert!(is_file_inventory_request("Readme"));
        assert!(is_file_inventory_request("Read the file README.md"));
        assert!(is_file_inventory_request("Can you list files in this directory?"));
    }

    #[test]
    fn non_inventory_requests_ignored() {
        assert!(!is_file_inventory_request("Summarize this code"));
        assert!(!is_file_inventory_request("Run the test suite"));
        assert!(!is_file_inventory_request("How can I optimize startup?"));
    }

    #[test]
    fn tool_only_response_includes_output() {
        let msg = finalize_tool_only_response("read_file", "# README\nHello");
        assert!(msg.contains("I used `read_file`"));
        assert!(msg.contains("# README"));
    }

    #[test]
    fn tool_only_response_handles_empty_output() {
        let msg = finalize_tool_only_response("read_file", "   ");
        assert!(msg.contains("returned no output"));
    }

    #[test]
    fn tool_name_from_action_defaults_when_missing() {
        assert_eq!(tool_name_from_action(&json!({})), "tool");
        assert_eq!(tool_name_from_action(&json!({"tool": "write_file"})), "write_file");
    }

    #[test]
    fn denied_message_contains_tool() {
        let msg = tool_denied_message("run_command");
        assert!(msg.contains("run_command"));
        assert!(msg.contains("denied"));
    }

    #[test]
    fn resume_payloads_exclude_system_and_append_tool_result() {
        let ctx_snapshot = vec![
            json!({"role": "system", "content": "policy"}),
            json!({"role": "user", "content": "Please read README"}),
            json!({"role": "assistant", "content": "<tool_call>{\"tool\":\"read_file\",\"args\":{\"path\":\"README.md\"}}</tool_call>"}),
        ];

        let payloads = build_resume_continuation_payloads(
            &ctx_snapshot,
            "assistant raw response",
            "# README\nhello",
        );

        assert_eq!(payloads.len(), 4);
        assert_eq!(payloads[0].role, "user");
        assert_eq!(payloads[0].content, "Please read README");
        assert_eq!(payloads[1].role, "assistant");
        assert!(payloads[1].content.contains("<tool_call>"));
        assert_eq!(payloads[2].role, "assistant");
        assert_eq!(payloads[2].content, "assistant raw response");
        assert_eq!(payloads[3].role, "user");
        assert!(payloads[3].content.contains("<tool_result>"));
        assert!(payloads[3].content.contains("# README"));
    }

    // ── Connection / pairing ────────────────────────────────────────────────

    /// Token generation logic: rand::thread_rng + Alphanumeric produces an
    /// 8-char alphanumeric string.  Test the shape, not the exact value.
    #[test]
    fn pair_token_format() {
        use rand::distributions::Alphanumeric;
        use rand::Rng;
        let token: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();

        assert_eq!(token.len(), 8, "token must be 8 characters");
        assert!(token.chars().all(|c| c.is_ascii_alphanumeric()),
            "token must be alphanumeric, got: {token}");
    }

    /// QR code generation produces valid SVG containing an `<svg` root element.
    #[test]
    fn generate_pair_qr_returns_svg() {
        use qrcode::QrCode;
        use qrcode::render::svg;

        let data = format!(
            "bonsai://connect?ip=192.168.1.100&port={}&token=ABCD1234",
            crate::config::DEFAULT_API_PORT
        );
        let code = QrCode::new(data.as_bytes()).expect("QR code creation failed");
        let svg_str = code.render::<svg::Color>().min_dimensions(200, 200).build();

        assert!(svg_str.contains("<svg"), "output must be an SVG document");
        assert!(svg_str.contains("</svg>"), "SVG must be properly closed");
        assert!(!svg_str.is_empty(), "SVG must not be empty");
    }

    /// QR code for empty payload must still succeed.
    #[test]
    fn generate_pair_qr_empty_data() {
        use qrcode::QrCode;
        use qrcode::render::svg;

        let code = QrCode::new(b"").expect("QR code with empty data failed");
        let svg_str = code.render::<svg::Color>().min_dimensions(100, 100).build();
        assert!(svg_str.contains("<svg"));
    }

    /// WsRouter integration: pair token flows into broadcast correctly.
    #[tokio::test]
    async fn ws_router_broadcasts_pair_info() {
        use crate::ws_router::WsRouter;
        use axum::extract::ws::Message;
        use serde_json::json;

        let router = WsRouter::new();
        let (_id, mut rx) = router.register();

        let payload = json!({"type": "pair_info", "payload": {"token": "TEST1234"}});
        router.broadcast(Message::Text(payload.to_string()));

        let msg = rx.recv().await.expect("should receive broadcast");
        let Message::Text(txt) = msg else { panic!("expected text message") };
        let v: serde_json::Value = serde_json::from_str(&txt).unwrap();
        assert_eq!(v["type"], "pair_info");
        assert_eq!(v["payload"]["token"], "TEST1234");
    }

    /// Local IP resolution returns a parseable IP address.
    #[test]
    fn local_ip_is_valid() {
        match local_ip_address::local_ip() {
            Ok(ip) => {
                let s = ip.to_string();
                // Must parse back to a valid IP.
                let parsed: std::net::IpAddr = s.parse()
                    .unwrap_or_else(|_| panic!("local_ip returned unparseable string: {s}"));
                // Must not be loopback (127.x or ::1).
                assert!(!parsed.is_loopback(), "local_ip should not return loopback, got {parsed}");
            }
            // In CI / sandboxed environments there may be no network — skip.
            Err(e) => tracing::warn!(error=%e, "[skip] local_ip_address unavailable"),
        }
    }
}

// ── Agent Host commands ───────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_agents(
    state: State<'_, crate::AppState>,
) -> Result<Vec<crate::agent::AgentMetadata>, String> {
    Ok(state.agent_host.list().await)
}

#[tauri::command]
pub async fn send_agent_message(
    state:    State<'_, crate::AppState>,
    agent_id: String,
    message:  crate::agent::AgentMessage,
) -> Result<crate::agent::AgentOutput, String> {
    let ctx = crate::agent::AgentContext {
        model_url: state.orchestrator.active_slot_url().await,
    };
    state.agent_host.handle(&agent_id, ctx, message).await.map_err(|e| e.to_string())
}
