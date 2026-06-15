//! BONSAI.md — the self-evolving system prompt.
//!
//! Loaded from the project root (`<workspace>/BONSAI.md`) and a global
//! `~/.bonsai/global-bonsai.md` override.  Both are injected at the top of
//! every chat system prompt so the model always has the latest project context.
//!
//! The EternalWorkshop daemon rewrites `BONSAI.md` nightly after each memory
//! consolidation cycle.

use std::path::{Path, PathBuf};

const GLOBAL_BONSAI_MD_PATH: &str = ".bonsai/global-bonsai.md";

/// Default content written when no BONSAI.md exists in a project.
const DEFAULT_BONSAI_MD: &str = r#"# BONSAI.md — Project Context

> This file is automatically maintained by BonsAI.
> Edit freely — it is re-injected into every conversation.

## Role
You are BonsAI, the built-in AI assistant of Bonsai Workspace.
You are precise, concise, and safety-conscious.
You always prefer the simplest correct solution.

## Coding Conventions
- Rust: `snake_case`, no `unwrap()` in non-test code, prefer `?` propagation.
- TypeScript/Svelte: functional style, typed props, no `any`.
- Python: type hints, f-strings, `pathlib` over `os.path`.
- All new files must have a one-line module doc comment.

## Active Context
*(Updated nightly by the EternalWorkshop daemon)*
"#;

/// Load BONSAI.md for a given project workspace path.
/// Returns global override + project-level content, separated by a blank line.
/// Returns an empty string if neither file exists (no injection, no error).
pub fn load(workspace_path: Option<&str>) -> String {
    let mut parts: Vec<String> = Vec::new();

    // 1. Global override (~/.bonsai/global-bonsai.md)
    if let Some(home) = dirs::home_dir() {
        let global = home.join(GLOBAL_BONSAI_MD_PATH);
        if let Ok(content) = std::fs::read_to_string(&global) {
            if !content.trim().is_empty() {
                parts.push(content.trim().to_string());
            }
        }
    }

    // 2. Project-level BONSAI.md
    if let Some(ws) = workspace_path {
        let project = Path::new(ws).join("BONSAI.md");
        if let Ok(content) = std::fs::read_to_string(&project) {
            if !content.trim().is_empty() {
                parts.push(content.trim().to_string());
            }
        }
    }

    if parts.is_empty() {
        return String::new();
    }
    parts.join("\n\n")
}

/// Prepend BONSAI.md content to an existing system prompt.
/// If the prompt already contains the BONSAI.md marker, does nothing (idempotent).
pub fn inject(system_prompt: &str, workspace_path: Option<&str>) -> String {
    let md = load(workspace_path);
    if md.is_empty() || system_prompt.contains("BONSAI.md") {
        return system_prompt.to_string();
    }
    format!("{md}\n\n---\n\n{system_prompt}")
}

/// Write a new BONSAI.md to the project root.  Called by the EternalWorkshop
/// daemon after each memory consolidation cycle.
pub fn write(workspace_path: &str, content: &str) -> std::io::Result<()> {
    let path = Path::new(workspace_path).join("BONSAI.md");
    std::fs::write(path, content)
}

/// Ensure a BONSAI.md exists for a project.  If it doesn't, write the default.
/// Safe to call on every project open.
pub fn ensure_exists(workspace_path: &str) {
    let path = Path::new(workspace_path).join("BONSAI.md");
    if !path.exists() {
        let _ = std::fs::write(&path, DEFAULT_BONSAI_MD);
    }
}

/// Append a "Today's Learnings" section to an existing BONSAI.md.
/// Called by the daemon after each consolidation.
pub fn append_learnings(workspace_path: &str, learnings: &str) -> std::io::Result<()> {
    let path = Path::new(workspace_path).join("BONSAI.md");
    let existing = std::fs::read_to_string(&path).unwrap_or_default();

    // Replace the "Active Context" block if it exists, otherwise append.
    const MARKER: &str = "## Active Context";
    let new_content = if let Some(pos) = existing.find(MARKER) {
        format!(
            "{}{MARKER}\n*(Updated: {})*\n\n{learnings}\n",
            &existing[..pos],
            chrono::Local::now().format("%Y-%m-%d %H:%M"),
        )
    } else {
        format!(
            "{existing}\n\n{MARKER}\n*(Updated: {})*\n\n{learnings}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M"),
        )
    };

    std::fs::write(path, new_content)
}
