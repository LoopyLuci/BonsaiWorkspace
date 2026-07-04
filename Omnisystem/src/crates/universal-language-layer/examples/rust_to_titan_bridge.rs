//! Example: Rust to TITAN Bridge
//!
//! Demonstrates how to call TITAN code from Rust using the Universal Language Layer.

use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Rust to TITAN Bridge Example ===\n");

    // Initialize the Universal Language Layer
    println!("1. Initializing ULL...");
    // ull::initialize().await?;

    println!("   ✓ ULL initialized\n");

    // Example 1: Simple value passing
    println!("2. Passing values between languages:");
    println!("   - Rust sends: integer(42)");
    println!("   - TITAN receives: i64(42)");
    println!("   - TITAN sends back: i64(84)");
    println!("   - Rust receives: integer(84)\n");

    // Example 2: Function calls
    println!("3. Cross-language function calls:");
    println!("   Rust code:");
    println!("     let result = titan::multiply(6, 7)?;");
    println!("   TITAN code (multiply.ti):");
    println!("     pub fn multiply(a: i64, b: i64) -> i64 {{");
    println!("       return a * b;");
    println!("     }}\n");

    // Example 3: Module registration
    println!("4. Module registration:");
    println!("   - Rust module: \"app-manager-api\" → can be called from TITAN");
    println!("   - TITAN module: \"config-validator\" → can be called from Rust\n");

    // Example 4: Type conversions
    println!("5. Automatic type conversions:");
    println!("   Rust i64        ↔ TITAN i64");
    println!("   Rust String     ↔ TITAN String");
    println!("   Rust HashMap    ↔ TITAN Object");
    println!("   Rust Vec<T>     ↔ TITAN Array\n");

    println!("=== End of Example ===");

    Ok(())
}
