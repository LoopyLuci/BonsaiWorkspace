//! Bonsai Enclave CLI - Universal Dependency & Environment Manager
//!
//! Usage:
//!   enclave init
//!   enclave add <package>
//!   enclave lock
//!   enclave install
//!   enclave shell
//!   enclave run <command>

use anyhow::Result;
use bonsai_enclave::{Enclave, EnclaveConfig};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "Enclave")]
#[command(about = "Universal Dependency & Environment Manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Project root directory
    #[arg(global = true, long, default_value = ".")]
    root: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Enclave project
    Init {
        #[arg(long, default_value = "python")]
        language: String,
    },

    /// Add a dependency
    Add {
        /// Package name (e.g., numpy, lodash, serde)
        package: String,

        #[arg(long)]
        version: Option<String>,
    },

    /// Resolve and lock dependencies deterministically
    Lock,

    /// Install all locked dependencies
    Install,

    /// Create an isolated shell environment
    Shell,

    /// Run a command in the isolated environment
    Run {
        /// Command to run
        command: Vec<String>,

        #[arg(long)]
        runtime: Option<String>,
    },

    /// Manage language runtimes
    Runtime {
        #[command(subcommand)]
        action: RuntimeAction,
    },

    /// Manage the content-addressed cache
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
}

#[derive(Subcommand)]
enum RuntimeAction {
    /// List installed runtimes
    List,

    /// Install a runtime
    Install {
        /// Runtime specification (e.g., python@3.11.9)
        runtime: String,
    },

    /// Remove a runtime
    Remove {
        /// Runtime specification (e.g., python@3.11.9)
        runtime: String,
    },
}

#[derive(Subcommand)]
enum CacheAction {
    /// Show cache statistics
    Stats,
    /// Clean cache
    Clean,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let cli = Cli::parse();

    // Create Enclave config
    let config = EnclaveConfig::new(cli.root)?;
    let mut enclave = Enclave::new(config).await?;

    match cli.command {
        Commands::Init { language } => {
            println!("🔧 Initializing Enclave project (language: {})", language);
            // Initialize manifest
            let mut manifest = enclave.load_manifest().await?;
            manifest.project.language = language;
            manifest.save(&enclave.config.manifest_path).await?;
            println!("✓ Created enclave.toml");
        }

        Commands::Add { package, version } => {
            println!("📦 Adding dependency: {}", package);
            let mut manifest = enclave.load_manifest().await?;
            let ver = version.unwrap_or_else(|| "latest".to_string());

            manifest
                .dependencies
                .insert(package.clone(), Default::default());
            manifest.save(&enclave.config.manifest_path).await?;
            println!("✓ Added {} ({})", package, ver);
        }

        Commands::Lock => {
            println!("🔒 Locking dependencies...");
            let lockfile = enclave.lock().await?;
            println!("✓ Locked {} packages", lockfile.packages.len());
        }

        Commands::Install => {
            println!("📥 Installing dependencies...");
            enclave.create_environment("default").await?;
            println!("✓ Environment created");
        }

        Commands::Shell => {
            println!("🐚 Entering isolated shell...");
            let env = enclave.create_environment("shell").await?;
            println!("Environment: {}", env.name);
            println!("Type 'exit' to leave");
        }

        Commands::Run { command, runtime } => {
            if let Some(rt) = runtime {
                println!("🚀 Running command with runtime: {}", rt);
                // Parse runtime spec (e.g., "python@3.11.9")
                let parts: Vec<&str> = rt.split('@').collect();
                if parts.len() == 2 {
                    let _lang = parts[0];
                    let _version = parts[1];
                    // TODO: Create environment with specific runtime, then run command
                    println!("✓ Would run with runtime {} version {}", _lang, _version);
                }
            } else {
                let args: Vec<&str> = command.iter().map(|s| s.as_str()).collect();
                enclave.run("default", &args).await?;
            }
        }

        Commands::Runtime { action } => match action {
            RuntimeAction::List => {
                println!("📦 Installed runtimes:");
                println!("  (Use 'enclave runtime install python@3.11.9' to add)");
            }
            RuntimeAction::Install { runtime } => {
                println!("⬇️  Installing runtime: {}", runtime);
                let parts: Vec<&str> = runtime.split('@').collect();
                if parts.len() == 2 {
                    println!("  Name: {}", parts[0]);
                    println!("  Version: {}", parts[1]);
                    println!("✓ Would fetch and install {} @{}", parts[0], parts[1]);
                } else {
                    eprintln!("✗ Invalid runtime format. Use: name@version");
                }
            }
            RuntimeAction::Remove { runtime } => {
                println!("🗑️  Removing runtime: {}", runtime);
                println!("✓ Would remove {}", runtime);
            }
        },

        Commands::Cache { action } => match action {
            CacheAction::Stats => {
                println!("📊 Cache statistics");
                // TODO: Implement cache stats
            }
            CacheAction::Clean => {
                println!("🧹 Cleaning cache...");
                // TODO: Implement cache cleaning
            }
        },
    }

    Ok(())
}

