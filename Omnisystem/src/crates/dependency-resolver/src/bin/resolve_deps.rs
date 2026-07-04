//! Dependency resolution CLI

use dependency_resolver::{DependencyResolutionConfig, DependencyResolutionSystem, ModuleId};
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
        "resolve" => {
            if args.len() < 3 {
                eprintln!("Usage: resolve-deps resolve <module-name> <version>");
                return;
            }

            let module = ModuleId {
                name: args[1].clone(),
                version: args[2].clone(),
            };

            let config = DependencyResolutionConfig::default();

            match DependencyResolutionSystem::new(config) {
                Ok(system) => {
                    match system.resolve_dependencies(&module).await {
                        Ok(result) => {
                            println!("✅ Dependencies resolved!");
                            println!("   Module: {} v{}", result.module.name, result.module.version);
                            println!("   Dependencies found: {}", result.dependencies.len());
                            println!("   Modules loaded: {}", result.loaded_modules.len());
                            for dep in &result.dependencies {
                                println!("   - {} v{}", dep.module.name, dep.module.version);
                            }
                        }
                        Err(e) => eprintln!("❌ Resolution failed: {}", e),
                    }
                }
                Err(e) => eprintln!("❌ Failed to create resolver: {}", e),
            }
        }
        "stats" => {
            let config = DependencyResolutionConfig::default();

            match DependencyResolutionSystem::new(config) {
                Ok(system) => {
                    match system.get_statistics().await {
                        Ok(stats) => {
                            println!("📊 Dependency Statistics:");
                            println!("   Total modules: {}", stats.total_modules);
                            println!("   Total dependencies: {}", stats.total_dependencies);
                            println!("   Unresolved: {}", stats.unresolved);
                            println!("   Conflicts fixed: {}", stats.conflicts_fixed);
                        }
                        Err(e) => eprintln!("❌ Error: {}", e),
                    }
                }
                Err(e) => eprintln!("❌ Failed to create resolver: {}", e),
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
    println!("Automatic Module Dependency Resolution System");
    println!();
    println!("USAGE:");
    println!("    resolve-deps <COMMAND> [OPTIONS]");
    println!();
    println!("COMMANDS:");
    println!("    resolve <MODULE> <VERSION>  Resolve module dependencies");
    println!("    stats                      Show dependency statistics");
    println!("    help                       Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("    resolve-deps resolve omnisystem-core 1.0.0");
    println!("    resolve-deps stats");
}
