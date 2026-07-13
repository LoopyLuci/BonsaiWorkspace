use clap::{Parser, Subcommand};
use tracing::{info, error};
use std::path::PathBuf;

mod commands;
mod output;

use commands::{
    install_command, uninstall_command, list_command, status_command,
    start_command, stop_command, update_command, config_command,
};

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

    /// Configure an application
    Config {
        /// Application ID
        #[arg(value_name = "APP_ID")]
        app: String,

        /// Configuration key
        #[arg(short, long)]
        set: Option<String>,

        /// Configuration value
        #[arg(short, long)]
        value: Option<String>,

        /// Get configuration value
        #[arg(short, long)]
        get: Option<String>,
    },

    /// Search marketplace
    Search {
        /// Search query
        #[arg(value_name = "QUERY")]
        query: String,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Show application logs
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

    /// Verify application integrity
    Verify {
        /// Application ID
        #[arg(value_name = "APP_ID")]
        app: String,
    },

    /// Rollback to previous version
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
        Commands::Uninstall { app, force } => {
            uninstall_command(&app, force).await
        }
        Commands::List { filter, format } => {
            list_command(filter, &format).await
        }
        Commands::Status { app, detailed } => {
            status_command(&app, detailed).await
        }
        Commands::Start { app } => {
            start_command(&app).await
        }
        Commands::Stop { app } => {
            stop_command(&app).await
        }
        Commands::Update { app, version } => {
            update_command(&app, version).await
        }
        Commands::Config { app, set, value, get } => {
            config_command(&app, set, value, get).await
        }
        Commands::Search { query, limit: _ } => {
            output::print_message(&format!("Searching for: {}", query));
            Ok(())
        }
        Commands::Logs { app, lines, follow } => {
            output::print_message(&format!("Fetching {} lines of logs for {}", lines, app));
            if follow {
                output::print_message("Following logs (press Ctrl+C to stop)");
            }
            Ok(())
        }
        Commands::Verify { app } => {
            output::print_message(&format!("Verifying {}", app));
            Ok(())
        }
        Commands::Rollback { app } => {
            output::print_message(&format!("Rolling back {}", app));
            Ok(())
        }
        Commands::Health => {
            output::print_message("Checking system health...");
            Ok(())
        }
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
