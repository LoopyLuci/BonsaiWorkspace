//! CLI for exercising the bedf-enhancements crate: lists the bedf fuzzing
//! suite's enhancement catalog and which modules are enabled.

use bedf_enhancements::{EnhancementEngine, EnhancementsConfig};

fn main() {
    let mut config = EnhancementsConfig::default();
    // Demonstrate that toggling a config flag actually changes what's
    // reported as enabled.
    config.enable_quantum_resistant = false;

    let engine = EnhancementEngine::new(config);

    println!("bedf enhancement catalog:");
    for enhancement in engine.list_enhancements() {
        println!(
            "  [{}] {} - {} ({})",
            enhancement.id,
            enhancement.name,
            enhancement.description,
            if enhancement.enabled { "enabled" } else { "disabled" }
        );
    }

    println!("\n{} of {} enhancements enabled", engine.get_enabled().len(), engine.list_enhancements().len());
}
