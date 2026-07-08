//! Survival System — crash-report ingestion.
//!
//! One of the Survival System's tools (see `super` for the others: `kb`,
//! `bug_db`, `bug_hunter`, `sns_bridge`, `daemon`). Folds `crash_reporter`'s
//! already-captured backend panics and frontend JS errors into the Bug
//! Database. No new capture mechanism — `crash_reporter` already writes one
//! JSON file per crash (`{timestamp}_{kind}.json`) and exposes them via
//! `list_crash_reports()`; this just tracks a high-water mark so repeated
//! polls don't re-ingest the same file.

use super::bug_db::DiscoveredBug;

pub struct CrashIngestCursor {
    /// Filenames sort lexicographically by timestamp (see `crash_reporter`'s
    /// naming), so the lexicographically-greatest `timestamp` seen so far is
    /// enough to identify "already ingested" without storing every filename.
    last_seen_timestamp: Option<String>,
}

impl CrashIngestCursor {
    pub fn new() -> Self {
        Self { last_seen_timestamp: None }
    }

    /// Returns bugs for every crash report newer than the last poll,
    /// advancing the cursor past them.
    pub fn poll_new(&mut self) -> Vec<DiscoveredBug> {
        let Ok(reports) = crate::crash_reporter::list_crash_reports() else { return Vec::new() };
        let mut bugs = Vec::new();
        let mut newest = self.last_seen_timestamp.clone();

        for report in &reports {
            let Some(ts) = report.get("timestamp").and_then(|v| v.as_str()) else { continue };
            if let Some(last) = &self.last_seen_timestamp {
                if ts <= last.as_str() {
                    continue;
                }
            }
            if newest.as_deref().map(|n| ts > n).unwrap_or(true) {
                newest = Some(ts.to_string());
            }

            let kind = report.get("kind").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
            let message = report.get("message").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if message.is_empty() {
                continue;
            }
            let file_path = report
                .get("location")
                .and_then(|v| v.as_str())
                .map(|loc| loc.split(':').next().unwrap_or(loc).to_string());

            bugs.push(DiscoveredBug {
                source: "runtime".to_string(),
                severity: "error".to_string(),
                title: format!("{kind}: {}", message.lines().next().unwrap_or(&message)).chars().take(200).collect(),
                message,
                file_path,
                line_number: None,
            });
        }

        self.last_seen_timestamp = newest;
        bugs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_starts_with_no_high_water_mark() {
        let cursor = CrashIngestCursor::new();
        assert!(cursor.last_seen_timestamp.is_none());
    }
}
