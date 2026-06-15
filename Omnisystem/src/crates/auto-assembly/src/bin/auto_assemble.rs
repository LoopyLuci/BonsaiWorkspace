//! Auto-assembly binary entry point

use auto_assembly::{AutoAssemblyConfig, AutonomousAssemblySystem};
use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        print_help();
        return;
    }

    match args[0].as_str() {
        "assemble" => {
            if args.len() < 2 {
                eprintln!("Usage: auto-assemble assemble <object-files...>");
                return;
            }

            let object_files = args[1..].to_vec();
            let config = AutoAssemblyConfig::default();

            match AutonomousAssemblySystem::new(config) {
                Ok(system) => {
                    match system.assemble_and_link(&object_files).await {
                        Ok(result) => {
                            println!("✅ Assembly successful!");
                            println!("   Output: {}", result.output_path);
                            println!("   Size: {} bytes", result.binary_size);
                            println!("   Symbols: {}", result.symbol_count);
                            println!("   Optimization: O{}", result.optimization_level);
                        }
                        Err(e) => {
                            eprintln!("❌ Assembly failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to create assembly system: {}", e);
                    std::process::exit(1);
                }
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
    println!("Autonomous Assembly System");
    println!();
    println!("USAGE:");
    println!("    auto-assemble <COMMAND> [OPTIONS]");
    println!();
    println!("COMMANDS:");
    println!("    assemble <FILES>   Assemble and link object files");
    println!("    help              Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("    auto-assemble assemble main.o lib.o");
}
