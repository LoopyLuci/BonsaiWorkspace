//! Audit-log CLI — inspect a Universe event ledger on disk.
//!
//! Usage:
//!   audit_log_cli <db-path> record <target> <summary>
//!   audit_log_cli <db-path> timeline [limit]
//!   audit_log_cli <db-path> snapshot [label]
//!   audit_log_cli <db-path> snapshots [limit]
//!   audit_log_cli <db-path> status

use std::env;
use std::path::PathBuf;

use audit_log::{AuditLog, EventSource, TimelineFilter};

#[tokio::main]
async fn main() -> audit_log::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "usage: {} <db-path> <record <target> <summary>|timeline [limit]|snapshot [label]|snapshots [limit]|status>",
            args.first().map(String::as_str).unwrap_or("audit_log_cli")
        );
        std::process::exit(1);
    }

    let db_path = PathBuf::from(&args[1]);
    let log = AuditLog::open(&db_path, "cli").await?;

    match args[2].as_str() {
        "record" => {
            let target = args.get(3).cloned().unwrap_or_else(|| "unknown".into());
            let summary = args.get(4).cloned().unwrap_or_else(|| "manual entry".into());
            log.record_file_change(
                &target,
                None,
                None,
                EventSource::User { peer_id: "cli".into() },
            );
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            println!("recorded: {summary} ({target})");
        }
        "timeline" => {
            let limit = args.get(3).and_then(|s| s.parse().ok());
            let events = log
                .store
                .query_timeline(TimelineFilter { limit, ..Default::default() })
                .await?;
            for e in events {
                println!("[{}] {} — {} ({})", e.timestamp_ns, e.category, e.summary, e.target);
            }
        }
        "snapshot" => {
            let label = args.get(3).cloned();
            let snap = log.snapshots.take_snapshot(label, "cli").await?;
            println!("snapshot created: {}", snap.snapshot_id);
        }
        "snapshots" => {
            let limit = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(20);
            let snaps = log.store.list_snapshots(limit).await?;
            for s in snaps {
                println!(
                    "{} @ {} — {} events",
                    s.snapshot_id,
                    s.timestamp_ns,
                    s.event_count_at_creation
                );
            }
        }
        "status" => {
            let state = log.health().await;
            println!("[{}] {}", state.timestamp, state.status);
        }
        other => {
            eprintln!("unknown command: {other}");
            std::process::exit(1);
        }
    }

    Ok(())
}
