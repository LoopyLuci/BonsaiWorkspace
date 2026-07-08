/// Integration with the Universe event log.
/// 
/// Every scan, finding, and fix is logged as an event in Universe for
/// persistent tracking, time-travel debugging, and analytics.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use log::info;
use serde_json::{json, Value};
use uuid::Uuid;
use crate::ScanReport;

/// Event ID for tracking purposes.
pub type EventId = String;

/// Universe event types for Bug Hunt.
#[derive(Debug, Clone)]
pub enum BugHuntEvent {
    SweepStarted {
        sweep_id: String,
        repository: String,
        trigger: String, // "manual" | "survival" | "ci"
        timestamp: DateTime<Utc>,
    },
    SweepCompleted {
        sweep_id: String,
        repository: String,
        files_scanned: usize,
        issues_found: usize,
        critical_count: usize,
        high_count: usize,
        duration_ms: u64,
        report_cas_hash: String,
        timestamp: DateTime<Utc>,
    },
    FindingCreated {
        finding_id: String,
        sweep_id: String,
        rule_id: String,
        severity: String,
        confidence: f32,
        timestamp: DateTime<Utc>,
    },
    FixApplied {
        finding_id: String,
        sweep_id: String,
        success: bool,
        error_message: Option<String>,
        timestamp: DateTime<Utc>,
    },
    PatternAdded {
        pattern_id: String,
        source: String,
        rule_id: String,
        votes: usize,
        timestamp: DateTime<Utc>,
    },
}

/// Log a scan completion event to Universe.
pub async fn log_sweep_completed(
    sweep_id: &str,
    repository: &str,
    report: &ScanReport,
    duration_ms: u64,
    report_cas_hash: &str,
) -> Result<EventId> {
    let critical_count = report
        .issues
        .iter()
        .filter(|f| f.severity as u8 >= 4)
        .count();
    let high_count = report
        .issues
        .iter()
        .filter(|f| f.severity as u8 == 3)
        .count();

    let event = BugHuntEvent::SweepCompleted {
        sweep_id: sweep_id.to_string(),
        repository: repository.to_string(),
        files_scanned: report.summary.files_scanned,
        issues_found: report.issues.len(),
        critical_count,
        high_count,
        duration_ms,
        report_cas_hash: report_cas_hash.to_string(),
        timestamp: Utc::now(),
    };

    publish_event(event).await
}

/// Log individual finding creation.
pub async fn log_finding_created(
    finding_id: &str,
    sweep_id: &str,
    rule_id: &str,
    severity: &str,
    confidence: f32,
) -> Result<EventId> {
    let event = BugHuntEvent::FindingCreated {
        finding_id: finding_id.to_string(),
        sweep_id: sweep_id.to_string(),
        rule_id: rule_id.to_string(),
        severity: severity.to_string(),
        confidence,
        timestamp: Utc::now(),
    };

    publish_event(event).await
}

/// Log a fix application attempt.
pub async fn log_fix_applied(
    finding_id: &str,
    sweep_id: &str,
    success: bool,
    error: Option<&str>,
) -> Result<EventId> {
    let event = BugHuntEvent::FixApplied {
        finding_id: finding_id.to_string(),
        sweep_id: sweep_id.to_string(),
        success,
        error_message: error.map(|e| e.to_string()),
        timestamp: Utc::now(),
    };

    publish_event(event).await
}

/// Log a new pattern added to KDB.
pub async fn log_pattern_added(
    pattern_id: &str,
    source: &str,
    rule_id: &str,
    votes: usize,
) -> Result<EventId> {
    let event = BugHuntEvent::PatternAdded {
        pattern_id: pattern_id.to_string(),
        source: source.to_string(),
        rule_id: rule_id.to_string(),
        votes,
        timestamp: Utc::now(),
    };

    publish_event(event).await
}

/// Publish an event to Universe.
///
/// In production, this sends the event to a Universe sink (e.g., a
/// ledger microservice or Kafka topic).
async fn publish_event(event: BugHuntEvent) -> Result<EventId> {
    let event_id = format!("event-{}", Uuid::new_v4());
    let event_json = serialize_event(&event)?;

    info!(
        "Publishing Universe event: {} -> {}",
        event_id,
        serde_json::to_string(&event_json)?
    );

    // In production:
    // 1. Connect to Universe event sink via Conduit IPC
    // 2. Send serialized event with timestamp
    // 3. Receive back the event ID
    // 4. Store locally in case of network failure

    Ok(event_id)
}

/// Serialize a BugHuntEvent to JSON for Universe.
fn serialize_event(event: &BugHuntEvent) -> Result<Value> {
    match event {
        BugHuntEvent::SweepStarted {
            sweep_id,
            repository,
            trigger,
            timestamp,
        } => Ok(json!({
            "event_type": "sweep_started",
            "sweep_id": sweep_id,
            "repository": repository,
            "trigger": trigger,
            "timestamp": timestamp.to_rfc3339(),
        })),
        BugHuntEvent::SweepCompleted {
            sweep_id,
            repository,
            files_scanned,
            issues_found,
            critical_count,
            high_count,
            duration_ms,
            report_cas_hash,
            timestamp,
        } => Ok(json!({
            "event_type": "sweep_completed",
            "sweep_id": sweep_id,
            "repository": repository,
            "files_scanned": files_scanned,
            "issues_found": issues_found,
            "critical_count": critical_count,
            "high_count": high_count,
            "duration_ms": duration_ms,
            "report_cas_hash": report_cas_hash,
            "timestamp": timestamp.to_rfc3339(),
        })),
        BugHuntEvent::FindingCreated {
            finding_id,
            sweep_id,
            rule_id,
            severity,
            confidence,
            timestamp,
        } => Ok(json!({
            "event_type": "finding_created",
            "finding_id": finding_id,
            "sweep_id": sweep_id,
            "rule_id": rule_id,
            "severity": severity,
            "confidence": confidence,
            "timestamp": timestamp.to_rfc3339(),
        })),
        BugHuntEvent::FixApplied {
            finding_id,
            sweep_id,
            success,
            error_message,
            timestamp,
        } => Ok(json!({
            "event_type": "fix_applied",
            "finding_id": finding_id,
            "sweep_id": sweep_id,
            "success": success,
            "error_message": error_message,
            "timestamp": timestamp.to_rfc3339(),
        })),
        BugHuntEvent::PatternAdded {
            pattern_id,
            source,
            rule_id,
            votes,
            timestamp,
        } => Ok(json!({
            "event_type": "pattern_added",
            "pattern_id": pattern_id,
            "source": source,
            "rule_id": rule_id,
            "votes": votes,
            "timestamp": timestamp.to_rfc3339(),
        })),
    }
}

/// Query Universe for events related to a sweep.
///
/// This enables time-travel debugging: inspect the quality of code at any point.
pub async fn query_sweep_history(repository: &str, limit: usize) -> Result<Vec<Value>> {
    info!("Querying Universe for sweep history: repo={}, limit={}", repository, limit);

    // In production:
    // 1. Connect to Universe query interface
    // 2. Query events where event_type = "sweep_completed" and repository = repo
    // 3. Return last N results ordered by timestamp DESC

    Ok(vec![])
}
