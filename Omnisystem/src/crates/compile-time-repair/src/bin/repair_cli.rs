//! compile-time-repair CLI.
//!
//! Usage:
//!   compile_time_repair_cli repair <file> [--db <path>]   analyze + apply repairs
//!   compile_time_repair_cli analyze <file>                 analyze only, no writes
//!   compile_time_repair_cli stats [--db <path>]            show repair history stats
//!   compile_time_repair_cli help

use compile_time_repair::{CompileTimeAnalyzer, RepairEngine};
use std::env;

const DEFAULT_DB_PATH: &str = ".omnisystem/repair_history.db";

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if let Err(e) = run(&args).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run(args: &[String]) -> anyhow::Result<()> {
    if args.is_empty() {
        print_help();
        return Ok(());
    }

    match args[0].as_str() {
        "repair" => repair(&args[1..]).await,
        "analyze" => analyze(&args[1..]),
        "stats" => show_stats(&args[1..]).await,
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => {
            eprintln!("Unknown command: {other}");
            print_help();
            Ok(())
        }
    }
}

fn db_path_arg(args: &[String]) -> String {
    args.iter()
        .position(|a| a == "--db")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| DEFAULT_DB_PATH.to_string())
}

async fn repair(args: &[String]) -> anyhow::Result<()> {
    let Some(file) = args.first() else {
        eprintln!("Usage: compile_time_repair_cli repair <file> [--db <path>]");
        return Ok(());
    };

    let db_path = db_path_arg(args);
    let analyzer = CompileTimeAnalyzer::new();
    let engine = RepairEngine::new(&db_path)?;

    let errors = analyzer.analyze_file(file)?;
    if errors.is_empty() {
        println!("No compile-time issues found in {file}.");
        return Ok(());
    }

    let repairs = engine.find_repairs(&errors)?;
    let applied = engine.apply_repairs(file, &repairs).await?;

    println!("Repair complete for {file}:");
    println!("  Issues found:  {}", errors.len());
    println!("  Repairs applied: {}", applied.len());
    for pattern_id in &applied {
        println!("   - {pattern_id}");
    }

    Ok(())
}

fn analyze(args: &[String]) -> anyhow::Result<()> {
    let Some(file) = args.first() else {
        eprintln!("Usage: compile_time_repair_cli analyze <file>");
        return Ok(());
    };

    let analyzer = CompileTimeAnalyzer::new();
    let errors = analyzer.analyze_file(file)?;

    if errors.is_empty() {
        println!("No compile-time issues found in {file}.");
    } else {
        println!("Found {} potential issue(s) in {file}:", errors.len());
        for e in &errors {
            println!("  line {}: {:?} - {}", e.line, e.error_type, e.message);
        }
    }

    Ok(())
}

async fn show_stats(args: &[String]) -> anyhow::Result<()> {
    let db_path = db_path_arg(args);
    let engine = RepairEngine::new(&db_path)?;
    let stats = engine.get_statistics().await?;

    println!("Repair Statistics ({db_path}):");
    println!("  Total repairs:      {}", stats.total_repairs);
    println!("  Successful:         {}", stats.successful_repairs);
    println!("  Failed:             {}", stats.failed_repairs);
    println!("  Average confidence: {:.2}%", stats.average_confidence * 100.0);
    if let Some(err) = &stats.most_common_error {
        println!("  Most common error:  {err}");
    }

    Ok(())
}

fn print_help() {
    println!("compile-time-repair CLI");
    println!();
    println!("USAGE:");
    println!("    compile_time_repair_cli <COMMAND> [OPTIONS]");
    println!();
    println!("COMMANDS:");
    println!("    repair <FILE> [--db <path>]   Analyze and repair a file");
    println!("    analyze <FILE>                 Analyze a file (no writes)");
    println!("    stats [--db <path>]            Show repair history statistics");
    println!("    help                            Show this help message");
}
