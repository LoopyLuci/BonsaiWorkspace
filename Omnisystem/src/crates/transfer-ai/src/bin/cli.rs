//! CLI: demonstrate the advisor's honest "no model loaded" behavior and the
//! safety envelope clamping out-of-bounds AI-suggested values.

use transfer_ai::{AiAdvice, AiCongestionAdvisor, SafetyEnvelope};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut advisor = AiCongestionAdvisor::new(0.9);
    println!("advisor healthy before model load: {}", advisor.is_healthy());

    let advice = advisor.advise(50.0, 0.01).await;
    println!("advice with no model loaded: {:?}", advice);

    advisor.load_model("dummy-model-path").await?;
    println!("advisor healthy after model load: {}", advisor.is_healthy());

    // Even once "healthy", the advisor currently has no model backend, so it
    // still reports no advice rather than fabricating one.
    let advice = advisor.advise(50.0, 0.01).await;
    println!("advice after model load (no backend wired yet): {:?}", advice);

    // Show the safety envelope clamping a hypothetical out-of-bounds
    // suggestion to provably-safe bounds.
    let envelope = SafetyEnvelope::defaults();
    let mut hypothetical = AiAdvice {
        suggested_cwnd: 500_000_000,
        suggested_pacing_rate: 5_000_000_000,
        confidence: 0.75,
        reasoning: "hypothetical oversized suggestion".to_string(),
    };
    envelope.clamp(&mut hypothetical);
    println!(
        "clamped advice: cwnd={} pacing_rate={} within_bounds={}",
        hypothetical.suggested_cwnd,
        hypothetical.suggested_pacing_rate,
        envelope.verify(&hypothetical)
    );

    Ok(())
}
