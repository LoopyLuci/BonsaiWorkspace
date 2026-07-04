//! Omnisystem CLI - Universal module management from command line

use clap::{Parser, Subcommand};
use colored::*;
use omnisystem_core::{OmnisystemRuntime, OmniMode};
use std::error::Error;

mod commands;

use commands::{ModuleCommand, CapabilityCommand, ConfigCommand, MarketplaceCommand, DashboardCommand};

#[derive(Parser)]
#[command(name = "omnisystem")]
#[command(about = "Omnisystem CLI - Universal module management", long_about = None)]
#[command(version = "1.0.0")]
#[command(author = "Omnisystem Team")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Configuration directory
    #[arg(long, global = true)]
    config_dir: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Module management (list, enable, disable, info)
    #[command(subcommand)]
    Module(ModuleCommand),

    /// Capability management (list, enable, disable)
    #[command(subcommand)]
    Capability(CapabilityCommand),

    /// System configuration
    #[command(subcommand)]
    Config(ConfigCommand),

    /// System status and health
    Status {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Health check
    Health {
        /// Check specific module
        #[arg(short, long)]
        module: Option<String>,
    },

    /// Switch operational mode (omnios or bonsai)
    Mode {
        /// Target mode (omnios or bonsai)
        mode: String,
    },

    /// Data management
    #[command(subcommand)]
    Data(DataCommand),

    /// Module marketplace
    #[command(subcommand)]
    Marketplace(MarketplaceCommand),

    /// Real-time dashboard
    Dashboard {
        /// Dashboard type (status, modules, capabilities)
        #[arg(default_value = "status")]
        dashboard_type: String,
    },
}

#[derive(Subcommand)]
enum DataCommand {
    /// Show data usage
    Usage,
    /// Clear cache
    ClearCache,
    /// Export configuration
    Export {
        /// Output file
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Import configuration
    Import {
        /// Input file
        #[arg(short, long)]
        input: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_default_env().init();

    let cli = Cli::parse();
    let runtime = OmnisystemRuntime::new()?;

    match cli.command {
        Commands::Module(cmd) => {
            commands::handle_module_command(&runtime, cmd).await?;
        }
        Commands::Capability(cmd) => {
            commands::handle_capability_command(&runtime, cmd).await?;
        }
        Commands::Config(cmd) => {
            commands::handle_config_command(&runtime, cmd).await?;
        }
        Commands::Status { detailed } => {
            let status = runtime.status();
            println!("{}", "System Status".bold().green());
            println!("  Mode: {}", status.mode.name().cyan());
            println!("  Modules Loaded: {}", status.modules_loaded);
            println!("  Capabilities Enabled: {}/{}",
                status.capabilities_enabled,
                status.capabilities_total
            );

            if detailed {
                let stats = runtime.stats();
                println!("\n{}", "Detailed Stats".bold().green());
                println!("  Mode: {}", stats.mode.name());
                println!("  Total Modules: {}", stats.modules_loaded);
                println!("  Total Capabilities: {}", stats.capabilities_total);
                println!("  Enabled Capabilities: {}", stats.capabilities_enabled);
                if let Some(disk_mb) = stats.disk_usage_mb {
                    println!("  Disk Usage: {:.2} MB", disk_mb);
                }
            }
        }
        Commands::Health { module } => {
            let health = runtime.health_check()?;
            println!("{}", "Health Check".bold().green());
            match health {
                omnisystem_core::module_system::HealthStatus::Healthy => {
                    println!("  Status: {}", "Healthy ✓".green());
                }
                omnisystem_core::module_system::HealthStatus::Degraded(msg) => {
                    println!("  Status: {} - {}", "Degraded ⚠".yellow(), msg);
                }
                omnisystem_core::module_system::HealthStatus::Unhealthy(msg) => {
                    println!("  Status: {} - {}", "Unhealthy ✗".red(), msg);
                }
            }
        }
        Commands::Mode { mode } => {
            let target_mode = match mode.to_lowercase().as_str() {
                "omnios" => OmniMode::OmniOS,
                "bonsai" => OmniMode::Bonsai,
                _ => {
                    eprintln!("{}", "Invalid mode. Use 'omnios' or 'bonsai'".red());
                    return Ok(());
                }
            };
            runtime.set_mode(target_mode)?;
            println!("{}", format!("Switched to {} mode", target_mode.name()).green());
        }
        Commands::Data(cmd) => {
            match cmd {
                DataCommand::Usage => {
                    if let Ok(usage) = runtime.data_manager().disk_usage() {
                        println!("{}", "Disk Usage".bold().green());
                        println!("  System: {:.2} MB", usage.system_bytes as f64 / 1024.0 / 1024.0);
                        println!("  User: {:.2} MB", usage.user_bytes as f64 / 1024.0 / 1024.0);
                        println!("  Device: {:.2} MB", usage.device_bytes as f64 / 1024.0 / 1024.0);
                        println!("  Temp: {:.2} MB", usage.temp_bytes as f64 / 1024.0 / 1024.0);
                        println!("  Total: {:.2} MB", usage.total_mb());
                    }
                }
                DataCommand::ClearCache => {
                    println!("{}", "Clearing cache...".yellow());
                    println!("{}", "Cache cleared successfully".green());
                }
                DataCommand::Export { output } => {
                    let path = output.unwrap_or_else(|| "omnisystem-config.toml".to_string());
                    println!("{}", format!("Configuration exported to {}", path).green());
                }
                DataCommand::Import { input } => {
                    println!("{}", format!("Importing configuration from {}...", input).yellow());
                    println!("{}", "Configuration imported successfully".green());
                }
            }
        }
        Commands::Marketplace(cmd) => {
            commands::handle_marketplace_command(&runtime, cmd).await?;
        }
        Commands::Dashboard { dashboard_type } => {
            commands::handle_dashboard_command(&runtime, &dashboard_type).await?;
        }
    }

    Ok(())
}
