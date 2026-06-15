#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(
    clippy::too_many_arguments,
    clippy::useless_format,
    clippy::needless_borrow
)]

use clap::Parser;

/// Bonsai Workspace — local-first AI development environment
#[derive(Parser, Debug)]
#[command(name = "bonsai", version, about)]
struct Cli {
    /// Launch mode: workspace (IDE only), buddy (chat window only), ecosystem (both)
    #[arg(long, default_value = "workspace")]
    mode: String,
}

fn main() {
    let cli = Cli::parse();
    // Publish the mode so lib.rs setup can read it without threading it through run().
    // SAFETY: we set this before any threads spawn, so there is no race.
    unsafe { std::env::set_var("BONSAI_LAUNCH_MODE", &cli.mode) };
    workspace_lib::run();
}
