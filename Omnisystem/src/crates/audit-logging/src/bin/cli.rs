//! CLI for exercising the audit-logging crate.

use audit_logging::AuditLogger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = AuditLogger::new();

    logger.log_action("alice", "login", "/api/auth", true).await?;
    let log2 = logger.log_action("alice", "delete_record", "/api/records/42", true).await?;
    logger.log_action("bob", "login", "/api/auth", false).await?;

    println!("Total logs: {}", logger.log_count());

    let verified = logger.verify_integrity(log2.log_id).await?;
    println!("Log {} integrity verified: {}", log2.log_id, verified);

    let chain_intact = logger.verify_chain().await?;
    println!("Full chain intact: {}", chain_intact);

    logger.set_retention_policy("security_logs", 365).await?;

    let report = logger.generate_report().await?;
    println!(
        "Report: {} total, {} succeeded, {} failed, actors: {:?}",
        report.total_logs, report.success_count, report.failure_count, report.actors
    );

    Ok(())
}
