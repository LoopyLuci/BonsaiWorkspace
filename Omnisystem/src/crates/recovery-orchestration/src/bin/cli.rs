//! CLI

use chrono::Utc;
use recovery_orchestration::{RecoveryOrchestrator, RecoveryPlan};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orchestrator = RecoveryOrchestrator::new();
    let plan = RecoveryPlan {
        plan_id: Uuid::new_v4(),
        name: "db_recovery".to_string(),
        resource_id: "db1".to_string(),
        steps: vec![],
        created_at: Utc::now(),
    };

    orchestrator.create_recovery_plan(&plan).await?;
    println!("Created recovery plan: {}", plan.name);
    println!("Total plans: {}", orchestrator.plan_count());

    Ok(())
}
