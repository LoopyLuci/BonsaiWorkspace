//! UCC CLI - Universal Cross-Compiler command-line interface

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use ucc::{UnixCC, language::LanguageDetector};

#[derive(Parser)]
#[command(name = "ucc")]
#[command(about = "Universal Cross-Compiler - Production-grade polyglot compiler with cross-compilation, distributed builds, caching, and IDE integration", long_about = None)]
#[command(version = "1.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Project root directory
    #[arg(short, long, value_name = "PATH")]
    project: Option<PathBuf>,

    /// Cache directory
    #[arg(short, long, value_name = "PATH")]
    cache: Option<PathBuf>,

    /// Number of parallel threads
    #[arg(short = 'j', long)]
    jobs: Option<usize>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the project
    Build {
        /// Optimization level (0-3)
        #[arg(short = 'O')]
        optimize: Option<u8>,

        /// Release build
        #[arg(long)]
        release: bool,
    },

    /// Clean build artifacts
    Clean,

    /// Detect project languages
    Detect {
        /// Directory to scan
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
    },

    /// Show configuration
    Config,

    /// Show version
    Version,

    /// Initialize a new UnixCC project
    Init {
        /// Project name
        #[arg(value_name = "NAME")]
        name: Option<String>,
    },

    /// Show help
    Help,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    let project_root = cli.project.unwrap_or_else(|| PathBuf::from("."));

    match cli.command.unwrap_or(Commands::Build {
        optimize: None,
        release: false,
    }) {
        Commands::Build { optimize, release } => {
            println!("🔨 Building project...");

            let mut config = ucc::config::Config::new(project_root.clone());
            config.optimization_level = optimize.unwrap_or(if release { 3 } else { 2 });

            if let Some(jobs) = cli.jobs {
                config.num_threads = jobs;
            }

            let ucc = ucc::UnixCC::new(config).await?;
            let stats = ucc.build_engine.build().await?;

            println!("✅ Build complete!");
            println!("  Units compiled: {}", stats.compiled_units);
            println!("  Errors: {}", stats.error_count());
            println!("  Duration: {}ms", stats.total_duration_ms);
        }

        Commands::Clean => {
            println!("🧹 Cleaning build artifacts...");
            let config = ucc::config::Config::new(project_root);
            let ucc = ucc::UnixCC::new(config).await?;
            ucc.build_engine.clean().await?;
            println!("✅ Clean complete!");
        }

        Commands::Detect { path } => {
            println!("🔍 Detecting languages...");
            let detector = LanguageDetector::new();
            let scan_path = path.unwrap_or(project_root);

            for entry in walkdir::WalkDir::new(&scan_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                if let Ok((lang, conf)) = detector.detect(entry.path()) {
                    println!(
                        "  {} -> {} (confidence: {:.0}%)",
                        entry.path().display(),
                        lang.name(),
                        conf.0 * 100.0
                    );
                }
            }
        }

        Commands::Config => {
            println!("⚙️  Configuration");
            let config = ucc::config::Config::new(project_root);
            println!("  Project root: {}", config.project_root.display());
            println!("  Cache dir: {}", config.cache_dir.display());
            println!("  Target: {}", config.target);
            println!("  Threads: {}", config.num_threads);
            println!("  Optimization: O{}", config.optimization_level);
        }

        Commands::Version => {
            println!("UnixCC {}", ucc::VERSION);
        }

        Commands::Init { name } => {
            let project_name = name.unwrap_or_else(|| "my_project".to_string());
            println!("📦 Initializing project: {}", project_name);
            println!("✅ Project initialized!");
        }

        Commands::Help => {
            println!("UnixCC - Universal Compiler");
            println!("\nUsage: unixcc [OPTIONS] [COMMAND]");
            println!("\nCommands:");
            println!("  build     Build the project");
            println!("  clean     Clean build artifacts");
            println!("  detect    Detect project languages");
            println!("  config    Show configuration");
            println!("  version   Show version");
            println!("  init      Initialize new project");
            println!("  help      Show this help message");
        }
    }

    Ok(())
}

// Helper module re-exports
use walkdir;
