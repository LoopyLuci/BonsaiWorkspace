//! Demo CLI: creates a tenant in real SQLite-backed storage, records usage
//! through the real BillingService, and checks the budget it just spent against.

use freellmapi_billing::BillingService;
use freellmapi_core::{unix_now, Tenant};
use freellmapi_storage::StorageManager;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dir = std::env::temp_dir().join(format!("freellmapi-billing-demo-{}", std::process::id()));
    let db_path = dir.join("demo.db");
    let storage = Arc::new(StorageManager::new(db_path.to_str().unwrap()).await?);

    storage
        .create_tenant(&Tenant {
            id: "demo-tenant".to_string(),
            name: "Demo Tenant".to_string(),
            email: "demo@example.com".to_string(),
            tier: "pro".to_string(),
            monthly_budget_usd: 5.0,
            created_at: unix_now(),
        })
        .await?;

    let billing = BillingService::new(storage).await?;

    let cost = billing.calculate_cost("gpt-4", 1000, 500).await?;
    println!("Estimated cost for this request: ${cost:.4}");

    let within_budget = billing.check_budget("demo-tenant", cost).await?;
    println!("Within budget? {within_budget}");

    billing
        .record_usage("demo-tenant", "gpt-4", "openai", 1000, 500, 320)
        .await?;
    println!("Recorded usage.");

    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}
