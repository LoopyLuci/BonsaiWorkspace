#![allow(dead_code)]
mod app;
mod client;
mod mode;
mod panel;
mod panels;
mod theme;
mod widgets;

use app::CliArgs;
use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() {
    let args = CliArgs::parse();

    // Set up tracing
    let log_path = args.log_file.clone().unwrap_or_else(|| {
        dirs::home_dir()
            .map(|h| h.join(".bonsai").join("tui.log"))
            .unwrap_or_else(|| PathBuf::from("bonsai-tui.log"))
    });

    // Ensure log dir exists
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .ok();

    if let Some(f) = file {
        let subscriber = fmt::Subscriber::builder()
            .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
            .with_writer(std::sync::Mutex::new(f))
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    }

    // Handle stub flags
    if args.headless {
        println!("headless mode not yet implemented");
        return;
    }

    if args.discover {
        println!("discover: LAN daemon discovery not yet implemented");
        return;
    }

    if let Some(cmd) = &args.exec {
        println!("exec: {} (non-interactive mode not yet implemented)", cmd);
        return;
    }

    // Run interactive TUI
    if let Err(e) = app::run(args).await {
        ratatui::restore();
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
