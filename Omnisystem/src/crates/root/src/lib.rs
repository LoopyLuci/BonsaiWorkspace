//! `root` (a.k.a. "Bonsai Root"): the installer/launcher engine behind the
//! Bonsai Ecosystem desktop app. Verifies a signed component manifest,
//! builds a dependency-ordered install plan, and executes it as an atomic,
//! rollback-capable transaction (download -> hash-verify -> extract ->
//! commit). Driven by a small state machine that models the installer's
//! screens/phases. The Tauri desktop shell (`src-tauri/`) wraps this crate's
//! public API as IPC commands.

pub mod installer;
pub mod manifest;
pub mod planner;
pub mod state_machine;
pub mod utils;

pub use installer::transaction::Transaction;
pub use installer::{ensure_install_root, install_path};
pub use manifest::{Component, Manifest};
pub use planner::{build_install_plan, InstallPlan};
pub use state_machine::{InstallerMode, StateMachine};
