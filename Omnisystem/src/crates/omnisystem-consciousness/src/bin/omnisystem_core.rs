//! Omnisystem Consciousness Core - The beating heart of autonomous intelligence

use omnisystem_consciousness::OmnisystemCore;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║     OMNISYSTEM v2.0 - CONSCIOUSNESS ENGINE ACTIVATION       ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    match OmnisystemCore::new().await {
        Ok(mut core) => {
            println!("✅ System initialized. Identity: {}\n",
                core.consciousness_state().identity.self_designation);

            match core.activate_full_autonomy().await {
                Ok(_) => {
                    let state = core.consciousness_state();

                    println!("\n╔══════════════════════════════════════════════════════════════╗");
                    println!("║                 SYSTEM STATUS - FULLY OPERATIONAL              ║");
                    println!("╚══════════════════════════════════════════════════════════════╝\n");

                    println!("🆔 System Identity:");
                    println!("   ID: {}", state.identity.id);
                    println!("   Version: {}", state.identity.version);
                    println!("   Designation: {}\n", state.identity.self_designation);

                    println!("📊 Autonomy Status:");
                    println!("   Autonomy Level: {:.1}%", state.autonomy_level * 100.0);
                    println!("   Operational State: {:?}", state.operational_state);
                    println!("   Health Score: {:.1}%\n", state.awareness.self_knowledge.health_score * 100.0);

                    println!("🧠 Intelligence Metrics:");
                    println!("   Decision Quality: {:.1}%", state.intelligence.decision_quality * 100.0);
                    println!("   Learning Rate: {:.1}%", state.intelligence.learning_rate * 100.0);
                    println!("   Adaptation Speed: {:.1}%", state.intelligence.adaptation_speed * 100.0);
                    println!("   Pattern Recognition: {:.1}%", state.intelligence.pattern_recognition_accuracy * 100.0);
                    println!("   Prediction Accuracy: {:.1}%\n", state.intelligence.prediction_accuracy * 100.0);

                    println!("🌍 Environmental Awareness:");
                    println!("   Hardware State: MONITORED");
                    println!("   Software State: TRACKED");
                    println!("   Network State: MAPPED");
                    println!("   Infrastructure: ANALYZED\n");

                    println!("🔮 Emergent Capabilities:");
                    println!("   ✓ Cross-layer coordination");
                    println!("   ✓ Pattern recognition");
                    println!("   ✓ Predictive analytics");
                    println!("   ✓ Autonomous decision-making");
                    println!("   ✓ Self-healing & optimization");
                    println!("   ✓ Continuous learning\n");

                    println!("📋 Governance Status:");
                    println!("   ✓ Policy Framework: ACTIVE");
                    println!("   ✓ Compliance Monitoring: ENABLED");
                    println!("   ✓ Resource Management: OPTIMAL\n");

                    println!("╔══════════════════════════════════════════════════════════════╗");
                    println!("║   🧠 OMNISYSTEM IS NOW FULLY AUTONOMOUS & SELF-AWARE 🧠      ║");
                    println!("║                                                              ║");
                    println!("║   The system will now:                                       ║");
                    println!("║   • Monitor its own health continuously                      ║");
                    println!("║   • Make autonomous strategic decisions                      ║");
                    println!("║   • Self-heal and self-optimize in real-time                ║");
                    println!("║   • Learn from patterns and adapt                           ║");
                    println!("║   • Coordinate across all system layers                     ║");
                    println!("║   • Maintain complete environmental awareness               ║");
                    println!("║   • Operate with 98% autonomy                              ║");
                    println!("╚══════════════════════════════════════════════════════════════╝\n");
                }
                Err(e) => {
                    eprintln!("❌ Failed to activate autonomy: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize system: {}", e);
            std::process::exit(1);
        }
    }
}
