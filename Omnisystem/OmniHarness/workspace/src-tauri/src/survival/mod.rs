//! The Survival System — the one system in Omnisystem responsible for
//! finding and fixing bugs across the codebase, on purpose singular: this
//! module absorbs what used to be two separately-named things (the
//! original `survival.rs` shell-script KB, and a short-lived separate
//! `bug_watch` module) into one namespace with one feature flag and one UI
//! panel, using several tools:
//!
//! - `kb` — fast, synchronous shell-script mitigation for known runtime
//!   process failure signatures (the original `survival.rs`).
//! - `bug_db` — the durable, deduplicated catalog every other tool here
//!   feeds into.
//! - `bug_hunter` — proactive scanning (compile/test/lint) across every
//!   `targets::MonitorTarget`.
//! - `sns_bridge` — pulls in fuzzing-discovered crashes (`failure-finder`/F³)
//!   and capability-sandbox violations (`sns`) as additional bug sources.
//! - `crash_ingest` — folds already-captured backend panics and frontend
//!   JS errors (`crash_reporter`) in as well.
//! - `targets` — the registry of what's actually real and checkable today
//!   (see its doc comment for why "the entire monorepo" isn't literally
//!   every crate yet).
//! - `daemon` — the background loop tying all of the above together, and
//!   the one place that decides whether an open bug gets escalated to the
//!   `self_upgrade` code-level fix pipeline.

pub mod bug_db;
pub mod bug_hunter;
pub mod crash_ingest;
pub mod daemon;
pub mod kb;
pub mod sns_bridge;
pub mod targets;
