use error_registry::{ErrorRegistry, ErrorCategory};

fn main() -> anyhow::Result<()> {
    println!("🌿 BonsaiWorkspace Error Registry");
    println!("==================================\n");

    let registry = ErrorRegistry::new();

    println!("Error Code Ranges:");
    println!("  E001–E099 : Installer errors");
    println!("  E100–E199 : Launcher errors");
    println!("  E200–E299 : Service errors");
    println!("  E300–E399 : Network errors");
    println!("  E400–E499 : Crypto/Identity errors");
    println!("  E500–E599 : Kernel errors\n");

    let all_codes = registry.list_all();
    println!("Total registered error codes: {}\n", all_codes.len());

    for category in &[
        ErrorCategory::Installer,
        ErrorCategory::Launcher,
        ErrorCategory::Service,
        ErrorCategory::Network,
        ErrorCategory::Crypto,
        ErrorCategory::Kernel,
    ] {
        let errors = registry.list_by_category(*category);
        if !errors.is_empty() {
            println!("{:?} Errors ({}):", category, errors.len());
            for error in errors {
                println!("  {} - {}", error.code, error.simple);
            }
            println!();
        }
    }

    println!("✅ Error registry loaded successfully");
    Ok(())
}
