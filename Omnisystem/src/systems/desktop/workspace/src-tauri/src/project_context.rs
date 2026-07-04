//! OMNISYSTEM.md — the self-evolving system prompt.
//! (Renamed from BONSAI.md; same convention, same auto-update behavior.)
//!
//! Loaded from the project root (`<workspace>/OMNISYSTEM.md`) and a global
//! `~/.omnisystem/global-context.md` override.  Both are injected at the top of
//! every chat system prompt so the model always has the latest project context.
//!
//! The EternalWorkshop daemon rewrites `OMNISYSTEM.md` nightly after each memory
//! consolidation cycle. This is the same idea as a CLAUDE.md/AGENTS.md project
//! instructions file, except it is auto-maintained rather than hand-edited —
//! a natural complement to the auto-compaction system in the OmniHarness VS
//! Code panel (that summarizes conversation history; this summarizes durable
//! project learnings).

use std::path::{Path, PathBuf};

const GLOBAL_CONTEXT_MD_PATH: &str = ".omnisystem/global-context.md";

/// Default content written when no OMNISYSTEM.md exists in a project.
const DEFAULT_CONTEXT_MD: &str = r#"# OMNISYSTEM.md — Project Context

> This file is automatically maintained by the Omnisystem assistant.
> Edit freely — it is re-injected into every conversation.

## Role
You are the built-in AI assistant of Omnisystem.
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

/// Load OMNISYSTEM.md for a given project workspace path.
/// Returns global override + project-level content, separated by a blank line.
/// Returns an empty string if neither file exists (no injection, no error).
pub fn load(workspace_path: Option<&str>) -> String {
    let mut parts: Vec<String> = Vec::new();

    // 1. Global override (~/.omnisystem/global-context.md)
    if let Some(home) = dirs::home_dir() {
        let global = home.join(GLOBAL_CONTEXT_MD_PATH);
        if let Ok(content) = std::fs::read_to_string(&global) {
            if !content.trim().is_empty() {
                parts.push(content.trim().to_string());
            }
        }
    }

    // 2. Project-level OMNISYSTEM.md
    if let Some(ws) = workspace_path {
        let project = Path::new(ws).join("OMNISYSTEM.md");
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

/// Prepend OMNISYSTEM.md content to an existing system prompt.
/// If the prompt already contains the OMNISYSTEM.md marker, does nothing (idempotent).
pub fn inject(system_prompt: &str, workspace_path: Option<&str>) -> String {
    let md = load(workspace_path);
    if md.is_empty() || system_prompt.contains("OMNISYSTEM.md") {
        return system_prompt.to_string();
    }
    format!("{md}\n\n---\n\n{system_prompt}")
}

/// Write a new OMNISYSTEM.md to the project root.  Called by the EternalWorkshop
/// daemon after each memory consolidation cycle.
pub fn write(workspace_path: &str, content: &str) -> std::io::Result<()> {
    let path = Path::new(workspace_path).join("OMNISYSTEM.md");
    std::fs::write(path, content)
}

/// Ensure an OMNISYSTEM.md exists for a project.  If it doesn't, write the default.
/// Safe to call on every project open.
pub fn ensure_exists(workspace_path: &str) {
    let path = Path::new(workspace_path).join("OMNISYSTEM.md");
    if !path.exists() {
        let _ = std::fs::write(&path, DEFAULT_CONTEXT_MD);
    }
}

/// Append a "Today's Learnings" section to an existing OMNISYSTEM.md.
/// Called by the daemon after each consolidation.
pub fn append_learnings(workspace_path: &str, learnings: &str) -> std::io::Result<()> {
    let path = Path::new(workspace_path).join("OMNISYSTEM.md");
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
