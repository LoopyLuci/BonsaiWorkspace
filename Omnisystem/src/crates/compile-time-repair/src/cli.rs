//! Command-line interface for compile-time repair

use crate::{CompileTimeRepairConfig, CompileTimeRepairSystem};
use anyhow::Result;

pub struct RepairCLI;

impl RepairCLI {
    pub async fn run(args: &[String]) -> Result<()> {
        if args.is_empty() {
            Self::print_help();
            return Ok(());
        }

        match args[0].as_str() {
            "repair" => Self::repair(&args[1..]).await,
            "analyze" => Self::analyze(&args[1..]).await,
            "stats" => Self::show_stats(&args[1..]).await,
            "help" | "--help" | "-h" => {
                Self::print_help();
                Ok(())
            }
            _ => {
                eprintln!("Unknown command: {}", args[0]);
                Self::print_help();
                Ok(())
            }
        }
    }

    async fn repair(args: &[String]) -> Result<()> {
        if args.is_empty() {
            eprintln!("Usage: repair-cli repair <file>");
            return Ok(());
        }

        let file = &args[0];
        let config = CompileTimeRepairConfig::default();
        let system = CompileTimeRepairSystem::new(config)?;

        println!("🔧 Analyzing {} for compile-time errors...", file);

        let result = system.analyze_and_repair(file).await?;

        println!("✅ Repair complete:");
        println!("   Files repaired: {}", result.files_repaired);
        println!("   Errors fixed: {}", result.errors_fixed);
        println!("   Average confidence: {:.2}%", result.confidence * 100.0);

        for repair in &result.repairs_applied {
            println!("   - Applied: {}", repair);
        }

        Ok(())
    }

    async fn analyze(args: &[String]) -> Result<()> {
        if args.is_empty() {
            eprintln!("Usage: repair-cli analyze <file>");
            return Ok(());
        }

        let file = &args[0];
        let config = CompileTimeRepairConfig {
            auto_repair: false, // Don't apply repairs, just analyze
            ..Default::default()
        };

        let system = CompileTimeRepairSystem::new(config)?;

        println!("🔍 Analyzing {} for compile-time errors...", file);

        let result = system.analyze_and_repair(file).await?;

        if result.errors_fixed == 0 {
            println!("✅ No compile-time errors found!");
        } else {
            println!("⚠️ Found {} potential errors:", result.errors_fixed);
            for repair in &result.repairs_applied {
                println!("   - {}", repair);
            }
        }

        Ok(())
    }

    async fn show_stats(args: &[String]) -> Result<()> {
        let config = CompileTimeRepairConfig::default();
        let system = CompileTimeRepairSystem::new(config)?;

        let stats = system.get_repair_statistics().await?;

        println!("📊 Repair Statistics:");
        println!("   Total repairs: {}", stats.total_repairs);
        println!("   Successful: {}", stats.successful_repairs);
        println!("   Failed: {}", stats.failed_repairs);
        println!("   Average confidence: {:.2}%", stats.average_confidence * 100.0);

        if let Some(error) = stats.most_common_error {
            println!("   Most common error: {}", error);
        }

        Ok(())
    }

    fn print_help() {
        println!("Compile-Time Repair System CLI");
        println!();
        println!("USAGE:");
        println!("    repair-cli <COMMAND> [OPTIONS]");
        println!();
        println!("COMMANDS:");
        println!("    repair <FILE>   Analyze and repair file");
        println!("    analyze <FILE>  Analyze file (no repairs)");
        println!("    stats          Show repair statistics");
        println!("    help           Show this help message");
        println!();
        println!("EXAMPLES:");
        println!("    repair-cli repair src/main.rs");
        println!("    repair-cli analyze src/lib.rs");
        println!("    repair-cli stats");
    }
}
