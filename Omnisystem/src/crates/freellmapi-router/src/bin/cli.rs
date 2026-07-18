//! Demo CLI: registers a few providers with different latency/cost/reliability
//! profiles and exercises each real selection strategy.

use freellmapi_router::RouterService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let router = RouterService::new().await?;
    router.register_provider("openai", 5.0, 1.0, 100.0, 0.002).await?;
    router.register_provider("groq", 1.0, 1.0, 50.0, 0.0).await?;

    for strategy in ["balanced", "fastest", "cheapest", "reliable"] {
        let choice = router.select_provider("gpt-4", strategy).await?;
        println!("strategy={strategy} -> provider={choice}");
    }

    router.record_feedback("groq", true, 45.0).await?;
    let stats = router.get_provider_stats("groq").await?;
    println!("groq stats after feedback: {:?}", stats);

    Ok(())
}
