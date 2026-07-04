use serde::Deserialize;
use tauri::{AppHandle, Emitter};

/// Every response the agent can produce.
#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)] // Variants/fields are parsed from agent JSON; not all are acted on yet
pub enum AgentAction {
    FileEdit {
        path: String,
        diff: String,
        rationale: String,
    },
    FileCreate {
        path: String,
        content: String,
        rationale: String,
    },
    FileDelete {
        path: String,
        paths_affected: Vec<String>,
        rationale: String,
    },
    ShellCommand {
        command: String,
        working_dir: String,
        rationale: String,
    },
    AskPermission {
        description: String,
        paths_affected: Vec<String>,
        rationale: String,
    },
    ReadFile {
        path: String,
        line_range: Option<String>,
        rationale: Option<String>,
    },
    Message {
        text: String,
        rationale: Option<String>,
    },
}

pub fn parse_agent_json(s: &str) -> Result<AgentAction, String> {
    // Strip ```json fences if the model included them
    let clean = s
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    serde_json::from_str(clean).map_err(|e| format!("JSON parse error: {e}  raw: {clean}"))
}

pub async fn handle_agent_response(app_handle: &AppHandle, raw_json: String) -> Result<(), String> {
    let action = parse_agent_json(&raw_json)?;

    match action {
        AgentAction::FileCreate {
            path,
            content,
            rationale,
        } => {
            // Sanitize: only allow paths that are relative and don't escape
            let safe_path = std::path::Path::new(&path);
            if safe_path.is_absolute() || path.contains("..") {
                return Err(format!("Unsafe path rejected: {path}"));
            }
            if let Some(parent) = safe_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            crate::atomic_write(std::path::Path::new(&path), content.as_bytes())
                .map_err(|e| e.to_string())?;
            let _ = app_handle.emit(
                "agent-response",
                serde_json::json!({
                    "type": "file_create",
                    "path": path,
                    "rationale": rationale
                }),
            );
        }

        AgentAction::FileEdit {
            path,
            diff,
            rationale,
        } => {
            let _ = app_handle.emit(
                "agent-response",
                serde_json::json!({
                    "type": "file_edit",
                    "path": path,
                    "diff": diff,
                    "rationale": rationale
                }),
            );
        }

        AgentAction::Message { text, .. } => {
            let _ = app_handle.emit(
                "agent-response",
                serde_json::json!({
                    "type": "message",
                    "text": text
                }),
            );
        }

        AgentAction::AskPermission {
            description,
            paths_affected,
            rationale,
        } => {
            let _ = app_handle.emit(
                "permission-request",
                serde_json::json!({
                    "description": description,
                    "paths_affected": paths_affected,
                    "rationale": rationale
                }),
            );
        }

        AgentAction::ShellCommand {
            command,
            working_dir,
            rationale,
        } => {
            let _ = app_handle.emit(
                "permission-request",
                serde_json::json!({
                    "type": "shell_command",
                    "description": format!("Run shell command: {command}"),
                    "paths_affected": [working_dir],
                    "rationale": rationale,
                    "command": command
                }),
            );
        }

        _ => {}
    }

    Ok(())
}
