//! App Manager CLI
//!
//! Command implementations and terminal output helpers for the Omnisystem
//! application-manager command-line tool. The `clap`-based entry point
//! itself lives in `src/bin/cli.rs`, built as the `app-manager-cli` binary;
//! this library crate exists so `commands` and `output` can be exercised by
//! `cargo test -p app-manager-cli --lib` directly.

pub mod commands;
pub mod output;
