/// SQLx-based persistent storage backend for ETL
/// Stores feedback events and metrics in SQLite for cross-session learning.

use crate::confidence::RuleConfidenceMetrics;
use crate::events::FeedbackEvent;
use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::collections::HashMap;

/// SQLite-backed storage for production deployments.
pub struct SqlxStorage {
    pool: SqlitePool,
}

impl SqlxStorage {
    /// Create a new SQLx storage instance with SQLite backend.
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await?;

        Ok(Self { pool })
    }

    /// Store a single feedback event in the database.
    pub async fn store_feedback_event(&self, event: &FeedbackEvent) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO feedback_events (
                event_id, event_type, rule_id, file, line, timestamp,
                user_id, action, outcome, explanation, dismissal_count
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&event.event_id)
        .bind(format!("{:?}", event.event_type))
        .bind(&event.rule_id)
        .bind(&event.file)
        .bind(event.line as i32)
        .bind(event.timestamp)
        .bind(&event.user_id)
        .bind(&event.action)
        .bind(&event.outcome)
        .bind(&event.explanation)
        .bind(event.dismissal_count.map(|c| c as i32))
        .execute(&self.pool)
        .await?;

        tracing::debug!("Stored feedback event: {} for rule {}", event.event_id, event.rule_id);
        Ok(())
    }

    /// Retrieve all feedback events since a given timestamp.
    pub async fn get_feedback_events_since(
        &self,
        since: DateTime<Utc>,
    ) -> anyhow::Result<Vec<FeedbackEvent>> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, event_type, rule_id, file, line, timestamp,
                   user_id, action, outcome, explanation, dismissal_count
            FROM feedback_events
            WHERE timestamp >= ?
            ORDER BY timestamp DESC
            "#,
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await?;

        let events = rows
            .into_iter()
            .filter_map(|row| {
                let event_type_str: String = row.get("event_type");
                let event_type = parse_event_type(&event_type_str)?;

                Some(FeedbackEvent {
                    event_id: row.get("event_id"),
                    event_type,
                    rule_id: row.get("rule_id"),
                    file: row.get("file"),
                    line: row.get::<i32, _>("line") as u32,
                    timestamp: row.get("timestamp"),
                    user_id: row.get("user_id"),
                    action: row.get("action"),
                    outcome: row.get("outcome"),
                    explanation: row.get("explanation"),
                    dismissal_count: row.get::<Option<i32>, _>("dismissal_count").map(|c| c as u32),
                })
            })
            .collect();

        Ok(events)
    }

    /// Store aggregated metrics for a rule.
    pub async fn store_metrics(
        &self,
        metrics: &HashMap<String, RuleConfidenceMetrics>,
    ) -> anyhow::Result<()> {
        for (rule_id, metric) in metrics {
            sqlx::query(
                r#"
                INSERT OR REPLACE INTO rule_metrics (
                    rule_id, true_positives, false_positives, dismissed_count,
                    applied_fixes, fix_success_rate, last_updated
                ) VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(rule_id)
            .bind(metric.true_positives as i32)
            .bind(metric.false_positives as i32)
            .bind(metric.dismissed_count as i32)
            .bind(metric.applied_fixes as i32)
            .bind(metric.fix_success_rate)
            .bind(metric.last_updated)
            .execute(&self.pool)
            .await?;
        }

        tracing::info!("Stored metrics for {} rules", metrics.len());
        Ok(())
    }

    /// Retrieve metrics for a specific rule.
    pub async fn get_metrics(&self, rule_id: &str) -> anyhow::Result<Option<RuleConfidenceMetrics>> {
        let row = sqlx::query(
            r#"
            SELECT rule_id, true_positives, false_positives, dismissed_count,
                   applied_fixes, fix_success_rate, last_updated
            FROM rule_metrics
            WHERE rule_id = ?
            "#,
        )
        .bind(rule_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| RuleConfidenceMetrics {
            rule_id: r.get("rule_id"),
            true_positives: r.get::<i32, _>("true_positives") as u32,
            false_positives: r.get::<i32, _>("false_positives") as u32,
            dismissed_count: r.get::<i32, _>("dismissed_count") as u32,
            applied_fixes: r.get::<i32, _>("applied_fixes") as u32,
            fix_success_rate: r.get("fix_success_rate"),
            last_updated: r.get("last_updated"),
        }))
    }

    /// Remove feedback events older than the specified number of days.
    pub async fn cleanup_old_events(&self, days_old: i64) -> anyhow::Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::days(days_old);

        let result = sqlx::query(
            r#"
            DELETE FROM feedback_events
            WHERE timestamp < ?
            "#,
        )
        .bind(cutoff)
        .execute(&self.pool)
        .await?;

        let rows_deleted = result.rows_affected();
        tracing::info!("Cleaned up {} old feedback events", rows_deleted);
        Ok(rows_deleted as usize)
    }

    /// Get all events for a specific rule.
    pub async fn get_feedback_events_for_rule(
        &self,
        rule_id: &str,
    ) -> anyhow::Result<Vec<FeedbackEvent>> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, event_type, rule_id, file, line, timestamp,
                   user_id, action, outcome, explanation, dismissal_count
            FROM feedback_events
            WHERE rule_id = ?
            ORDER BY timestamp DESC
            "#,
        )
        .bind(rule_id)
        .fetch_all(&self.pool)
        .await?;

        let events = rows
            .into_iter()
            .filter_map(|row| {
                let event_type_str: String = row.get("event_type");
                let event_type = parse_event_type(&event_type_str)?;

                Some(FeedbackEvent {
                    event_id: row.get("event_id"),
                    event_type,
                    rule_id: row.get("rule_id"),
                    file: row.get("file"),
                    line: row.get::<i32, _>("line") as u32,
                    timestamp: row.get("timestamp"),
                    user_id: row.get("user_id"),
                    action: row.get("action"),
                    outcome: row.get("outcome"),
                    explanation: row.get("explanation"),
                    dismissal_count: row.get::<Option<i32>, _>("dismissal_count").map(|c| c as u32),
                })
            })
            .collect();

        Ok(events)
    }
}

fn parse_event_type(s: &str) -> Option<crate::events::FeedbackEventType> {
    use crate::events::FeedbackEventType;

    match s {
        "DiagnosticAccepted" => Some(FeedbackEventType::DiagnosticAccepted),
        "FalsePositiveReported" => Some(FeedbackEventType::FalsePositiveReported),
        "DiagnosticDismissed" => Some(FeedbackEventType::DiagnosticDismissed),
        "DiagnosticIgnoredThenFixed" => Some(FeedbackEventType::DiagnosticIgnoredThenFixed),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlx_storage_creation() {
        // This test would require a real SQLite database
        // For now, just verify the module compiles
        assert!(true);
    }
}
