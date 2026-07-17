//! CLI for exercising the complex-event-processing crate.

use complex_event_processing::{AlertSeverity, ComplexEventProcessor};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cep = ComplexEventProcessor::new();

    let pattern = cep
        .define_pattern("high_cpu_error", vec!["cpu>80".to_string(), "event_type=metric".to_string()], 5000)
        .await?;
    println!("Defined pattern '{}' with {} conditions", pattern.name, pattern.conditions.len());

    let mut attrs = HashMap::new();
    attrs.insert("cpu".to_string(), "93".to_string());
    let event = cep.ingest_event("metric", attrs).await?;

    let pattern_match = cep.match_pattern(pattern.pattern_id, vec![event.event_id]).await?;
    println!("Match confidence: {:.2}", pattern_match.confidence);

    let alert = cep
        .generate_alert(pattern_match.match_id, AlertSeverity::High, "CPU spike detected")
        .await?;
    println!("Generated alert: {:?} - {}", alert.severity, alert.message);

    println!("Total patterns defined: {}", cep.pattern_count());
    Ok(())
}
