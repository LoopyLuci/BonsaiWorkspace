//! Survival System — the monitored-target registry.
//!
//! One of the Survival System's tools (see `super` for the others: `kb`,
//! `bug_db`, `sns_bridge`, `crash_ingest`, `daemon`). Scanning "the entire
//! monorepo" in one pass isn't practical: the root `Cargo.toml` workspace
//! covers ~2,400 crates, most of which are dead-in-place pre-purge
//! scaffolding with broken intra-workspace path dependencies (confirmed:
//! 2,392 of 2,400 crates under `Omnisystem/src/crates/*` reference at least
//! one path dependency that doesn't resolve — this is far beyond a simple
//! rename-mismatch fix, so that tree is deliberately excluded from the
//! registry below until it's repaired crate-by-crate, which is out of scope
//! for this pass). This registry instead lists every target that is
//! *actually* real and checkable today — confirmed either by carrying its
//! own `[workspace]` table (Rust) or by having live `node_modules` + CI
//! coverage (JS/TS) — and `daemon.rs` round-robins across them one per
//! cycle rather than scanning everything every cycle.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    /// A crate with its own `[workspace]` table — `cargo check`/`clippy`/
    /// `test --lib` all work standalone from its own directory.
    RustCrate,
    /// A Svelte project checked via `svelte-check --output machine`
    /// (this app's own frontend pattern).
    NpmSvelteCheck,
    /// A plain TS project checked via `tsc --noEmit` + `eslint --format json`
    /// (only used where both are actually configured — confirmed via its
    /// `package.json`, not assumed).
    NpmTypecheckLint,
}

#[derive(Debug, Clone)]
pub struct MonitorTarget {
    pub name: &'static str,
    /// Path relative to the monorepo root (`self_upgrade::find_repo_root()`).
    pub rel_path: &'static str,
    pub kind: TargetKind,
}

impl MonitorTarget {
    pub fn abs_path(&self, repo_root: &Path) -> PathBuf {
        repo_root.join(self.rel_path)
    }
}

/// The CI-verified-real + self-contained-`[workspace]` targets found this
/// session. Every entry here was individually confirmed to exist and be
/// independently buildable — see the plan doc for the research that
/// produced this exact list (`.github/workflows/*.yml` for the JS/TS
/// entries, each crate's own `[workspace]` table for the Rust entries).
pub fn default_registry() -> Vec<MonitorTarget> {
    vec![
        // This app itself.
        MonitorTarget { name: "workspace-backend", rel_path: "Omnisystem/OmniHarness/workspace/src-tauri", kind: TargetKind::RustCrate },
        MonitorTarget { name: "workspace-frontend", rel_path: "Omnisystem/OmniHarness/workspace/src", kind: TargetKind::NpmSvelteCheck },
        // CI-verified real, newly added.
        MonitorTarget { name: "kernel", rel_path: "Omnisystem/OmniHarness/kernel", kind: TargetKind::RustCrate },
        MonitorTarget { name: "vscode-omnisystem", rel_path: "Omnisystem/vscode-omnisystem", kind: TargetKind::NpmTypecheckLint },
        // The other 18 self-contained (own `[workspace]`) Rust crates.
        MonitorTarget { name: "bootstrap-rs", rel_path: "Omnisystem/bootstrap-rs", kind: TargetKind::RustCrate },
        MonitorTarget { name: "bootstrap-aether-rs", rel_path: "Omnisystem/bootstrap-aether-rs", kind: TargetKind::RustCrate },
        MonitorTarget { name: "bootstrap-axiom-rs", rel_path: "Omnisystem/bootstrap-axiom-rs", kind: TargetKind::RustCrate },
        MonitorTarget { name: "bootstrap-helix-rs", rel_path: "Omnisystem/bootstrap-helix-rs", kind: TargetKind::RustCrate },
        MonitorTarget { name: "bootstrap-nexus-rs", rel_path: "Omnisystem/bootstrap-nexus-rs", kind: TargetKind::RustCrate },
        MonitorTarget { name: "bootstrap-sylva-rs", rel_path: "Omnisystem/bootstrap-sylva-rs", kind: TargetKind::RustCrate },
        MonitorTarget { name: "bootstrap-vera-rs", rel_path: "Omnisystem/bootstrap-vera-rs", kind: TargetKind::RustCrate },
        MonitorTarget { name: "mcp-server", rel_path: "Omnisystem/OmniHarness/crates/mcp-server", kind: TargetKind::RustCrate },
        MonitorTarget { name: "compiler-aether", rel_path: "Omnisystem/src/compiler/aether", kind: TargetKind::RustCrate },
        MonitorTarget { name: "compiler-axiom", rel_path: "Omnisystem/src/compiler/axiom", kind: TargetKind::RustCrate },
        MonitorTarget { name: "compiler-sylva", rel_path: "Omnisystem/src/compiler/sylva", kind: TargetKind::RustCrate },
        MonitorTarget { name: "compiler-titan", rel_path: "Omnisystem/src/compiler/titan", kind: TargetKind::RustCrate },
        MonitorTarget { name: "omnisystem-cli", rel_path: "Omnisystem/src/tools/omnisystem-cli", kind: TargetKind::RustCrate },
        MonitorTarget { name: "ucc", rel_path: "Omnisystem/src/systems/ucc", kind: TargetKind::RustCrate },
        MonitorTarget { name: "polyglot-pong", rel_path: "Omnisystem/src/testing/polyglot-pong/polyglot-pong", kind: TargetKind::RustCrate },
        MonitorTarget { name: "omnisystem-launcher-gui", rel_path: "Omnisystem/src/crates/omnisystem-launcher-gui/src-tauri", kind: TargetKind::RustCrate },
        MonitorTarget { name: "ui-widgets-tauri", rel_path: "Omnisystem/src/crates/ui-widgets/tauri/src-tauri", kind: TargetKind::RustCrate },
        // NOTE: `bonsai-desktop-environment` deliberately excluded — confirmed
        // orphaned duplicate of the live `omnisystem-desktop-environment`.
        // NOTE: no "root-workspace" (`Omnisystem/src/crates/*`, ~2,400 crates)
        // entry — see module doc comment for why.
    ]
}

/// Round-robins one target per call rather than scanning everything every
/// cycle — with ~20 targets and a 20-minute cycle, a full rotation takes
/// roughly 6-7 hours, which is fine for a low-priority background daemon.
pub struct TargetScheduler {
    targets: Vec<MonitorTarget>,
    next_index: usize,
}

impl TargetScheduler {
    pub fn new(targets: Vec<MonitorTarget>) -> Self {
        Self { targets, next_index: 0 }
    }

    /// Returns `None` only if the registry is empty.
    pub fn next(&mut self) -> Option<&MonitorTarget> {
        if self.targets.is_empty() {
            return None;
        }
        let target = &self.targets[self.next_index];
        self.next_index = (self.next_index + 1) % self.targets.len();
        Some(target)
    }

    pub fn len(&self) -> usize {
        self.targets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_targets() -> Vec<MonitorTarget> {
        vec![
            MonitorTarget { name: "a", rel_path: "a", kind: TargetKind::RustCrate },
            MonitorTarget { name: "b", rel_path: "b", kind: TargetKind::RustCrate },
        ]
    }

    #[test]
    fn advances_one_target_per_call() {
        let mut sched = TargetScheduler::new(two_targets());
        assert_eq!(sched.next().unwrap().name, "a");
        assert_eq!(sched.next().unwrap().name, "b");
    }

    #[test]
    fn wraps_around_after_the_last_target() {
        let mut sched = TargetScheduler::new(two_targets());
        sched.next();
        sched.next();
        assert_eq!(sched.next().unwrap().name, "a", "must wrap back to the first target");
    }

    #[test]
    fn empty_registry_returns_none() {
        let mut sched = TargetScheduler::new(vec![]);
        assert!(sched.next().is_none());
    }

    #[test]
    fn default_registry_is_non_empty_and_has_no_duplicate_names() {
        let registry = default_registry();
        assert!(!registry.is_empty());
        let mut names: Vec<&str> = registry.iter().map(|t| t.name).collect();
        names.sort_unstable();
        let mut deduped = names.clone();
        deduped.dedup();
        assert_eq!(names.len(), deduped.len(), "target names must be unique");
    }
}
