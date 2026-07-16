//! CLI: register a pricing model, calculate cost for some usage, and print
//! a cost report and trend analysis.

use chrono::Utc;
use cost_analyzer::{BillingPeriod, CostCalculator, PricingModel};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let calculator = CostCalculator::new();

    let model = PricingModel {
        model_id: Uuid::new_v4(),
        resource_type: "cpu".to_string(),
        unit_cost: 0.05,
        currency: "USD".to_string(),
        billing_period: BillingPeriod::Hourly,
    };
    calculator.register_pricing_model(&model).await?;

    let cost = calculator.calculate_cost("tenant1", "cpu", 240.0).await?;
    println!("cost for 240 cpu-hours: ${:.2}", cost);

    let now = Utc::now();
    let report = calculator
        .generate_report("tenant1", now - chrono::Duration::days(1), now)
        .await?;
    println!("report total: ${:.2}", report.total_cost);

    let trend = calculator.analyze_trend("tenant1").await?;
    println!(
        "trend: {:?} (avg daily ${:.2}, peak ${:.2})",
        trend.cost_trend, trend.avg_daily_cost, trend.peak_daily_cost
    );

    println!("total cost records: {}", calculator.record_count());

    Ok(())
}
