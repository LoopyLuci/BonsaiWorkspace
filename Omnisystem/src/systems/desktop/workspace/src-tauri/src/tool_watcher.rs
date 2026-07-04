//! Filesystem watcher that invalidates `ToolCache` entries when source files change.
//!
//! Uses the `notify` crate (cross-platform: inotify on Linux, FSEvents on macOS,
//! ReadDirectoryChangesW on Windows). A single watcher instance covers the entire
//! workspace root and invalidates cache entries for all file-reading tools whenever
//! any watched path changes.
//!
//! Tools that are filesystem-sensitive (read_file, code_symbols, grep_files, etc.)
//! benefit automatically — the first read after a file change is always fresh,
//! and subsequent identical reads within the TTL window are still cached.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::tool_cache::ToolCache;

// ── File-sensitive tool names ─────────────────────────────────────────────────
// These tools' cache entries are invalidated when their source files change.
// Immutable tools (get_datetime, system_info, etc.) are NOT listed here.

const FILE_SENSITIVE_TOOLS: &[&str] = &[
    "read_file",
    "list_files",
    "list_all_files",
    "search_files",
    "grep_files",
    "find_files",
    "code_symbols",
    "find_references",
    "get_ast",
    "query_csv",
    "search_knowledge",
    "diff_files",
];

// ── ToolWatcher ───────────────────────────────────────────────────────────────

pub struct ToolWatcher {
    _watcher: RecommendedWatcher, // must be kept alive
    watch_root: PathBuf,
}

impl ToolWatcher {
    /// Start watching `workspace_root` and invalidate `cache` on any file change.
    /// Returns `Err` if the watcher cannot be initialised (e.g. on sandboxed platforms).
    pub fn start(workspace_root: &Path, cache: Arc<ToolCache>) -> Result<Self, String> {
        let (tx, mut rx) = mpsc::channel::<notify::Result<Event>>(128);

        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.blocking_send(res);
        })
        .map_err(|e| format!("[tool_watcher] Failed to create watcher: {e}"))?;

        watcher
            .watch(workspace_root, RecursiveMode::Recursive)
            .map_err(|e| {
                format!(
                    "[tool_watcher] Failed to watch {}: {e}",
                    workspace_root.display()
                )
            })?;

        info!(root=%workspace_root.display(), "[tool_watcher] Watching workspace for cache invalidation");

        // Background task: receive events and invalidate affected cache entries.
        let root = workspace_root.to_path_buf();
        let workspace_str = workspace_root.to_string_lossy().to_string();
        tokio::spawn(async move {
            // Debounce: collect events over 150ms before processing
            let debounce = Duration::from_millis(150);
            let mut pending = false;
            let mut last_event = tokio::time::Instant::now();

            loop {
                tokio::select! {
                    Some(res) = rx.recv() => {
                        match res {
                            Ok(event) => {
                                if should_invalidate(&event) {
                                    pending = true;
                                    last_event = tokio::time::Instant::now();
                                    debug!(kind=?event.kind, paths=?event.paths, "[tool_watcher] file event");
                                }
                            }
                            Err(e) => warn!(error=%e, "[tool_watcher] watch error"),
                        }
                    }
                    _ = tokio::time::sleep(debounce) => {
                        if pending && last_event.elapsed() >= debounce {
                            // Invalidate all file-sensitive tools for this workspace
                            for tool in FILE_SENSITIVE_TOOLS {
                                cache.invalidate_tool(tool);
                            }
                            // Also bust workspace-wide entries (catches list_files, etc.)
                            cache.invalidate_workspace(&workspace_str);
                            debug!("[tool_watcher] cache invalidated after filesystem change");
                            pending = false;
                        }
                    }
                }
            }
        });

        Ok(Self {
            _watcher: watcher,
            watch_root: root,
        })
    }

    pub fn watch_root(&self) -> &Path {
        &self.watch_root
    }

    /// Watch an additional path (e.g. ~/.bonsai/data/).
    pub fn add_path(&mut self, path: &Path) {
        if let Err(e) = self._watcher.watch(path, RecursiveMode::Recursive) {
            warn!(path=%path.display(), error=%e, "[tool_watcher] Failed to add path");
        }
    }
}

/// Decide whether an event should trigger cache invalidation.
/// Ignores metadata-only events (attribute changes, access timestamps)
/// to avoid thrashing on read-heavy workloads.
fn should_invalidate(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_)
            | EventKind::Modify(notify::event::ModifyKind::Data(_))
            | EventKind::Modify(notify::event::ModifyKind::Name(_))
            | EventKind::Remove(_)
    )
}
