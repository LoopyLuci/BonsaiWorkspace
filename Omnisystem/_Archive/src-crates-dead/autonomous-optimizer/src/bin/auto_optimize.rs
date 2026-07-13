//! Autonomous optimization CLI

use autonomous_optimizer::{AutonomousOptimizerConfig, AutonomousOptimizerSystem};
use std::env;

#[tokio::main]
async fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        print_help();
        return;
    }

    match args[0].as_str() {
        "start" => {
            println!("🚀 Starting autonomous optimizer...");
            let config = AutonomousOptimizerConfig::default();

            match AutonomousOptimizerSystem::new(config) {
                Ok(system) => {
                    match system.start_continuous_optimization().await {
                        Ok(_) => println!("✅ Optimizer completed"),
                        Err(e) => eprintln!("❌ Optimizer error: {}", e),
                    }
                }
                Err(e) => eprintln!("❌ Failed to create optimizer: {}", e),
            }
        }
        "stats" => {
            println!("📊 Getting optimization statistics...");
            let config = AutonomousOptimizerConfig::default();

            match AutonomousOptimizerSystem::new(config) {
                Ok(system) => {
                    match system.get_statistics().await {
                        Ok(stats) => {
                            println!("Optimization Statistics:");
                            println!("  Total optimizations: {}", stats.total_optimizations);
                            println!("  Successful: {}", stats.successful);
                            println!("  Failed: {}", stats.failed);
                            println!(
                                "  Average improvement: {:.2}%",
                                stats.avg_improvement_percent
                            );
                        }
                        Err(e) => eprintln!("❌ Error: {}", e),
                    }
                }
                Err(e) => eprintln!("❌ Failed to create optimizer: {}", e),
            }
        }
        "help" | "--help" | "-h" => print_help(),
        _ => {
            eprintln!("Unknown command: {}", args[0]);
            print_help();
            std::process::exit(1);
        }
    }
}

fn print_help() {
    println!("Autonomous Performance Optimization System");
    println!();
    println!("USAGE:");
    println!("    auto-optimize <COMMAND>");
    println!();
    println!("COMMANDS:");
    println!("    start   Start continuous optimization");
    println!("    stats   Show optimization statistics");
    println!("    help    Show this help message");
}
