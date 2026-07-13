use anyhow::Result;
use poe_core::{PoeCore, HostBiometricTelemetry, config::PoeConfig};
use std::sync::Arc;

mod scenarios;
mod telemetry;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧬 Poe AI — BUSH Simulation Suite");
    println!("{:=<60}", "");

    // ─── Phase 1: Initialize Poe AI ───
    println!("\n📟 Initializing Poe AI for simulation...");
    let config = PoeConfig {
        mesh_enabled: false,
        ..PoeConfig::default()
    };

    let core = match PoeCore::new(config).await {
        Ok(c) => {
            println!("✅ Poe Core initialized");
            Arc::new(c)
        }
        Err(e) => {
            eprintln!("⚠️  Poe Core init warning: {}", e);
            println!("🔄 Creating fallback core for BUSH testing...");
            Arc::new(PoeCore::new(PoeConfig::default()).await?)
        }
    };

    // ─── Phase 2: Run simulation scenarios ───
    println!("\n{:=<60}", "");
    println!("🧪 Running Simulation Scenarios");
    println!("{:=<60}", "");

    let results = scenarios::run_all(&core).await?;

    // ─── Phase 3: Report ───
    println!("\n{:=<60}", "");
    println!("📊 SIMULATION RESULTS");
    println!("{:=<60}", "");
    for result in &results {
        let status = if result.passed { "✅ PASS" } else { "❌ FAIL" };
        println!("{} — {}", status, result.name);
        if !result.passed {
            println!("   Error: {}", result.error.as_deref().unwrap_or("unknown"));
        }
    }

    let passed = results.iter().filter(|r| r.passed).count();
    let total = results.len();
    println!("\n{}/{} scenarios passed", passed, total);

    if passed == total {
        println!("\n🎉 All scenarios passed! Poe AI is ready for pendant deployment.");
    } else {
        println!("\n⚠️  Some scenarios failed. Review the report above.");
    }

    Ok(())
}
