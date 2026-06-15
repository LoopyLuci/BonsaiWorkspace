// BMN CLI — Command-line interface for streaming

use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "bonsai-stream")]
#[command(about = "Bonsai Media Nexus streaming CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a stream
    Start {
        /// Name of the stream
        #[arg(short, long)]
        name: String,

        /// Configuration file path
        #[arg(short, long)]
        config: Option<String>,
    },

    /// Stop the current stream
    Stop,

    /// Show stream status
    Status,

    /// List available sources
    Sources,

    /// List available scenes
    Scenes,

    /// Switch to a scene
    Switch {
        /// Scene name
        name: String,
    },

    /// Show metrics
    Metrics,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Start { name, config } => {
            println!("Starting stream: {}", name);
            if let Some(cfg) = config {
                println!("Using config: {}", cfg);
            }
        }
        Commands::Stop => {
            println!("Stopping stream...");
        }
        Commands::Status => {
            println!("Stream status: Running");
        }
        Commands::Sources => {
            println!("Available sources:");
            println!("  - Display (1080p60)");
            println!("  - Camera (720p30)");
            println!("  - Microphone");
        }
        Commands::Scenes => {
            println!("Available scenes:");
            println!("  - Main");
            println!("  - Gameplay");
            println!("  - Pause");
        }
        Commands::Switch { name } => {
            println!("Switching to scene: {}", name);
        }
        Commands::Metrics => {
            println!("Stream metrics:");
            println!("  FPS: 60.0");
            println!("  Bitrate: 5000 kbps");
            println!("  Latency: 150ms");
        }
    }

    Ok(())
}
