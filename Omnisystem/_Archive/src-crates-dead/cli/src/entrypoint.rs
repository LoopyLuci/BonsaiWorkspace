/// Main entry point and command dispatch for Bonsai CLI.

use anyhow::Result;
use clap::Parser;
use log::info;
use std::path::PathBuf;

use crate::bug_hunt::{self, ReportFormat};
use crate::{Cli, Commands, BugHuntAction};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    let workspace = if let Some(ws) = cli.workspace {
        ws
    } else {
        crate::detect_workspace_root(&std::env::current_dir()?)?
    };

    if cli.verbose {
        info!("Workspace: {:?}", workspace);
        info!("Command: {:?}", cli.command);
    }

    let result = match cli.command {
        Commands::BugHunt { action } => match action {
            BugHuntAction::Scan {
                path,
                format,
                output,
                quick,
                ai,
            } => {
                bug_hunt::scan(path, format, output, quick, ai).await?;
                Ok(())
            }
            BugHuntAction::List { severity } => {
                bug_hunt::list(severity)?;
                Ok(())
            }
            BugHuntAction::Fix { id, all, confirm } => {
                bug_hunt::fix(id, all, confirm)?;
                Ok(())
            }
            BugHuntAction::Status => {
                bug_hunt::status()?;
                Ok(())
            }
            BugHuntAction::ClearCache => {
                bug_hunt::clear_cache()?;
                Ok(())
            }
        },
        _ => {
            eprintln!("Other commands are not yet implemented in this version.");
            std::process::exit(1);
        }
    };

    result
}
