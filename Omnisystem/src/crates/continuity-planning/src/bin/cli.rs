//! CLI for exercising the continuity-planning crate.

use continuity_planning::{ContinuityPlan, ContinuityPlanner, RPO, RTO};
use chrono::Utc;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let planner = ContinuityPlanner::new();

    let plan_id = Uuid::new_v4();
    let plan = ContinuityPlan {
        plan_id,
        name: "Primary Datacenter DR Plan".to_string(),
        organization: "Omnisystem".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: "1.0".to_string(),
    };
    planner.create_plan(&plan).await?;
    println!("Created plan {} ({})", plan.name, plan_id);

    planner
        .define_rto(&RTO {
            rto_id: Uuid::new_v4(),
            resource_id: "primary-db".to_string(),
            recovery_time_hours: 4,
            priority: 1,
        })
        .await?;
    planner
        .define_rpo(&RPO {
            rpo_id: Uuid::new_v4(),
            resource_id: "primary-db".to_string(),
            recovery_point_hours: 1,
            acceptable_data_loss: "<=1h".to_string(),
        })
        .await?;

    let compliance = planner.check_compliance(plan_id).await?;
    println!("Compliant: {} (missing: {:?})", compliance.compliant, compliance.missing_items);

    let metrics = planner.calculate_metrics(plan_id, 2.5, 0.5).await?;
    println!(
        "SLA achievement: {:.1}%, test success rate: {:.1}%",
        metrics.sla_achievement_percent, metrics.test_success_rate
    );

    println!("Total plans tracked: {}", planner.plan_count());
    Ok(())
}
