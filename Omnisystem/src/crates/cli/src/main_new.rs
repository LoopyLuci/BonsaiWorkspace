/// Bonsai CLI - Main entry point
/// This replaces the previous main.rs with a complete, functional implementation.

use anyhow::{anyhow, bail, Context, Result};
use blake3::Hasher;
use cargo_metadata::MetadataCommand;
use clap::{Parser, Subcommand, ValueEnum};
use log::{info, debug};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

mod bug_hunt;

use bug_hunt::ReportFormat;

#[derive(Parser, Debug)]
#[command(name = "bonsai")]
#[command(about = "Unified Bonsai Developer Toolkit CLI")]
#[command(version = "0.1.0")]
struct Cli {
    #[arg(long)]
    json: bool,
    #[arg(long)]
    workspace: Option<PathBuf>,
    #[arg(long)]
    verbose: bool,
    #[arg(long)]
    dry_run: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    BugHunt {
        #[command(subcommand)]
        action: BugHuntAction,
    },
    Version,
}

#[derive(Clone, Debug, Subcommand)]
enum BugHuntAction {
    Scan {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, value_enum)]
        format: Option<ReportFormatArg>,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        quick: bool,
        #[arg(long)]
        ai: bool,
    },
    List {
        #[arg(long)]
        severity: Option<String>,
    },
    Fix {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        confirm: bool,
    },
    Status,
    ClearCache,
}

#[derive(Clone, Debug, ValueEnum)]
enum ReportFormatArg {
    Json,
    Sarif,
    Html,
    Markdown,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let cli = Cli::parse();

    if cli.verbose {
        info!("Bonsai CLI starting");
        info!("Command: {:?}", cli.command);
    }

    match cli.command {
        Commands::Version => {
            println!("bonsai-cli v0.1.0");
        }
        Commands::BugHunt { action } => {
            handle_bug_hunt(action).await?;
        }
    }

    Ok(())
}

async fn handle_bug_hunt(action: BugHuntAction) -> Result<()> {
    match action {
        BugHuntAction::Scan {
            path,
            format,
            output,
            quick,
            ai,
        } => {
            let format = format.map(|f| match f {
                ReportFormatArg::Json => ReportFormat::Json,
                ReportFormatArg::Sarif => ReportFormat::Sarif,
                ReportFormatArg::Html => ReportFormat::Html,
                ReportFormatArg::Markdown => ReportFormat::Markdown,
            });

            bug_hunt::scan(path, format, output, quick, ai).await?;
        }
        BugHuntAction::List { severity } => {
            bug_hunt::list(severity)?;
        }
        BugHuntAction::Fix { id, all, confirm } => {
            bug_hunt::fix(id, all, confirm)?;
        }
        BugHuntAction::Status => {
            bug_hunt::status()?;
        }
        BugHuntAction::ClearCache => {
            bug_hunt::clear_cache()?;
        }
    }

    Ok(())
}

fn detect_workspace_root(start: &Path) -> Result<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join("Cargo.toml");
        if candidate.exists() {
            let content = fs::read_to_string(&candidate)
                .with_context(|| format!("failed reading {}", candidate.display()))?;
            if content.contains("[workspace]") {
                return Ok(current);
            }
        }
        if !current.pop() {
            break;
        }
    }
    bail!("could not find workspace root")
}
