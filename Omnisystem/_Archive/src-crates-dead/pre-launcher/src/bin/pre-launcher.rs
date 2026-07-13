// Pre-Launcher Binary
// Automatic bootstrap and dependency installation for Omnisystem

use pre_launcher::Bootstrap;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    // Parse arguments
    let check_only = args.contains(&"--check".to_string())
        || args.contains(&"-c".to_string());
    let verbose = args.contains(&"--verbose".to_string())
        || args.contains(&"-v".to_string());

    if verbose {
        env::set_var("RUST_LOG", "debug");
    }

    // Run bootstrap
    let result = if check_only {
        println!("Running in check mode (no installation)...");
        Bootstrap::check().await?
    } else {
        println!("Running in automatic installation mode...");
        Bootstrap::run().await?
    };

    // Print results
    if result.initialized {
        println!("\n✅ Pre-launcher bootstrap successful!");
        println!("System: {}", result.system_info);
        println!("Startup Time: {}ms", result.startup_duration_ms);
        println!("Services Started: {}", result.services_started.join(", "));

        if !result.dependencies_installed.is_empty() {
            println!(
                "Dependencies Installed: {}",
                result.dependencies_installed.join(", ")
            );
        }

        println!("\n🚀 You're ready to use Omnisystem!");
        Ok(())
    } else {
        println!("\n⚠️  Bootstrap incomplete. Please check the output above.");
        Err(anyhow::anyhow!(
            "Bootstrap failed: some dependencies could not be installed"
        ))
    }
}
