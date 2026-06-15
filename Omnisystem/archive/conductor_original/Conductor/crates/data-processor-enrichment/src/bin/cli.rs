//! CLI

use data_processor_enrichment::Analytics;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let analytics = Analytics::new();
    println!("Analytics initialized");

    analytics.add_point("metrics", 42.0);
    let result = analytics.analyze("metrics").await?;
    println!("Result: {}", result);

    let insights = analytics.get_insights();
    println!("Insights: {}", insights);

    Ok(())
}
