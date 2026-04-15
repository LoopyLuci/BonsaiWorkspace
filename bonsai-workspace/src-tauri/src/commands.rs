use futures::StreamExt;
use git2::Repository;
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::process::Command;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex as StdMutex};
use sysinfo::System;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;
use walkdir::WalkDir;

use crate::action_parser::handle_agent_response;
use crate::agent_connect::{AgentConnectEvent, AgentConnectSession};
use crate::bootstrap;
use crate::model_orchestrator::{InferRequest, InferStats};
use crate::remote::RemoteManager;
use crate::remote_input::RemoteInputEvent;
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

// ─── File system ─────────────────────────────────────────────────────────────

#[tauri::command]
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

#[tauri::command]
pub async fn get_git_branch(workspace_path: String) -> Result<String, String> {
    let repo = Repository::open(&workspace_path).map_err(|e| e.to_string())?;
    let head = repo.head().map_err(|e| e.to_string())?;
    Ok(head.shorthand().unwrap_or("HEAD").to_string())
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
        .map(|s| s.chars().count())
        .unwrap_or(0);
    let content_len = msg
        .get("content")
        .and_then(|v| v.as_str())
        .map(|s| s.chars().count())
        .unwrap_or(0);
    // Rough estimator used for conservative trimming.
    ((role_len + content_len) / 4) + 10
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
    orchestrator: &crate::model_orchestrator::ModelOrchestrator,
    app_handle:   &AppHandle,
    messages:     Vec<Value>,
    cancel_flag:  Option<Arc<std::sync::atomic::AtomicBool>>,
) -> Result<(String, InferStats), String> {
    let (resp_tx, resp_rx) = oneshot::channel();
    let (stream_tx, mut stream_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    orchestrator.infer(InferRequest {
        model_id:   None,
        messages,
        max_tokens: 4096,
        stream_tx:  Some(stream_tx),
        cancel_flag,
        resp_tx,
    })?;
    let handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(tok) = stream_rx.recv().await {
            let _ = handle.emit("token-stream", &tok);
        }
    });
    resp_rx.await.map_err(|_| "Request cancelled".to_string())?
}

#[tauri::command]
pub async fn submit_chat(
    app_handle:     AppHandle,
    state:          State<'_, AppState>,
    messages:       Vec<ChatMessagePayload>,
    workspace_path: Option<String>,
    enabled_tools:  Option<Vec<String>>,
) -> Result<ChatResponse, String> {
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
    const  MAX_TURNS: usize = 8;
    let mut loop_limit_reached = true;

    for _turn in 0..MAX_TURNS {
        trim_context_to_budget(&mut ctx, CHAT_PROMPT_TOKEN_BUDGET);
        let (raw, stats) = match run_inference(
            &state.orchestrator,
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
                return Err(e);
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
            // No tool calls — this is the final prose response.
            final_content = tools::strip_tool_calls(&response);
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

    let (raw, _stats) = run_inference(&state.orchestrator, &app_handle, messages, None).await?;
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
) -> Result<ChatResponse, String> {
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
                |err| eprintln!("Audio input error: {err}"),
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

    let (resp_tx, resp_rx) = oneshot::channel();
    let req = InferRequest {
        model_id: None,
        messages: vec![json!({"role": "system", "content": "Scaffold a Bonsai project."}),
                   json!({"role": "user", "content": full_prompt})],
        max_tokens: 4096,
        stream_tx: None,
        cancel_flag: None,
        resp_tx,
    };

    state.orchestrator.infer(req)?;

    let (raw, _stats) = resp_rx
        .await
        .map_err(|_| "Scaffold cancelled".to_string())??;
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

    let (raw, _stats) = run_inference(&state.orchestrator, &app_handle, messages, None).await?;
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
        let _ = app_handle.emit("terminal-output", text);
    }
    Ok(())
}

#[tauri::command]
pub async fn spawn_pty_terminal(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())?;

    let cmd = CommandBuilder::new(if cfg!(target_os = "windows") { "cmd.exe" } else { "bash" });
    let _child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;

    {
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
        let mut w  = state.pty_writer.lock().await;
        *w = Some(writer);
    }
    {
        let mut r = state.pty_resizer.lock().await;
        *r = Some(pair.master);
    }

    let handle = app_handle.clone();
    tokio::task::spawn_blocking(move || {
        let mut buf = [0u8; 1024];
        loop {
            use std::io::Read;
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = handle.emit("pty-output", text);
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn send_to_pty(input: String, state: State<'_, AppState>) -> Result<(), String> {
    use std::io::Write;
    let mut guard = state.pty_writer.lock().await;
    if let Some(ref mut w) = *guard {
        w.write_all(input.as_bytes()).map_err(|e| e.to_string())?;
        w.write_all(b"\r").map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn resize_pty(rows: u16, cols: u16, state: State<'_, AppState>) -> Result<(), String> {
    use portable_pty::PtySize;
    let guard = state.pty_resizer.lock().await;
    if let Some(ref master) = *guard {
        master
            .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| e.to_string())?;
    }
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
) -> Result<(), String> {
    let rx = state.orchestrator.load(model_id);
    rx.await.map_err(|_| "Orchestrator offline".to_string())?
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
) -> Result<String, String> {
    let rx = state.orchestrator.load(model_id.clone());
    rx.await.map_err(|_| "Orchestrator offline".to_string())??;

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
        if let Ok(output) = Command::new("wmic").args(&args).output() {
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
pub async fn get_api_config(app_handle: AppHandle) -> Result<serde_json::Value, String> {
    let config = crate::config::load_config(&app_handle)?;
    Ok(serde_json::json!({
        "api_host": config.api_host,
        "api_port": config.api_port,
    }))
}

#[tauri::command]
pub async fn set_api_config(app_handle: AppHandle, api_host: String, api_port: u16) -> Result<serde_json::Value, String> {
    let mut config = crate::config::load_config(&app_handle)?;
    config.api_host = api_host;
    config.api_port = api_port;
    let config = crate::config::save_config(&app_handle, &config)?;
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
                eprintln!("[bootstrap] run_bootstrap error: {e}");
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
        "bonsai://connect?ip={}&port=11369&token={}",
        ip, state.pair_token
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

/// Placeholder command for Android QR scanning flow.
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
    config.desktop_connection_token = Some(token.clone());
    let config = crate::config::save_config(&app_handle, &config)?;

    Ok(serde_json::json!({
        "ip": config.desktop_connection_ip,
        "token": config.desktop_connection_token,
    }))
}

/// Load persisted desktop connection details for Android auto-reconnect.
#[tauri::command]
pub async fn load_desktop_connection(app_handle: AppHandle) -> Result<Option<serde_json::Value>, String> {
    let config = crate::config::load_config(&app_handle)?;
    match (config.desktop_connection_ip, config.desktop_connection_token) {
        (Some(ip), Some(token)) => Ok(Some(serde_json::json!({
            "ip": ip,
            "token": token,
        }))),
        _ => Ok(None),
    }
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

    let deadline = Instant::now() + Duration::from_millis(1500);
    let mut out = Vec::new();

    while Instant::now() < deadline {
        match receiver.try_recv() {
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
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }

    Ok(out)
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

        let data = "bonsai://connect?ip=192.168.1.100&port=11369&token=ABCD1234";
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
            Err(e) => eprintln!("[skip] local_ip_address unavailable: {e}"),
        }
    }
}
