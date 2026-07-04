//! Cross-layer repair CLI

use cross_layer_repair::{CrossLayerError, CrossLayerRepairConfig, CrossLayerRepairSystem, SystemLayer};
use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        print_help();
        return;
    }

    match args[0].as_str() {
        "repair" => {
            if args.len() < 2 {
                eprintln!("Usage: cross-layer-repair repair <layer> <error-type>");
                return;
            }

            let layer_str = &args[1];
            let error_type = &args.get(2).map(|s| s.as_str()).unwrap_or("unknown");

            let layer = match layer_str {
                "uosc" => SystemLayer::UOSC,
                "omnisystem" => SystemLayer::Omnisystem,
                "bonsai" => SystemLayer::BonsaiEcosystem,
                _ => {
                    eprintln!("Unknown layer: {}", layer_str);
                    return;
                }
            };

            let config = CrossLayerRepairConfig::default();

            match CrossLayerRepairSystem::new(config) {
                Ok(system) => {
                    let error = CrossLayerError {
                        origin_layer: layer.clone(),
                        error_type: error_type.to_string(),
                        message: format!("Error in {:?} layer", layer),
                        affected_components: vec![],
                    };

                    match system.repair_system_wide(error).await {
                        Ok(result) => {
                            println!("✅ Cross-layer repair complete!");
                            println!("   Layers repaired: {}", result.layers_repaired);
                            println!("   Total repairs: {}", result.total_repairs);
                            println!("   Cascade depth: {}", result.cascade_depth);
                            for repair in &result.repairs_applied {
                                println!("   - Applied: {}", repair);
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Repair failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to create repair system: {}", e);
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
    println!("Cross-Layer Unified Repair Coordination System");
    println!();
    println!("USAGE:");
    println!("    cross-layer-repair <COMMAND> [OPTIONS]");
    println!();
    println!("COMMANDS:");
    println!("    repair <LAYER> <ERROR>  Perform cross-layer repair");
    println!("    help                   Show this help message");
    println!();
    println!("LAYERS:");
    println!("    uosc                   UOSC kernel layer");
    println!("    omnisystem             Omnisystem services layer");
    println!("    bonsai                 BonsaiEcosystem applications layer");
    println!();
    println!("EXAMPLES:");
    println!("    cross-layer-repair repair uosc kernel_panic");
    println!("    cross-layer-repair repair omnisystem service_crash");
}
