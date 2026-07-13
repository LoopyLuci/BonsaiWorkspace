//! CLI demo: log an audit event and run a compliance check.

use audit_system::{AuditEvent, AuditLogger, ComplianceChecker};
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = AuditLogger::new();
    let checker = ComplianceChecker::new();

    logger
        .log_event(&AuditEvent {
            event_id: "e1".to_string(),
            user_id: "u1".to_string(),
            action: "login".to_string(),
            resource: "/api/auth".to_string(),
            timestamp: Utc::now(),
            status: "success".to_string(),
        })
        .await?;
    println!("Logged {} event(s)", logger.event_count());

    let events = logger.get_events("u1").await?;
    println!("Events for u1: {:?}", events.iter().map(|e| &e.action).collect::<Vec<_>>());

    let report = checker.check_compliance("u1").await?;
    println!("Compliance status for u1: {} ({} violations)", report.status, report.violations);

    Ok(())
}
