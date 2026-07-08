//! Self-building agent infrastructure — see `agents/self_upgrader.rs` for
//! the `Agent` implementation that drives this, and the module doc
//! comments in `risk.rs`/`sandbox.rs`/`gate.rs` for the tiered-autonomy
//! design (the "hybrid" of full autonomy / sandboxed staging / human
//! review chosen for this feature).

pub mod gate;
pub mod risk;
pub mod sandbox;

use std::path::PathBuf;

/// Walks upward from the current working directory looking for a `.git`
/// directory — the monorepo root, not necessarily `current_dir()` itself
/// (which, depending on how the app was launched, may be `src-tauri` or
/// elsewhere). Falls back to `current_dir()` if none is found.
pub fn find_repo_root() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        if dir.join(".git").exists() {
            return dir;
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => return std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}
