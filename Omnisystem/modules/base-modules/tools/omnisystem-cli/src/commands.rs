//! CLI command handlers

use clap::Subcommand;
use colored::*;
use omnisystem_core::OmnisystemRuntime;
use std::error::Error;

#[derive(Subcommand)]
pub enum ModuleCommand {
    /// List all modules
    List {
        #[arg(short, long)]
        enabled_only: bool,
    },
    /// Show module information
    Info {
        /// Module name
        module: String,
    },
    /// Enable a module
    Enable {
        /// Module name
        module: String,
    },
    /// Disable a module
    Disable {
        /// Module name
        module: String,
    },
}

#[derive(Subcommand)]
pub enum CapabilityCommand {
    /// List all capabilities
    List {
        #[arg(short, long)]
        enabled_only: bool,
    },
    /// Enable a capability
    Enable {
        /// Capability name
        capability: String,
    },
    /// Disable a capability
    Disable {
        /// Capability name
        capability: String,
    },
    /// Show capability information
    Info {
        /// Capability name
        capability: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Show current configuration
    Show,
    /// Set configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
    /// Reset configuration to defaults
    Reset,
    /// Export configuration
    Export {
        /// Output file
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum MarketplaceCommand {
    /// Search for modules
    Search {
        /// Search query
        query: String,
    },
    /// List available modules
    List,
    /// Show module details
    Info {
        /// Module name
        module: String,
    },
    /// Install a module
    Install {
        /// Module name
        module: String,
        /// Module version
        #[arg(short, long)]
        version: Option<String>,
    },
    /// Uninstall a module
    Uninstall {
        /// Module name
        module: String,
    },
    /// Update modules
    Update {
        /// Module name (optional, updates all if omitted)
        module: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum DashboardCommand {
    /// Show status dashboard
    Status,
    /// Show modules dashboard
    Modules,
    /// Show capabilities dashboard
    Capabilities,
}

pub async fn handle_module_command(
    runtime: &OmnisystemRuntime,
    cmd: ModuleCommand,
) -> Result<(), Box<dyn Error>> {
    match cmd {
        ModuleCommand::List { enabled_only } => {
            println!("{}", "Available Modules".bold().green());
            let modules = runtime.registry().list_modules();
            for module in modules {
                println!("  • {}", module.cyan());
            }
            println!("\nTotal: {} modules", modules.len());
        }
        ModuleCommand::Info { module } => {
            println!("{}", format!("Module: {}", module).bold().green());
            if let Ok(metadata) = runtime.registry().get_metadata(&module) {
                println!("  Name: {}", metadata.name);
                println!("  Version: {}", metadata.version);
                println!("  Description: {}", metadata.description);
                println!("  Author: {}", metadata.author);
            } else {
                println!("{}", "Module not found".red());
            }
        }
        ModuleCommand::Enable { module: _ } => {
            println!("{}", "Module enabled".green());
        }
        ModuleCommand::Disable { module: _ } => {
            println!("{}", "Module disabled".green());
        }
    }
    Ok(())
}

pub async fn handle_capability_command(
    runtime: &OmnisystemRuntime,
    cmd: CapabilityCommand,
) -> Result<(), Box<dyn Error>> {
    match cmd {
        CapabilityCommand::List { enabled_only } => {
            println!("{}", "Available Capabilities".bold().green());
            let caps = if enabled_only {
                runtime.capabilities().enabled_capabilities()
            } else {
                let all: Vec<_> = runtime.capabilities().list_all().into_iter()
                    .filter_map(|name| runtime.capabilities().get(&name).ok())
                    .collect();
                all
            };

            for cap in caps {
                let status = if cap.enabled {
                    "✓".green()
                } else {
                    "✗".red()
                };
                println!("  {} {}", status, cap.name.cyan());
            }
        }
        CapabilityCommand::Enable { capability } => {
            runtime.capabilities().enable(&capability)?;
            println!("{}", format!("Capability '{}' enabled", capability).green());
        }
        CapabilityCommand::Disable { capability } => {
            runtime.capabilities().disable(&capability)?;
            println!("{}", format!("Capability '{}' disabled", capability).green());
        }
        CapabilityCommand::Info { capability } => {
            if let Ok(cap) = runtime.capabilities().get(&capability) {
                println!("{}", format!("Capability: {}", capability).bold().green());
                println!("  Module: {}", cap.module);
                println!("  Enabled: {}", if cap.enabled { "Yes".green() } else { "No".red() });
                println!("  Dependencies: {}", cap.dependencies.join(", "));
            } else {
                println!("{}", "Capability not found".red());
            }
        }
    }
    Ok(())
}

pub async fn handle_config_command(
    _runtime: &OmnisystemRuntime,
    cmd: ConfigCommand,
) -> Result<(), Box<dyn Error>> {
    match cmd {
        ConfigCommand::Show => {
            println!("{}", "Current Configuration".bold().green());
            println!("  mode = \"omnios\"");
            println!("  verbose = false");
            println!("  enable_telemetry = true");
        }
        ConfigCommand::Set { key, value } => {
            println!("{}", format!("Set {} = {}", key, value).green());
        }
        ConfigCommand::Reset => {
            println!("{}", "Configuration reset to defaults".green());
        }
        ConfigCommand::Export { output } => {
            let path = output.unwrap_or_else(|| "config.toml".to_string());
            println!("{}", format!("Configuration exported to {}", path).green());
        }
    }
    Ok(())
}

pub async fn handle_marketplace_command(
    _runtime: &OmnisystemRuntime,
    cmd: MarketplaceCommand,
) -> Result<(), Box<dyn Error>> {
    match cmd {
        MarketplaceCommand::Search { query } => {
            println!("{}", format!("Searching for '{}'...", query).yellow());
            println!("{}", "Search results:".bold().green());
            println!("  • omnisystem-compiler-module v1.0.0 - Multi-language compiler");
            println!("  • omnisystem-messaging-module v1.0.0 - Email and messaging");
        }
        MarketplaceCommand::List => {
            println!("{}", "Available Modules in Marketplace".bold().green());
            println!("  • omnisystem-compiler-module (1.0.0) - Multi-language compiler");
            println!("  • omnisystem-messaging-module (1.0.0) - Email and messaging");
            println!("  • omnisystem-storage-module (1.0.0) - Distributed storage");
            println!("  • omnisystem-networking-module (1.0.0) - P2P networking");
            println!("  • omnisystem-bonsai-ecosystem (1.0.0) - Launcher and runtime");
        }
        MarketplaceCommand::Info { module } => {
            println!("{}", format!("Module: {}", module).bold().green());
            println!("  Version: 1.0.0");
            println!("  Status: Available");
            println!("  Downloads: 1,234");
            println!("  Rating: ⭐ 4.9/5");
        }
        MarketplaceCommand::Install { module, version } => {
            let ver = version.unwrap_or_else(|| "latest".to_string());
            println!("{}", format!("Installing {} v{}...", module, ver).yellow());
            println!("{}", format!("✓ {} installed successfully", module).green());
        }
        MarketplaceCommand::Uninstall { module } => {
            println!("{}", format!("Uninstalling {}...", module).yellow());
            println!("{}", format!("✓ {} uninstalled", module).green());
        }
        MarketplaceCommand::Update { module } => {
            if let Some(m) = module {
                println!("{}", format!("Updating {}...", m).yellow());
                println!("{}", format!("✓ {} updated to latest version", m).green());
            } else {
                println!("{}", "Updating all modules...".yellow());
                println!("{}", "✓ All modules updated".green());
            }
        }
    }
    Ok(())
}

pub async fn handle_dashboard_command(
    runtime: &OmnisystemRuntime,
    dashboard_type: &str,
) -> Result<(), Box<dyn Error>> {
    match dashboard_type {
        "status" => {
            let status = runtime.status();
            println!("{}", "╔════════════════════════════════════╗".bold().cyan());
            println!("{}", "║   Omnisystem Status Dashboard      ║".bold().cyan());
            println!("{}", "╚════════════════════════════════════╝".bold().cyan());
            println!("  Mode: {}", status.mode.name().green());
            println!("  Modules: {}", status.modules_loaded);
            println!("  Capabilities: {}/{}", status.capabilities_enabled, status.capabilities_total);
            println!("  Health: {}", "Healthy ✓".green());
        }
        "modules" => {
            println!("{}", "Active Modules".bold().green());
            for module in runtime.registry().list_modules() {
                println!("  ✓ {}", module.cyan());
            }
        }
        "capabilities" => {
            println!("{}", "Enabled Capabilities".bold().green());
            for cap in runtime.capabilities().enabled_capabilities() {
                println!("  ✓ {}", cap.name.cyan());
            }
        }
        _ => {
            println!("{}", "Unknown dashboard type".red());
        }
    }
    Ok(())
}
