//! Example: Cross-Language Module Registration
//!
//! Demonstrates registering modules from different languages in the
//! `LanguageRegistry`, recording their exports/dependencies, and querying
//! them back — the real registry API, not a printed illustration.

use universal_language_layer::{Language, LanguageRegistry};

fn main() {
    let mut registry = LanguageRegistry::new();

    registry.register_module("app-manager-api", Language::Rust);
    registry.add_export("app-manager-api", "list_apps");
    registry.add_export("app-manager-api", "launch_app");

    registry.register_module("config-validator", Language::Titan);
    registry.add_export("config-validator", "validate");
    registry.add_dependency("config-validator", "app-manager-api");

    println!("Modules registered with the Universal Language Layer:");
    for module in registry.list_modules() {
        println!("  - {} ({:?})", module.name, module.language);
    }

    println!("\nRust modules:");
    for module in registry.list_by_language(Language::Rust) {
        println!("  - {}", module.name);
    }

    let config_validator = registry
        .get_module("config-validator")
        .expect("config-validator was just registered above");
    println!(
        "\n'{}' exports {:?} and depends on {:?}",
        config_validator.name, config_validator.exported_functions, config_validator.dependencies
    );
}
