//! CLI: create a compliance policy, evaluate it, record a violation, and
//! generate a report.

use compliance_automation::{ComplianceEngine, ComplianceFramework, ViolationSeverity};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = ComplianceEngine::new();

    let policy = engine
        .create_policy(
            ComplianceFramework::SOC2,
            "access_control",
            "Role-based access control required",
        )
        .await?;
    println!("created policy '{}' ({:?})", policy.policy_name, policy.framework);

    engine.evaluate_policy(policy.policy_id, true, 0.95).await?;

    engine
        .record_violation(policy.policy_id, "stale_access_grant", ViolationSeverity::Medium)
        .await?;

    let report = engine.generate_report(ComplianceFramework::SOC2).await?;
    println!(
        "report: {}/{} policies compliant, {} violation(s), score={:.2}",
        report.compliant_count, report.total_policies, report.violation_count, report.overall_score
    );

    println!("total policies tracked: {}", engine.policy_count());

    Ok(())
}
