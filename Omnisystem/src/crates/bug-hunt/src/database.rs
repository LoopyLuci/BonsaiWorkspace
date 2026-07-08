/// Persistent findings database using SQLite.
/// 
/// Stores findings locally for querying, tracking status changes, and
/// correlating with Survival System events.

use anyhow::{Context, Result};
use chrono::Utc;
use log::{debug, info};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use uuid::Uuid;

use crate::Finding;

/// Path to the findings database.
pub fn findings_db_path() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .context("Could not find cache directory")?
        .join("bonsai/bug-hunt");
    std::fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir.join("findings.db"))
}

/// Initialize the findings database schema.
pub fn init_db() -> Result<Connection> {
    let db_path = findings_db_path()?;
    let conn = Connection::open(&db_path)?;

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS findings (
            id TEXT PRIMARY KEY,
            sweep_id TEXT NOT NULL,
            file_path TEXT NOT NULL,
            line_start INTEGER NOT NULL,
            line_end INTEGER NOT NULL,
            column_start INTEGER,
            column_end INTEGER,
            rule_id TEXT NOT NULL,
            severity TEXT NOT NULL,
            message TEXT NOT NULL,
            suggestion TEXT,
            suggested_diff TEXT,
            confidence REAL NOT NULL,
            analyzer TEXT NOT NULL,
            status TEXT NOT NULL,
            first_seen TEXT NOT NULL,
            last_seen TEXT NOT NULL,
            fixed_by_commit TEXT,
            tags TEXT, -- JSON array
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_findings_sweep_id ON findings(sweep_id);
        CREATE INDEX IF NOT EXISTS idx_findings_severity ON findings(severity);
        CREATE INDEX IF NOT EXISTS idx_findings_status ON findings(status);
        CREATE INDEX IF NOT EXISTS idx_findings_file_path ON findings(file_path);
        CREATE INDEX IF NOT EXISTS idx_findings_rule_id ON findings(rule_id);

        CREATE TABLE IF NOT EXISTS scan_history (
            sweep_id TEXT PRIMARY KEY,
            repository TEXT NOT NULL,
            trigger TEXT NOT NULL,
            files_scanned INTEGER NOT NULL,
            issues_found INTEGER NOT NULL,
            duration_ms INTEGER NOT NULL,
            report_cas_hash TEXT NOT NULL,
            started_at TEXT NOT NULL,
            completed_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_scans_repository ON scan_history(repository);
        CREATE INDEX IF NOT EXISTS idx_scans_completed_at ON scan_history(completed_at);

        CREATE TABLE IF NOT EXISTS fix_history (
            id TEXT PRIMARY KEY,
            finding_id TEXT NOT NULL,
            applied_at TEXT NOT NULL,
            success INTEGER NOT NULL,
            error_message TEXT,
            diff TEXT NOT NULL,
            FOREIGN KEY(finding_id) REFERENCES findings(id)
        );

        CREATE INDEX IF NOT EXISTS idx_fixes_finding_id ON fix_history(finding_id);
        CREATE INDEX IF NOT EXISTS idx_fixes_applied_at ON fix_history(applied_at);
        "#,
    )?;

    info!("Findings database initialized at {:?}", db_path);
    Ok(conn)
}

/// Insert a finding into the database.
pub fn insert_finding(conn: &Connection, finding: &Finding, sweep_id: &str) -> Result<()> {
    let tags_json = serde_json::to_string(&finding.tags)?;

    conn.execute(
        r#"
        INSERT INTO findings (
            id, sweep_id, file_path, line_start, line_end, column_start, column_end,
            rule_id, severity, message, suggestion, suggested_diff, confidence,
            analyzer, status, first_seen, last_seen, fixed_by_commit, tags, created_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
        "#,
        params![
            finding.id.to_string(),
            sweep_id,
            finding.file_path.to_string_lossy().to_string(),
            finding.line_start,
            finding.line_end,
            finding.column_start,
            finding.column_end,
            finding.rule_id,
            format!("{:?}", finding.severity),
            finding.message,
            finding.suggestion,
            finding.suggested_diff,
            finding.confidence,
            finding.analyzer,
            format!("{:?}", finding.status),
            finding.first_seen.to_rfc3339(),
            finding.last_seen.to_rfc3339(),
            finding.fixed_by_commit,
            tags_json,
            Utc::now().to_rfc3339(),
        ],
    )?;

    debug!("Inserted finding: {} in sweep {}", finding.id, sweep_id);
    Ok(())
}

/// Query findings by severity.
pub fn query_by_severity(conn: &Connection, severity: &str) -> Result<Vec<Finding>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM findings WHERE severity = ?1 ORDER BY confidence DESC LIMIT 100",
    )?;

    let findings = stmt
        .query_map(params![severity], |row| {
            Ok((
                row.get::<_, String>(0)?,  // id
                row.get::<_, String>(2)?,  // file_path
                row.get::<_, u32>(3)?,     // line_start
                row.get::<_, u32>(4)?,     // line_end
                row.get::<_, String>(8)?,  // severity
                row.get::<_, String>(9)?,  // message
                row.get::<_, f32>(12)?,    // confidence
                row.get::<_, String>(13)?, // analyzer
            ))
        })?
        .filter_map(|row| row.ok())
        .map(|(id, file_path, line_start, line_end, _severity, message, confidence, analyzer)| {
            Finding {
                id: uuid::Uuid::parse_str(&id).unwrap(),
                file_path: file_path.into(),
                line_start: line_start as usize,
                line_end: line_end as usize,
                column_start: None,
                column_end: None,
                rule_id: "unknown".to_string(),
                severity: crate::Severity::High, // Placeholder
                message,
                suggestion: None,
                suggested_diff: None,
                confidence,
                analyzer,
                first_seen: Utc::now(),
                last_seen: Utc::now(),
                status: crate::FindingStatus::Open,
                fixed_by_commit: None,
                tags: vec![],
            }
        })
        .collect();

    Ok(findings)
}

/// Record a scan in the history table.
pub fn record_scan(
    conn: &Connection,
    sweep_id: &str,
    repository: &str,
    trigger: &str,
    files_scanned: usize,
    issues_found: usize,
    duration_ms: u64,
    report_cas_hash: &str,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        INSERT INTO scan_history (
            sweep_id, repository, trigger, files_scanned, issues_found,
            duration_ms, report_cas_hash, started_at, completed_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#,
        params![
            sweep_id,
            repository,
            trigger,
            files_scanned,
            issues_found,
            duration_ms,
            report_cas_hash,
            now,
            now,
        ],
    )?;

    info!(
        "Recorded scan {} in history: {} issues found",
        sweep_id, issues_found
    );
    Ok(())
}

/// Record a fix attempt in the database.
pub fn record_fix(
    conn: &Connection,
    finding_id: &str,
    success: bool,
    error: Option<&str>,
    diff: &str,
) -> Result<()> {
    let fix_id = format!("fix-{}", Uuid::new_v4());

    conn.execute(
        r#"
        INSERT INTO fix_history (id, finding_id, applied_at, success, error_message, diff)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![
            fix_id,
            finding_id,
            Utc::now().to_rfc3339(),
            if success { 1 } else { 0 },
            error,
            diff,
        ],
    )?;

    debug!("Recorded fix for finding {}: success={}", finding_id, success);
    Ok(())
}

/// Get the most recent scan for a repository.
pub fn get_last_scan(conn: &Connection, repository: &str) -> Result<Option<(String, u64)>> {
    let result = conn
        .query_row(
            "SELECT sweep_id, completed_at FROM scan_history WHERE repository = ?1 ORDER BY completed_at DESC LIMIT 1",
            params![repository],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                ))
            },
        )
        .optional()?;

    Ok(result.map(|(sweep_id, _ts)| {
        (sweep_id, Utc::now().timestamp_millis() as u64)
    }))
}

/// Clear old findings from the database (older than N days).
pub fn prune_old_findings(conn: &Connection, days: i64) -> Result<usize> {
    let cutoff = Utc::now() - chrono::Duration::days(days);

    let affected = conn.execute(
        "DELETE FROM findings WHERE created_at < ?1",
        params![cutoff.to_rfc3339()],
    )?;

    info!("Pruned {} old findings from database", affected);
    Ok(affected)
}
