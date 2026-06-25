//! Polyglot Pong Orchestrator - Distributed Language Validation
//!
//! Central coordinator that distributes Pong compilation & execution jobs
//! across 750+ language sandboxes and aggregates results.

use clap::Parser;
use polyglot_pong_common::*;
use polyglot_pong_orchestrator::Orchestrator;
use std::path::Path;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(name = "Polyglot Pong Orchestrator")]
#[command(about = "Distributed language validation framework")]
struct Args {
    /// Path to language manifest (JSON list of languages)
    #[arg(long, default_value = "languages.json")]
    manifest: String,

    /// Number of sandbox nodes to coordinate
    #[arg(long, default_value = "10")]
    nodes: usize,

    /// Enable AI enhancements (requires feature flag)
    #[arg(long, default_value = "false")]
    ai: bool,

    /// Enable differential fuzzing for bug discovery
    #[arg(long, default_value = "true")]
    fuzz: bool,

    /// Output directory for results
    #[arg(long, default_value = "output")]
    output: String,

    /// Number of conversion rounds to test
    #[arg(long, default_value = "1")]
    rounds: u32,

    /// Maximum languages to test (0 = all)
    #[arg(long, default_value = "0")]
    limit: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();

    info!("Starting Polyglot Pong Orchestrator");
    info!("Manifest: {}", args.manifest);
    info!("Nodes: {}", args.nodes);
    info!("AI Enabled: {}", args.ai);
    info!("Fuzzing Enabled: {}", args.fuzz);
    info!("Rounds: {}", args.rounds);

    // Load language manifest
    let languages = load_manifest(&args.manifest)?;
    let languages = if args.limit > 0 {
        languages.into_iter().take(args.limit).collect()
    } else {
        languages
    };

    info!("Loaded {} languages", languages.len());

    // Create output directory
    if !Path::new(&args.output).exists() {
        std::fs::create_dir_all(&args.output)?;
        info!("Created output directory: {}", args.output);
    }

    // Create orchestrator
    let orchestrator = Orchestrator::new(languages, args.ai, args.fuzz).await?;
    info!("Orchestrator initialized");

    // Run test matrix
    info!("Starting test matrix execution...");
    orchestrator.run().await?;

    info!("All jobs completed successfully!");
    Ok(())
}

/// Load languages from JSON manifest
fn load_manifest(path: &str) -> anyhow::Result<Vec<Language>> {
    if !Path::new(path).exists() {
        // Return default languages if manifest doesn't exist
        let defaults = vec![
            "Rust".into(),
            "Python".into(),
            "JavaScript".into(),
            "Go".into(),
            "C".into(),
            "C++".into(),
            "Java".into(),
            "C#".into(),
            "TypeScript".into(),
            "Swift".into(),
        ];
        info!("Manifest not found, using {} default languages", defaults.len());
        return Ok(defaults);
    }

    let contents = std::fs::read_to_string(path)?;
    let languages: Vec<Language> = serde_json::from_str(&contents)?;
    Ok(languages)
}
