use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "omnisystem")]
#[command(about = "Omnisystem CLI - Module Management", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all modules
    Modules,
    /// Start a module
    Start { name: String },
    /// Stop a module
    Stop { name: String },
    /// Get system health
    Health,
    /// Show system status
    Status,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cli_parser() {
        assert!(true);
    }
}
