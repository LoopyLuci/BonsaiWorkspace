//! Omnisystem Application Manager CLI
//!
//! The real, full-featured `clap`-based entry point for `app-manager-cli`.
//! Every subcommand below either drives a real `ApplicationManager` /
//! `ConfigManager` operation, or -- for `search`, `logs`, and `verify`,
//! which have no real backing system wired into this crate yet -- reports
//! an honest "not yet implemented" error instead of pretending to succeed.

use app_manager_cli::commands::{
    config_command, health_command, install_command, list_command, logs_command,
    rollback_command, search_command, start_command, status_command, stop_command,
    uninstall_command, update_command, verify_command,
};
use app_manager_cli::output;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "omnisystem-app")]
#[command(about = "Universal Application Manager for Omnisystem", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(global = true, short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Install an application
    Install {
        /// Application ID or GitHub URL
        #[arg(value_name = "APP_ID")]
        app: String,

        /// Version to install
        #[arg(short, long)]
        version: Option<String>,

        /// Installation path
        #[arg(short, long)]
        path: Option<PathBuf>,

        /// Force installation if already installed
        #[arg(short, long)]
        force: bool,
    },

    /// Uninstall an application
    Uninstall {
        /// Application ID
        #[arg(value_name = "APP_ID")]
        app: String,

        /// Force uninstall
        #[arg(short, long)]
        force: bool,
    },

    /// List installed applications
    List {
        /// Filter by state (installed, running, etc.)
        #[arg(short, long)]
        filter: Option<String>,

        /// Output format (table, json)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Show application status
    Status {
        /// Application ID
        #[arg(value_name = "APP_ID")]
        app: String,

        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Start an application
    Start {
        /// Application ID
        #[arg(value_name = "APP_ID")]
        app: String,
    },

    /// Stop an application
    Stop {
        /// Application ID
        #[arg(value_name = "APP_ID")]
        app: String,
    },

    /// Update an application
    Update {
        /// Application ID
        #[arg(value_name = "APP_ID")]
        app: String,

        /// Version to update to
        #[arg(short, long)]
        version: Option<String>,
    },

    /// Configure an application (real, persisted to a JSON file per app id)
    Config {
        /// Application ID
        #[arg(value_name = "APP_ID")]
        app: String,

        /// Configuration key
        #[arg(short, long)]
        set: Option<String>,

        /// Configuration value
        // NB: no `short` here -- `-v` is already claimed by the global
        // `--verbose` flag; clap panics at startup on that collision.
        #[arg(long)]
        value: Option<String>,

        /// Get configuration value
        #[arg(short, long)]
        get: Option<String>,
    },

    /// Search marketplace (not yet implemented)
    Search {
        /// Search query
        #[arg(value_name = "QUERY")]
        query: String,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Show application logs (not yet implemented)
    Logs {
        /// Application ID
        #[arg(value_name = "APP_ID")]
        app: String,

        /// Number of lines
        #[arg(short, long, default_value = "50")]
        lines: usize,

        /// Follow log output
        #[arg(short, long)]
        follow: bool,
    },

    /// Verify application integrity (not yet implemented)
    Verify {
        /// Application ID
        #[arg(value_name = "APP_ID")]
        app: String,
    },

    /// Rollback to previous version (real, via Installer::rollback)
    Rollback {
        /// Application ID
        #[arg(value_name = "APP_ID")]
        app: String,
    },

    /// System health check
    Health,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    info!("Omnisystem Application Manager CLI");

    let result = match cli.command {
        Commands::Install { app, version, path, force } => {
            install_command(&app, version, path, force).await
        }
        Commands::Uninstall { app, force } => uninstall_command(&app, force).await,
        Commands::List { filter, format } => list_command(filter, &format).await,
        Commands::Status { app, detailed } => status_command(&app, detailed).await,
        Commands::Start { app } => start_command(&app).await,
        Commands::Stop { app } => stop_command(&app).await,
        Commands::Update { app, version } => update_command(&app, version).await,
        Commands::Config { app, set, value, get } => config_command(&app, set, value, get).await,
        Commands::Search { query, limit } => search_command(&query, limit).await,
        Commands::Logs { app, lines, follow } => logs_command(&app, lines, follow).await,
        Commands::Verify { app } => verify_command(&app).await,
        Commands::Rollback { app } => rollback_command(&app).await,
        Commands::Health => health_command().await,
    };

    match result {
        Ok(_) => {
            info!("Command completed successfully");
            std::process::exit(0);
        }
        Err(e) => {
            error!("Command failed: {}", e);
            output::print_error(&format!("Error: {}", e));
            std::process::exit(1);
        }
    }
}
