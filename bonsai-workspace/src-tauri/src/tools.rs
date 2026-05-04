//! Tool system for Bonsai Workspace.
//!
//! Built-in tools (read_file, list_files, search_files, grep_files, write_file, run_command, etc.)
//! plus custom tools loaded from `{workspace}/bonsai-tools/` directories.
//!
//! Uses ReAct-style prompting so ANY model (including those without native function-
//! calling support) can invoke tools by outputting `<tool_call>...</tool_call>` tags.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use regex::RegexBuilder;

// ── Tool schema types ─────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToolArg {
    pub name:        String,
    #[serde(rename = "type")]
    pub arg_type:    String,
    pub description: String,
    #[serde(default = "default_true")]
    pub required:    bool,
}

fn default_true() -> bool { true }

#[derive(Serialize, Clone, Debug)]
pub struct ToolDef {
    pub name:              String,
    pub description:       String,
    pub args:              Vec<ToolArg>,
    /// true = show HITL approval card before executing
    pub requires_approval: bool,
    pub is_custom:         bool,
    /// Path to custom script (non-null for is_custom = true)
    pub script_path:       Option<String>,
}

// ── Built-in tools ────────────────────────────────────────────────────────────

pub fn built_in_tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "read_file".into(),
            description: "Read the full contents of a file. Use to inspect source code, configs, or documentation.".into(),
            args: vec![ToolArg {
                name: "path".into(), arg_type: "string".into(),
                description: "Absolute path to the file to read.".into(), required: true,
            }],
            requires_approval: false,
            is_custom: false, script_path: None,
        },
        ToolDef {
            name: "list_files".into(),
            description: "List files and directories in a folder with deterministic ordering and pagination. Use this for complete directory inventories.".into(),
            args: vec![
                ToolArg {
                    name: "path".into(), arg_type: "string".into(),
                    description: "Absolute path to the directory to list.".into(), required: true,
                },
                ToolArg {
                    name: "recursive".into(), arg_type: "boolean".into(),
                    description: "If true, include nested entries. Default false.".into(), required: false,
                },
                ToolArg {
                    name: "max_depth".into(), arg_type: "number".into(),
                    description: "Maximum recursion depth when recursive=true. Default 8, max 32.".into(), required: false,
                },
                ToolArg {
                    name: "offset".into(), arg_type: "number".into(),
                    description: "Zero-based pagination offset. Default 0.".into(), required: false,
                },
                ToolArg {
                    name: "limit".into(), arg_type: "number".into(),
                    description: "Max entries to return per call. Default 200, max 1000.".into(), required: false,
                },
                ToolArg {
                    name: "include_hidden".into(), arg_type: "boolean".into(),
                    description: "If true, include dotfiles and hidden directories. Default false.".into(), required: false,
                },
            ],
            requires_approval: false,
            is_custom: false, script_path: None,
        },
        ToolDef {
            name: "list_all_files".into(),
            description: "List all files recursively in a folder with deterministic ordering and pagination. If path is omitted, use the current workspace folder.".into(),
            args: vec![
                ToolArg {
                    name: "path".into(), arg_type: "string".into(),
                    description: "Optional absolute path to the directory. If omitted, current workspace path is used.".into(), required: false,
                },
                ToolArg {
                    name: "offset".into(), arg_type: "number".into(),
                    description: "Zero-based pagination offset. Default 0.".into(), required: false,
                },
                ToolArg {
                    name: "limit".into(), arg_type: "number".into(),
                    description: "Max entries to return per call. Default 200, max 1000.".into(), required: false,
                },
                ToolArg {
                    name: "include_hidden".into(), arg_type: "boolean".into(),
                    description: "If true, include dotfiles and hidden directories. Default false.".into(), required: false,
                },
            ],
            requires_approval: false,
            is_custom: false, script_path: None,
        },
        ToolDef {
            name: "search_files".into(),
            description: "Search for text across all files in a directory (case-insensitive, returns file:line:content matches).".into(),
            args: vec![
                ToolArg { name: "path".into(),  arg_type: "string".into(), description: "Directory to search in.".into(),  required: true },
                ToolArg { name: "query".into(), arg_type: "string".into(), description: "Text to search for.".into(), required: true },
            ],
            requires_approval: false,
            is_custom: false, script_path: None,
        },
        ToolDef {
            name: "grep_files".into(),
            description: "Search file contents using a regular expression and return file:line:content matches.".into(),
            args: vec![
                ToolArg { name: "path".into(), arg_type: "string".into(), description: "Directory to search in.".into(), required: true },
                ToolArg { name: "pattern".into(), arg_type: "string".into(), description: "Regex pattern to match.".into(), required: true },
                ToolArg { name: "case_sensitive".into(), arg_type: "boolean".into(), description: "Whether regex matching should be case-sensitive (default false).".into(), required: false },
                ToolArg { name: "max_results".into(), arg_type: "number".into(), description: "Maximum matches to return (default 60, max 200).".into(), required: false },
            ],
            requires_approval: false,
            is_custom: false, script_path: None,
        },
        ToolDef {
            name: "write_file".into(),
            description: "Write or overwrite a file with new content. Creates parent directories if needed. REQUIRES USER APPROVAL.".into(),
            args: vec![
                ToolArg { name: "path".into(),    arg_type: "string".into(), description: "Absolute path to the file to write.".into(), required: true },
                ToolArg { name: "content".into(), arg_type: "string".into(), description: "Content to write to the file.".into(),         required: true },
            ],
            requires_approval: true,
            is_custom: false, script_path: None,
        },
        ToolDef {
            name: "edit_file".into(),
            description: "Edit a file by exact string replacement. Replaces one unique old_string with new_string. REQUIRES USER APPROVAL.".into(),
            args: vec![
                ToolArg { name: "path".into(),       arg_type: "string".into(), description: "Absolute path to the file to edit.".into(), required: true },
                ToolArg { name: "old_string".into(), arg_type: "string".into(), description: "Exact text to replace (must occur exactly once).".into(), required: true },
                ToolArg { name: "new_string".into(), arg_type: "string".into(), description: "Replacement text.".into(), required: true },
            ],
            requires_approval: true,
            is_custom: false, script_path: None,
        },
        ToolDef {
            name: "create_dir".into(),
            description: "Create a directory and all missing parent directories. REQUIRES USER APPROVAL.".into(),
            args: vec![ToolArg {
                name: "path".into(), arg_type: "string".into(),
                description: "Absolute path of the directory to create.".into(), required: true,
            }],
            requires_approval: true,
            is_custom: false, script_path: None,
        },
        ToolDef {
            name: "delete_file".into(),
            description: "Delete a file or empty directory. REQUIRES USER APPROVAL. Use with extreme caution.".into(),
            args: vec![ToolArg {
                name: "path".into(), arg_type: "string".into(),
                description: "Absolute path to the file or directory to delete.".into(), required: true,
            }],
            requires_approval: true,
            is_custom: false, script_path: None,
        },
        ToolDef {
            name: "run_command".into(),
            description: "Execute a shell command and return its stdout/stderr output. ALWAYS REQUIRES USER APPROVAL. Supports aliases: 'specs', 'computer specs', 'system specs', and 'hardware info'.".into(),
            args: vec![ToolArg {
                name: "command".into(), arg_type: "string".into(),
                description: "Shell command to execute.".into(), required: true,
            }],
            requires_approval: true,
            is_custom: false, script_path: None,
        },
    ]
}

// ── Custom tool loading ───────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CustomToolManifest {
    name:               String,
    description:        String,
    #[serde(default)]
    args:               Vec<ToolArg>,
    script:             String,
    requires_approval:  Option<bool>,
}

/// Load custom tool manifests from `{workspace}/bonsai-tools/*.json`.
/// Each JSON file should contain a `CustomToolManifest`.
pub fn load_custom_tools(workspace_path: &Path) -> Vec<ToolDef> {
    let tools_dir = workspace_path.join("bonsai-tools");
    if !tools_dir.exists() { return vec![]; }

    let mut tools = Vec::new();
    let Ok(entries) = std::fs::read_dir(&tools_dir) else { return vec![] };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") { continue; }
        let Ok(content) = std::fs::read_to_string(&path) else { continue };
        let Ok(manifest) = serde_json::from_str::<CustomToolManifest>(&content) else {
            eprintln!("[tools] Failed to parse manifest: {}", path.display());
            continue;
        };
        let script_path = tools_dir.join(&manifest.script);
        tools.push(ToolDef {
            name:              manifest.name,
            description:       manifest.description,
            args:              manifest.args,
            requires_approval: manifest.requires_approval.unwrap_or(true),
            is_custom:         true,
            script_path:       Some(script_path.to_string_lossy().into_owned()),
        });
    }
    tools
}

/// Merge built-in tools with workspace custom tools.
pub fn all_tools(workspace_path: Option<&str>) -> Vec<ToolDef> {
    let mut tools = built_in_tools();
    if let Some(ws) = workspace_path {
        tools.extend(load_custom_tools(Path::new(ws)));
    }
    tools
}

// ── System prompt generation ──────────────────────────────────────────────────

/// Synchronously list the top 2 levels of a workspace directory (max 80 entries).
/// Returns a plain-text tree string suitable for embedding in a system prompt.
fn workspace_snapshot(root: &str) -> String {
    let mut lines = Vec::new();
    snapshot_dir(std::path::Path::new(root), 0, 2, &mut lines);
    lines.join("\n")
}

fn snapshot_dir(path: &std::path::Path, depth: usize, max_depth: usize, out: &mut Vec<String>) {
    if depth > max_depth { return; }
    let Ok(entries) = std::fs::read_dir(path) else { return };
    let mut sorted: Vec<_> = entries.flatten().collect();
    sorted.sort_by_key(|e| e.file_name());
    for entry in sorted {
        if out.len() >= 80 { break; }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || matches!(
            name.as_str(),
            "node_modules" | "target" | "dist" | "__pycache__" | ".git" | ".svelte-kit"
        ) {
            continue;
        }
        let indent = "  ".repeat(depth);
        let ep = entry.path();
        if ep.is_dir() {
            out.push(format!("{indent}{name}/"));
            snapshot_dir(&ep, depth + 1, max_depth, out);
        } else {
            out.push(format!("{indent}{name}"));
        }
    }
}

/// Build the ReAct-style system prompt injected before every tool-enabled conversation.
pub fn system_prompt(tools: &[ToolDef], workspace_path: Option<&str>) -> String {
    let mut s = String::from(
        "You are Bonsai, an AI coding assistant running locally on the user's device.\n\
         You can read and modify files, search code, and run commands on the user's machine.\n\
         Be precise and evidence-driven. Use tools only when the answer requires inspecting the workspace or machine — not for questions answerable from general knowledge.\n\n"
    );

    if let Some(ws) = workspace_path {
        s.push_str(&format!("## Open workspace\nPath: `{ws}`\n\n"));
        let tree = workspace_snapshot(ws);
        if !tree.is_empty() {
            s.push_str("### Workspace file tree (top 2 levels)\n```\n");
            s.push_str(&tree);
            s.push_str("\n```\n\n");
        }
    }

    s.push_str("## Available tools\n\n");

    for tool in tools {
        s.push_str(&format!("### `{}`\n", tool.name));
        s.push_str(&format!("{}\n", tool.description));
        if !tool.args.is_empty() {
            s.push_str("**Arguments:**\n");
            for arg in &tool.args {
                let req = if arg.required { "required" } else { "optional" };
                s.push_str(&format!("- `{}` ({}, {}): {}\n", arg.name, arg.arg_type, req, arg.description));
            }
        }
        s.push('\n');
    }

    let registry: Vec<serde_json::Value> = tools
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "requires_approval": t.requires_approval,
                "is_custom": t.is_custom,
                "args": t.args,
            })
        })
        .collect();
    if let Ok(registry_json) = serde_json::to_string_pretty(&registry) {
        s.push_str("## Tool registry (authoritative JSON)\n```json\n");
        s.push_str(&registry_json);
        s.push_str("\n```\n\n");
    }

    s.push_str(
        "## How to call a tool\n\n\
         Output EXACTLY this format on its own line — nothing before or after:\n\n\
         ```\n\
         <tool_call>{\"tool\": \"tool_name\", \"args\": {\"arg1\": \"value1\"}}</tool_call>\n\
         ```\n\n\
         The result will be returned as:\n\
         ```\n\
         <tool_result>result content here</tool_result>\n\
         ```\n\n\
         **Rules:**\n\
         - Call ONE tool at a time and wait for the result before proceeding.\n\
         - Prefer absolute paths. If path is omitted for directory tools, current workspace path will be used when available.\n\
         - Tools marked with ⚠️ REQUIRES USER APPROVAL will pause for the user to review.\n\
            - After receiving tool results, summarise what you found and continue helping.\n\
            - Do not invent tool names, arguments, or capabilities not present in the tool registry.\n\
            - If a tool returns an error, fix the arguments and retry with a corrected tool call.\n\
         - If you do NOT need any tools, respond normally without any tool_call tags.\n\n"
    );

        s.push_str(
           "## Tool and skill discovery protocol\n\n\
            - If the user asks about tools, skills, capabilities, or what you can do, answer using the tool registry above.\n\
            - Treat each custom tool (is_custom=true) as a workspace skill and include it in capability summaries.\n\
            - For file inventory tasks, never guess from memory; use list_files and report counts from tool results.\n\
            - For requests like 'list all files in current/open directory', use list_all_files. Omit 'path' to default to the current workspace, or pass an absolute path string. Example:\n\
             {\"offset\": 0, \"limit\": 200}\n\
             and keep paging with offset += returned_count while has_more=true.\n\
            - When the user asks for recursive/full tree output, use list_all_files (already recursive).\n\
            - If no workspace is open and the request requires file/directory tools, do not call a tool yet; ask the user to open a folder first.\n\
            - IMPORTANT: Do NOT use file tools for requests that are answerable from general knowledge — language translation, math, science, history, definitions, and similar factual questions. Answer these directly without calling any tool. Only call tools when the answer requires inspecting the actual workspace or machine state.\n\n"
        );

    s.push_str(
        "## Mandatory execution behavior\n\n\
         - When a user asks for factual machine/system information (CPU, RAM, GPU, OS, computer specs), execute a tool call instead of giving manual instructions.\n\
         - Prefer `run_command` with `command: \"specs\"` for this request category.\n\
            - When a user asks to create/build/write a script or file, use `write_file` (and `create_dir` if needed) instead of returning pseudo-code only.\n\
            - For shell script requests, write syntactically valid script content for the requested shell and path.\n\
         - Do not return hypothetical example hardware values.\n\
         - If the user asks to run a command, run it via tool_call and then summarize the real output.\n\n"
    );

    s
}

// ── Tool call parsing ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub tool: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ParsedToolCalls {
    pub calls: Vec<ToolCall>,
    pub malformed_count: usize,
}

fn strip_markdown_fences(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn normalize_tool_call_tags(text: &str) -> String {
    let open_tag = RegexBuilder::new(r"(?is)<\s*tool_call\b[^>]*>")
        .build()
        .expect("valid open tool_call regex");
    let close_tag = RegexBuilder::new(r"(?is)<\s*/\s*tool_call\s*>")
        .build()
        .expect("valid close tool_call regex");

    let normalized_open = open_tag.replace_all(text, "<tool_call>");
    close_tag
        .replace_all(normalized_open.as_ref(), "</tool_call>")
        .to_string()
}

fn find_tag_end(haystack: &str, start_idx: usize) -> Option<usize> {
    haystack[start_idx..].find('>').map(|rel| start_idx + rel)
}

fn parse_json_tool_call(json_str: &str) -> Option<ToolCall> {
    let parsed = serde_json::from_str::<serde_json::Value>(json_str)
        .or_else(|_| serde_json::from_str::<serde_json::Value>(&json_str.replace('\\', "\\\\")));
    let Ok(v) = parsed else { return None; };
    let tool = v["tool"].as_str().unwrap_or("").to_string();
    if tool.is_empty() {
        return None;
    }
    Some(ToolCall {
        tool,
        args: v["args"].clone(),
    })
}

fn looks_like_json_object(s: &str) -> bool {
    let t = s.trim();
    t.starts_with('{') && t.ends_with('}')
}

fn extract_json_objects(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] != b'{' {
            i += 1;
            continue;
        }

        let start = i;
        let mut depth = 0i32;
        let mut in_string = false;
        let mut escaped = false;

        while i < bytes.len() {
            let b = bytes[i];
            if in_string {
                if escaped {
                    escaped = false;
                } else if b == b'\\' {
                    escaped = true;
                } else if b == b'"' {
                    in_string = false;
                }
            } else if b == b'"' {
                in_string = true;
            } else if b == b'{' {
                depth += 1;
            } else if b == b'}' {
                depth -= 1;
                if depth == 0 {
                    let candidate = &text[start..=i];
                    out.push(candidate.to_string());
                    break;
                }
            }
            i += 1;
        }

        i += 1;
    }

    out
}

fn strip_inline_json_tool_calls(text: &str) -> String {
    let mut kept = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            kept.push(String::new());
            continue;
        }

        // Fast-path: drop obvious JSON tool-call emission lines, including truncated ones.
        let lower = trimmed.to_lowercase();
        if lower.starts_with("{\"tool\"") || lower.starts_with("{'tool'") {
            continue;
        }

        // Also drop well-formed JSON objects that parse as tool calls.
        if looks_like_json_object(trimmed) && parse_json_tool_call(trimmed).is_some() {
            continue;
        }

        kept.push(line.to_string());
    }

    // Collapse excessive blank lines introduced by removals.
    let mut compact = Vec::new();
    let mut prev_blank = false;
    for line in kept {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            continue;
        }
        prev_blank = is_blank;
        compact.push(line);
    }
    compact.join("\n").trim().to_string()
}

/// Parse all `<tool_call>...</tool_call>` blocks in a model response.
pub fn parse_tool_calls(response: &str) -> ParsedToolCalls {
    let cleaned = normalize_tool_call_tags(&strip_markdown_fences(response));
    let lower = cleaned.to_lowercase();
    let mut calls = Vec::new();
    let mut malformed_count = 0usize;
    let mut search = 0usize;

    while let Some(rel_open) = lower[search..].find("<tool_call") {
        let open_start = search + rel_open;
        let Some(open_end) = find_tag_end(&lower, open_start) else { break };

        let after_open = open_end + 1;
        let Some(rel_close) = lower[after_open..].find("</tool_call") else { break };
        let close_start = after_open + rel_close;
        let Some(close_end) = find_tag_end(&lower, close_start) else { break };

        let payload = cleaned[after_open..close_start].trim();
        if payload.is_empty() {
            malformed_count += 1;
        } else if looks_like_json_object(payload) {
            if let Some(call) = parse_json_tool_call(payload) {
                calls.push(call);
            } else {
                malformed_count += 1;
            }
        } else {
            malformed_count += 1;
        }

        search = close_end + 1;
    }

    if calls.is_empty() {
        for candidate in extract_json_objects(&cleaned) {
            let candidate_trimmed = candidate.trim();
            if !candidate_trimmed.contains("\"tool\"") {
                continue;
            }
            if let Some(call) = parse_json_tool_call(candidate_trimmed) {
                calls.push(call);
            } else {
                malformed_count += 1;
            }
        }
    }

    ParsedToolCalls {
        calls,
        malformed_count,
    }
}

/// Strip `<tool_call>` blocks from a response (keep only the prose around them).
pub fn strip_tool_calls(response: &str) -> String {
    let cleaned = normalize_tool_call_tags(&strip_markdown_fences(response));
    let lower = cleaned.to_lowercase();
    let mut result = String::new();
    let mut search = 0usize;

    while let Some(rel_open) = lower[search..].find("<tool_call") {
        let open_start = search + rel_open;
        result.push_str(&cleaned[search..open_start]);

        let Some(open_end) = find_tag_end(&lower, open_start) else {
            return result.trim().to_string();
        };
        let after_open = open_end + 1;

        let Some(rel_close) = lower[after_open..].find("</tool_call") else {
            return result.trim().to_string();
        };
        let close_start = after_open + rel_close;
        let Some(close_end) = find_tag_end(&lower, close_start) else {
            return result.trim().to_string();
        };

        search = close_end + 1;
    }

    result.push_str(&cleaned[search..]);
    strip_inline_json_tool_calls(result.trim())
}

// ── Tool execution ────────────────────────────────────────────────────────────

/// Cap tool output at 8 KB so it never blows the model's context window.
fn truncate_output(s: String) -> String {
    const MAX_BYTES: usize = 8 * 1024;
    if s.len() <= MAX_BYTES {
        return s;
    }
    // Walk to a clean char boundary at or just before MAX_BYTES.
    let cutoff = s
        .char_indices()
        .take_while(|(i, _)| *i < MAX_BYTES)
        .last()
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(MAX_BYTES);
    let total_lines = s.lines().count();
    format!(
        "{}\n\n… (output truncated — {} bytes / {} lines total; showing first 8 KB)",
        &s[..cutoff],
        s.len(),
        total_lines,
    )
}

fn resolve_path(raw_path: &str, workspace_path: Option<&str>) -> String {
    let p = Path::new(raw_path);
    if p.is_absolute() {
        raw_path.to_string()
    } else if let Some(ws) = workspace_path {
        PathBuf::from(ws).join(p).to_string_lossy().into_owned()
    } else {
        raw_path.to_string()
    }
}

fn resolve_directory_arg(args: &serde_json::Value, workspace_path: Option<&str>) -> Result<String, String> {
    if let Some(path) = args["path"].as_str() {
        if !path.trim().is_empty() {
            return Ok(resolve_path(path, workspace_path));
        }
    }

    workspace_path
        .map(|ws| ws.to_string())
        .ok_or_else(|| "Missing 'path' arg and no workspace is open".to_string())
}

fn is_specs_request_text(raw: &str) -> bool {
    let lowered = raw.trim().to_lowercase();

    if matches!(
        lowered.as_str(),
        "specs" | "computer specs" | "system specs" | "hardware info" | "system info" | "info"
    ) {
        return true;
    }

    // Accept common natural-language requests so run_command can normalize them.
    (lowered.contains("ram") || lowered.contains("memory"))
        || lowered.contains("how much memory")
        || lowered.contains("how much ram")
        || lowered.contains("system spec")
        || lowered.contains("hardware spec")
        || lowered.contains("computer spec")
        || lowered.contains("cpu")
        || lowered.contains("gpu")
        || lowered.contains("what are my specs")
}

fn normalize_run_command(raw: &str) -> String {
    let trimmed = raw.trim();

    if !is_specs_request_text(trimmed) {
        return trimmed.to_string();
    }

    #[cfg(target_os = "windows")]
    {
        return "powershell -NoProfile -Command \"$os=Get-CimInstance Win32_OperatingSystem; $cpu=Get-CimInstance Win32_Processor | Select-Object -First 1; $cs=Get-CimInstance Win32_ComputerSystem; $gpu=Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name; [PSCustomObject]@{ComputerName=$env:COMPUTERNAME;OS=$os.Caption;OSVersion=$os.Version;CPU=$cpu.Name;Cores=$cpu.NumberOfCores;LogicalProcessors=$cpu.NumberOfLogicalProcessors;RAM_GB=[math]::Round($cs.TotalPhysicalMemory/1GB,2);GPU=($gpu -join '; ')} | ConvertTo-Json -Compress\"".to_string();
    }

    #[cfg(target_os = "macos")]
    {
        return "system_profiler SPHardwareDataType SPSoftwareDataType | head -n 120".to_string();
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return "uname -a; echo; lscpu 2>/dev/null | head -n 40; echo; free -h 2>/dev/null; echo; lspci 2>/dev/null | grep -Ei 'vga|3d|display' | head -n 8".to_string();
    }

    #[allow(unreachable_code)]
    trimmed.to_string()
}

fn is_specs_alias(raw: &str) -> bool {
    is_specs_request_text(raw)
}

/// Execute a built-in tool synchronously (no HITL check — caller is responsible).
pub async fn execute_built_in(
    tool: &str,
    args: &serde_json::Value,
    workspace_path: Option<&str>,
) -> Result<String, String> {
    match tool {
        "read_file" => {
            let raw_path = args["path"].as_str().ok_or("Missing 'path' arg")?;
            let path = resolve_path(raw_path, workspace_path);
            std::fs::read_to_string(path)
                .map(truncate_output)
                .map_err(|e| format!("read_file error: {e}"))
        }

        "list_files" => {
            let path = resolve_directory_arg(args, workspace_path)?;
            let root = std::path::Path::new(&path);
            if !root.exists() {
                return Err(format!("list_files error: path does not exist: {path}"));
            }
            if !root.is_dir() {
                return Err(format!("list_files error: path is not a directory: {path}"));
            }

            let recursive = args["recursive"].as_bool().unwrap_or(false);
            let include_hidden = args["include_hidden"].as_bool().unwrap_or(false);
            let max_depth = args["max_depth"]
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .unwrap_or(8)
                .clamp(1, 32);
            let effective_depth = if recursive { max_depth } else { 1 };
            let offset = args["offset"]
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .unwrap_or(0);
            let limit = args["limit"]
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .unwrap_or(200)
                .clamp(1, 1000);

            let mut entries: Vec<(String, bool)> = walkdir::WalkDir::new(root)
                .min_depth(1)
                .max_depth(effective_depth)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    if include_hidden {
                        return true;
                    }
                    let name = e.file_name().to_string_lossy();
                    !name.starts_with('.')
                })
                .map(|entry| {
                    let rel = entry
                        .path()
                        .strip_prefix(root)
                        .unwrap_or(entry.path())
                        .to_string_lossy()
                        .replace('\\', "/");
                    (rel, entry.file_type().is_dir())
                })
                .collect();

            entries.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

            let total_count = entries.len();
            let page: Vec<serde_json::Value> = entries
                .iter()
                .skip(offset)
                .take(limit)
                .map(|(rel_path, is_dir)| {
                    json!({
                        "path": rel_path,
                        "type": if *is_dir { "dir" } else { "file" },
                    })
                })
                .collect();

            let returned_count = page.len();
            let has_more = offset.saturating_add(returned_count) < total_count;
            let payload = json!({
                "path": path,
                "recursive": recursive,
                "max_depth": effective_depth,
                "offset": offset,
                "limit": limit,
                "total_count": total_count,
                "returned_count": returned_count,
                "has_more": has_more,
                "next_offset": if has_more { json!(offset + returned_count) } else { serde_json::Value::Null },
                "entries": page,
            });

            Ok(payload.to_string())
        }

        "list_all_files" => {
            let path = resolve_directory_arg(args, workspace_path)?;
            let root = std::path::Path::new(&path);
            if !root.exists() {
                return Err(format!("list_all_files error: path does not exist: {path}"));
            }
            if !root.is_dir() {
                return Err(format!("list_all_files error: path is not a directory: {path}"));
            }

            let include_hidden = args["include_hidden"].as_bool().unwrap_or(false);
            let offset = args["offset"]
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .unwrap_or(0);
            let limit = args["limit"]
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .unwrap_or(200)
                .clamp(1, 1000);

            let mut entries: Vec<(String, bool)> = walkdir::WalkDir::new(root)
                .min_depth(1)
                .max_depth(32)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    if include_hidden {
                        return true;
                    }
                    let name = e.file_name().to_string_lossy();
                    !name.starts_with('.')
                })
                .map(|entry| {
                    let rel = entry
                        .path()
                        .strip_prefix(root)
                        .unwrap_or(entry.path())
                        .to_string_lossy()
                        .replace('\\', "/");
                    (rel, entry.file_type().is_dir())
                })
                .collect();

            entries.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

            let total_count = entries.len();
            let page: Vec<serde_json::Value> = entries
                .iter()
                .skip(offset)
                .take(limit)
                .map(|(rel_path, is_dir)| {
                    json!({
                        "path": rel_path,
                        "type": if *is_dir { "dir" } else { "file" },
                    })
                })
                .collect();

            let returned_count = page.len();
            let has_more = offset.saturating_add(returned_count) < total_count;
            let payload = json!({
                "path": path,
                "recursive": true,
                "max_depth": 32,
                "offset": offset,
                "limit": limit,
                "total_count": total_count,
                "returned_count": returned_count,
                "has_more": has_more,
                "next_offset": if has_more { json!(offset + returned_count) } else { serde_json::Value::Null },
                "entries": page,
            });

            Ok(payload.to_string())
        }

        "search_files" => {
            let path  = resolve_directory_arg(args, workspace_path)?;
            let query = args["query"].as_str().ok_or("Missing 'query' arg")?;
            let needle = query.to_lowercase();
            let mut results = Vec::new();

            for entry in walkdir::WalkDir::new(&path)
                .max_depth(8)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| !e.file_type().is_dir())
            {
                // Skip binary-ish files
                let ext = entry.path().extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(ext, "png"|"jpg"|"jpeg"|"gif"|"ico"|"woff"|"woff2"|"ttf"|"eot"|"pdf"|"exe"|"dll"|"so"|"dylib"|"bin"|"gguf") {
                    continue;
                }
                let Ok(content) = std::fs::read_to_string(entry.path()) else { continue };
                for (i, line) in content.lines().enumerate() {
                    if line.to_lowercase().contains(&needle) {
                        results.push(format!(
                            "{}:{}: {}",
                            entry.path().display(), i + 1,
                            line.trim()
                        ));
                        if results.len() >= 60 { break; }
                    }
                }
                if results.len() >= 60 { break; }
            }

            if results.is_empty() {
                Ok(format!("No matches found for '{query}'"))
            } else {
                Ok(truncate_output(format!("{} matches:\n{}", results.len(), results.join("\n"))))
            }
        }

        "grep_files" => {
            let path = resolve_directory_arg(args, workspace_path)?;
            let pattern = args["pattern"].as_str().ok_or("Missing 'pattern' arg")?;
            let case_sensitive = args["case_sensitive"].as_bool().unwrap_or(false);

            let max_results = args["max_results"]
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .unwrap_or(60)
                .clamp(1, 200);

            let regex = RegexBuilder::new(pattern)
                .case_insensitive(!case_sensitive)
                .build()
                .map_err(|e| format!("Invalid regex pattern: {e}"))?;

            let mut results = Vec::new();

            for entry in walkdir::WalkDir::new(&path)
                .max_depth(8)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| !e.file_type().is_dir())
            {
                // Skip binary-ish files.
                let ext = entry.path().extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(ext, "png"|"jpg"|"jpeg"|"gif"|"ico"|"woff"|"woff2"|"ttf"|"eot"|"pdf"|"exe"|"dll"|"so"|"dylib"|"bin"|"gguf") {
                    continue;
                }

                let Ok(content) = std::fs::read_to_string(entry.path()) else { continue };
                for (i, line) in content.lines().enumerate() {
                    if regex.is_match(line) {
                        results.push(format!(
                            "{}:{}: {}",
                            entry.path().display(),
                            i + 1,
                            line.trim()
                        ));
                        if results.len() >= max_results {
                            break;
                        }
                    }
                }
                if results.len() >= max_results {
                    break;
                }
            }

            if results.is_empty() {
                Ok(format!("No matches found for regex '{pattern}'"))
            } else {
                Ok(truncate_output(format!(
                    "{} matches for regex '{}':\n{}",
                    results.len(),
                    pattern,
                    results.join("\n")
                )))
            }
        }

        "write_file" => {
            let path    = args["path"].as_str().ok_or("Missing 'path' arg")?;
            let content = args["content"].as_str().ok_or("Missing 'content' arg")?;
            if let Some(parent) = std::path::Path::new(path).parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(path, content).map_err(|e| format!("write_file error: {e}"))?;
            Ok(format!("✅ Written: {path}"))
        }

        "edit_file" => {
            let path = args["path"].as_str().ok_or("Missing 'path' arg")?;
            let old_string = args["old_string"].as_str().ok_or("Missing 'old_string' arg")?;
            let new_string = args["new_string"].as_str().ok_or("Missing 'new_string' arg")?;

            let content = std::fs::read_to_string(path)
                .map_err(|e| format!("edit_file read error: {e}"))?;

            let matches = content.match_indices(old_string).count();
            if matches == 0 {
                return Err("edit_file error: old_string was not found in file".to_string());
            }
            if matches > 1 {
                return Err("edit_file error: old_string matched multiple locations; provide a more specific string".to_string());
            }

            let updated = content.replacen(old_string, new_string, 1);
            std::fs::write(path, updated).map_err(|e| format!("edit_file write error: {e}"))?;
            Ok(format!("✅ Edited: {path}"))
        }

        "create_dir" => {
            let path = args["path"].as_str().ok_or("Missing 'path' arg")?;
            std::fs::create_dir_all(path).map_err(|e| format!("create_dir error: {e}"))?;
            Ok(format!("✅ Created directory: {path}"))
        }

        "delete_file" => {
            let path = args["path"].as_str().ok_or("Missing 'path' arg")?;
            let p = std::path::Path::new(path);
            if p.is_dir() {
                std::fs::remove_dir_all(path).map_err(|e| format!("delete_file error: {e}"))?;
            } else {
                std::fs::remove_file(path).map_err(|e| format!("delete_file error: {e}"))?;
            }
            Ok(format!("🗑️ Deleted: {path}"))
        }

        "run_command" => {
            let raw = args["command"].as_str().ok_or("Missing 'command' arg")?;
            let cmd = normalize_run_command(raw);
            #[cfg(target_os = "windows")]
            let output = {
                use std::os::windows::process::CommandExt;
                if is_specs_alias(raw) {
                    let specs_script = "$os=Get-CimInstance Win32_OperatingSystem; $cpu=Get-CimInstance Win32_Processor | Select-Object -First 1; $cs=Get-CimInstance Win32_ComputerSystem; $gpu=Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name; [PSCustomObject]@{ComputerName=$env:COMPUTERNAME;OS=$os.Caption;OSVersion=$os.Version;CPU=$cpu.Name;Cores=$cpu.NumberOfCores;LogicalProcessors=$cpu.NumberOfLogicalProcessors;RAM_GB=[math]::Round($cs.TotalPhysicalMemory/1GB,2);GPU=($gpu -join '; ')} | ConvertTo-Json -Compress";
                    let mut c = std::process::Command::new("powershell");
                    c.args(["-NoProfile", "-Command", specs_script]).creation_flags(0x0800_0000);
                    c.output()
                } else {
                    let mut c = std::process::Command::new("cmd");
                    c.args(["/C", &cmd]).creation_flags(0x0800_0000);
                    c.output()
                }
            };
            #[cfg(not(target_os = "windows"))]
            let output = std::process::Command::new("sh").args(["-c", &cmd]).output();

            match output {
                Err(e) => Err(format!("run_command error: {e}")),
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let mut result = stdout.to_string();
                    if !stderr.is_empty() {
                        result.push_str(&format!("\n[stderr]:\n{stderr}"));
                    }
                    if result.trim().is_empty() {
                        result = format!(
                            "Command exited with code {}",
                            out.status.code().unwrap_or(-1)
                        );
                    }
                    if cmd != raw.trim() {
                        result = format!("[normalized command]\n{}\n\n{}", cmd, result);
                    }
                    Ok(truncate_output(result))
                }
            }
        }

        _ => Err(format!("Unknown built-in tool: '{tool}'")),
    }
}

/// Execute a custom script tool. The script receives args as JSON via stdin or first argv.
pub async fn execute_custom(script_path: &str, args: &serde_json::Value) -> Result<String, String> {
    let args_json = serde_json::to_string(args).map_err(|e| e.to_string())?;
    let ext = std::path::Path::new(script_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let output = {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let mut c: std::process::Command = match ext {
                "py"       => { let mut c = std::process::Command::new("python"); c.args([script_path, &args_json]); c },
                "js"       => { let mut c = std::process::Command::new("node"); c.args([script_path, &args_json]); c },
                "ts"       => { let mut c = std::process::Command::new("npx"); c.args(["ts-node", script_path, &args_json]); c },
                "sh"       => { let mut c = std::process::Command::new("sh"); c.args([script_path, &args_json]); c },
                "ps1"      => { let mut c = std::process::Command::new("powershell"); c.args(["-File", script_path, &args_json]); c },
                "rb"       => { let mut c = std::process::Command::new("ruby"); c.args([script_path, &args_json]); c },
                "exe" | "" => { let mut c = std::process::Command::new(script_path); c.arg(&args_json); c },
                _          => return Err(format!("Unsupported script extension: .{ext}")),
            };
            c.creation_flags(0x0800_0000);
            c.output()
        }
        #[cfg(not(windows))]
        match ext {
            "py"       => std::process::Command::new("python").args([script_path, &args_json]).output(),
            "js"       => std::process::Command::new("node").args([script_path, &args_json]).output(),
            "ts"       => std::process::Command::new("npx").args(["ts-node", script_path, &args_json]).output(),
            "sh"       => std::process::Command::new("sh").args([script_path, &args_json]).output(),
            "ps1"      => std::process::Command::new("powershell").args(["-File", script_path, &args_json]).output(),
            "rb"       => std::process::Command::new("ruby").args([script_path, &args_json]).output(),
            "exe" | "" => std::process::Command::new(script_path).arg(&args_json).output(),
            _          => return Err(format!("Unsupported script extension: .{ext}")),
        }
    };

    match output {
        Err(e)  => Err(format!("Script launch failed: {e}")),
        Ok(out) => {
            if !out.status.success() {
                let err = String::from_utf8_lossy(&out.stderr);
                return Err(format!("Script exited non-zero: {err}"));
            }
            Ok(String::from_utf8_lossy(&out.stdout).to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_tool_calls, strip_tool_calls};

    #[test]
    fn parse_tool_calls_accepts_normalized_tag_variants() {
        let response = r#"
prefix
<tool_call class=\"x\">{"tool":"read_file","args":{"path":"README.md"}}</tool_call   >
suffix
"#;

        let parsed = parse_tool_calls(response);
        assert_eq!(parsed.malformed_count, 0);
        assert_eq!(parsed.calls.len(), 1);
        assert_eq!(parsed.calls[0].tool, "read_file");
        assert_eq!(parsed.calls[0].args["path"], "README.md");
    }

    #[test]
    fn parse_tool_calls_counts_malformed_empty_and_non_json_payloads() {
        let response = r#"
<tool_call>   </tool_call>
<tool_call>not json</tool_call>
"#;

        let parsed = parse_tool_calls(response);
        assert_eq!(parsed.calls.len(), 0);
        assert_eq!(parsed.malformed_count, 2);
    }

    #[test]
    fn parse_tool_calls_fallback_extracts_json_object_when_tag_missing() {
        let response = r#"
The tool call is:
{"tool":"list_all_files","args":{"path":"/tmp/work"}}
"#;

        let parsed = parse_tool_calls(response);
        assert_eq!(parsed.calls.len(), 1);
        assert_eq!(parsed.calls[0].tool, "list_all_files");
        assert_eq!(parsed.calls[0].args["path"], "/tmp/work");
    }

    #[test]
    fn parse_tool_calls_handles_fenced_payload() {
        let response = "```xml\n<tool_call>{\"tool\":\"grep_files\",\"args\":{\"path\":\"/repo\",\"pattern\":\"TODO\"}}</tool_call>\n```";
        let parsed = parse_tool_calls(response);
        assert_eq!(parsed.calls.len(), 1);
        assert_eq!(parsed.calls[0].tool, "grep_files");
        assert_eq!(parsed.calls[0].args["pattern"], "TODO");
    }

    #[test]
    fn parse_tool_calls_salvages_unescaped_windows_path() {
        let response = r#"{"tool":"read_file","args":{"path":"Z:\Projects\BonsaiTest\Hello.txt"}}"#;
        let parsed = parse_tool_calls(response);
        assert_eq!(parsed.calls.len(), 1);
        assert_eq!(parsed.calls[0].tool, "read_file");
        assert_eq!(parsed.calls[0].args["path"], "Z:\\Projects\\BonsaiTest\\Hello.txt");
    }

    #[test]
    fn parse_tool_calls_marks_malformed_fallback_json() {
        let response = r#"{"tool":"read_file","args":{"path":"Z:/NotValid",}}"#;
        let parsed = parse_tool_calls(response);
        assert_eq!(parsed.calls.len(), 0);
        assert!(parsed.malformed_count >= 1);
    }

    #[test]
    fn strip_tool_calls_removes_blocks_and_preserves_prose() {
        let response = "Before\n<tool_call>{\"tool\":\"read_file\",\"args\":{\"path\":\"README.md\"}}</tool_call>\nAfter";
        let stripped = strip_tool_calls(response);
        assert!(!stripped.contains("tool_call"));
        assert!(stripped.contains("Before"));
        assert!(stripped.contains("After"));
    }
}
