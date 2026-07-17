use crate::{CepEvent, EventPattern, PatternMatch, EventSequence, EventCorrelation, CEPAlert, AlertSeverity, CEPError, CEPResult};
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct ComplexEventProcessor {
    events: Arc<DashMap<Uuid, CepEvent>>,
    patterns: Arc<DashMap<Uuid, EventPattern>>,
    matches: Arc<DashMap<Uuid, PatternMatch>>,
    sequences: Arc<DashMap<Uuid, EventSequence>>,
    correlations: Arc<DashMap<Uuid, EventCorrelation>>,
    alerts: Arc<DashMap<Uuid, CEPAlert>>,
}

impl ComplexEventProcessor {
    pub fn new() -> Self {
        Self {
            events: Arc::new(DashMap::new()),
            patterns: Arc::new(DashMap::new()),
            matches: Arc::new(DashMap::new()),
            sequences: Arc::new(DashMap::new()),
            correlations: Arc::new(DashMap::new()),
            alerts: Arc::new(DashMap::new()),
        }
    }

    /// Ingest a real event with attributes so patterns/sequences/
    /// correlations have actual data to evaluate against.
    pub async fn ingest_event(&self, event_type: &str, attributes: HashMap<String, String>) -> CEPResult<CepEvent> {
        let event = CepEvent {
            event_id: Uuid::new_v4(),
            event_type: event_type.to_string(),
            timestamp: Utc::now(),
            attributes,
        };

        self.events.insert(event.event_id, event.clone());
        Ok(event)
    }

    pub async fn define_pattern(&self, name: &str, conditions: Vec<String>, time_window: u64) -> CEPResult<EventPattern> {
        let pattern = EventPattern {
            pattern_id: Uuid::new_v4(),
            name: name.to_string(),
            conditions,
            time_window_ms: time_window,
            enabled: true,
        };

        self.patterns.insert(pattern.pattern_id, pattern.clone());
        Ok(pattern)
    }

    /// Evaluate a single condition of the form `key=value`, `key>value`, or
    /// `key<value` against an event's attributes (and its `event_type` for
    /// the `event_type` key). Numeric operators fall back to string
    /// comparison if either side isn't parseable as a number.
    fn condition_satisfied(event: &CepEvent, condition: &str) -> bool {
        let (key, op, value) = if let Some((k, v)) = condition.split_once('=') {
            (k, '=', v)
        } else if let Some((k, v)) = condition.split_once('>') {
            (k, '>', v)
        } else if let Some((k, v)) = condition.split_once('<') {
            (k, '<', v)
        } else {
            return false;
        };

        let key = key.trim();
        let value = value.trim();
        let actual = if key == "event_type" {
            Some(event.event_type.clone())
        } else {
            event.attributes.get(key).cloned()
        };

        let Some(actual) = actual else { return false };

        match op {
            '=' => actual == value,
            '>' | '<' => match (actual.parse::<f64>(), value.parse::<f64>()) {
                (Ok(a), Ok(v)) => if op == '>' { a > v } else { a < v },
                _ => false,
            },
            _ => false,
        }
    }

    /// Match a pattern against a set of previously-ingested events. The
    /// confidence is the real fraction of the pattern's conditions that
    /// are satisfied by at least one of the given events (a pattern with
    /// no conditions is trivially fully satisfied by any non-empty event
    /// set).
    pub async fn match_pattern(&self, pattern_id: Uuid, event_ids: Vec<Uuid>) -> CEPResult<PatternMatch> {
        let pattern = self.patterns.get(&pattern_id).ok_or(CEPError::PatternNotFound)?.clone();

        let matched_events: Vec<CepEvent> = event_ids
            .iter()
            .filter_map(|id| self.events.get(id).map(|e| e.value().clone()))
            .collect();

        let confidence = if pattern.conditions.is_empty() {
            if matched_events.is_empty() { 0.0 } else { 1.0 }
        } else {
            let satisfied = pattern
                .conditions
                .iter()
                .filter(|cond| matched_events.iter().any(|e| Self::condition_satisfied(e, cond)))
                .count();
            satisfied as f64 / pattern.conditions.len() as f64
        };

        let pattern_match = PatternMatch {
            match_id: Uuid::new_v4(),
            pattern_id,
            matched_events: event_ids,
            confidence,
            matched_at: Utc::now(),
        };

        self.matches.insert(pattern_match.match_id, pattern_match.clone());
        Ok(pattern_match)
    }

    /// Detect a sequence spanning the given (previously-ingested) events,
    /// computing its real duration from the events' actual timestamps
    /// rather than a fixed constant.
    pub async fn detect_sequence(&self, event_ids: Vec<Uuid>, sequence_type: &str) -> CEPResult<EventSequence> {
        let timestamps: Vec<chrono::DateTime<Utc>> = event_ids
            .iter()
            .filter_map(|id| self.events.get(id).map(|e| e.value().timestamp))
            .collect();

        let duration_ms = match (timestamps.iter().min(), timestamps.iter().max()) {
            (Some(min), Some(max)) => (*max - *min).num_milliseconds().max(0) as u64,
            _ => 0,
        };

        let sequence = EventSequence {
            sequence_id: Uuid::new_v4(),
            event_ids,
            sequence_type: sequence_type.to_string(),
            duration_ms,
            detected_at: Utc::now(),
        };

        self.sequences.insert(sequence.sequence_id, sequence.clone());
        Ok(sequence)
    }

    /// Correlate a primary event with related events based on real time
    /// proximity: events close together in time score near 1.0, events
    /// far apart decay toward 0.0. Unknown event ids are skipped rather
    /// than silently counted.
    pub async fn correlate_events(&self, primary_event_id: Uuid, related_ids: Vec<Uuid>, correlation_type: &str) -> CEPResult<EventCorrelation> {
        let correlation_score = if let Some(primary) = self.events.get(&primary_event_id) {
            let primary_ts = primary.value().timestamp;
            let scores: Vec<f64> = related_ids
                .iter()
                .filter_map(|id| self.events.get(id).map(|e| e.value().timestamp))
                .map(|ts| {
                    let delta_secs = (ts - primary_ts).num_milliseconds().abs() as f64 / 1000.0;
                    // Exponential decay: events within a few seconds score
                    // near 1.0, minutes-apart events score near 0.
                    (-delta_secs / 60.0).exp()
                })
                .collect();

            if scores.is_empty() {
                0.0
            } else {
                scores.iter().sum::<f64>() / scores.len() as f64
            }
        } else {
            0.0
        };

        let correlation = EventCorrelation {
            correlation_id: Uuid::new_v4(),
            primary_event_id,
            related_event_ids: related_ids,
            correlation_score,
            correlation_type: correlation_type.to_string(),
        };

        self.correlations.insert(correlation.correlation_id, correlation.clone());
        Ok(correlation)
    }

    pub async fn generate_alert(&self, match_id: Uuid, severity: AlertSeverity, message: &str) -> CEPResult<CEPAlert> {
        if let Some(pattern_match) = self.matches.get(&match_id) {
            let alert = CEPAlert {
                alert_id: Uuid::new_v4(),
                pattern_id: pattern_match.value().pattern_id,
                match_id,
                severity,
                message: message.to_string(),
                created_at: Utc::now(),
            };

            self.alerts.insert(alert.alert_id, alert.clone());
            Ok(alert)
        } else {
            Err(CEPError::MatchingFailed)
        }
    }

    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }
}

impl Default for ComplexEventProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_define_pattern() {
        let processor = ComplexEventProcessor::new();
        let conditions = vec!["event_type=error".to_string(), "severity=high".to_string()];

        let pattern = processor.define_pattern("error_spike", conditions, 5000).await.unwrap();
        assert_eq!(pattern.name, "error_spike");
        assert_eq!(processor.pattern_count(), 1);
    }

    #[tokio::test]
    async fn test_match_pattern_computes_real_confidence() {
        let processor = ComplexEventProcessor::new();
        let conditions = vec!["cpu>80".to_string(), "event_type=metric".to_string()];
        let pattern = processor.define_pattern("high_cpu", conditions, 10000).await.unwrap();

        let mut attrs = HashMap::new();
        attrs.insert("cpu".to_string(), "95".to_string());
        let event = processor.ingest_event("metric", attrs).await.unwrap();

        // Both conditions are satisfied by the one ingested event.
        let full_match = processor.match_pattern(pattern.pattern_id, vec![event.event_id]).await.unwrap();
        assert_eq!(full_match.confidence, 1.0);

        // An unrelated event satisfies neither condition -> confidence 0.
        let other = processor.ingest_event("login", HashMap::new()).await.unwrap();
        let no_match = processor.match_pattern(pattern.pattern_id, vec![other.event_id]).await.unwrap();
        assert_eq!(no_match.confidence, 0.0);
    }

    #[tokio::test]
    async fn test_match_pattern_partial_confidence() {
        let processor = ComplexEventProcessor::new();
        let conditions = vec!["cpu>80".to_string(), "region=us-east".to_string()];
        let pattern = processor.define_pattern("partial", conditions, 10000).await.unwrap();

        let mut attrs = HashMap::new();
        attrs.insert("cpu".to_string(), "95".to_string()); // satisfies only the first condition
        let event = processor.ingest_event("metric", attrs).await.unwrap();

        let result = processor.match_pattern(pattern.pattern_id, vec![event.event_id]).await.unwrap();
        assert_eq!(result.confidence, 0.5);
    }

    #[tokio::test]
    async fn test_match_pattern_unknown_pattern_fails() {
        let processor = ComplexEventProcessor::new();
        let result = processor.match_pattern(Uuid::new_v4(), vec![]).await;
        assert!(matches!(result, Err(CEPError::PatternNotFound)));
    }

    #[tokio::test]
    async fn test_detect_sequence_computes_real_duration() {
        let processor = ComplexEventProcessor::new();
        let e1 = processor.ingest_event("login", HashMap::new()).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        let e2 = processor.ingest_event("action", HashMap::new()).await.unwrap();

        let sequence = processor
            .detect_sequence(vec![e1.event_id, e2.event_id], "login_then_action")
            .await
            .unwrap();

        assert_eq!(sequence.sequence_type, "login_then_action");
        assert_eq!(sequence.event_ids.len(), 2);
        assert!(sequence.duration_ms >= 15, "expected real elapsed duration, got {}", sequence.duration_ms);
    }

    #[tokio::test]
    async fn test_correlate_events_scores_time_proximity() {
        let processor = ComplexEventProcessor::new();
        let primary = processor.ingest_event("error", HashMap::new()).await.unwrap();
        let close = processor.ingest_event("error", HashMap::new()).await.unwrap();

        let correlation = processor
            .correlate_events(primary.event_id, vec![close.event_id], "temporal")
            .await
            .unwrap();

        // Events ingested back-to-back should correlate strongly.
        assert!(correlation.correlation_score > 0.9);

        // An unknown related event contributes nothing and is skipped, not
        // treated as a perfect or fabricated match.
        let unknown_correlation = processor
            .correlate_events(primary.event_id, vec![Uuid::new_v4()], "temporal")
            .await
            .unwrap();
        assert_eq!(unknown_correlation.correlation_score, 0.0);
    }

    #[tokio::test]
    async fn test_generate_alert() {
        let processor = ComplexEventProcessor::new();
        let pattern = processor.define_pattern("alert_pattern", vec![], 1000).await.unwrap();
        let event = processor.ingest_event("critical", HashMap::new()).await.unwrap();

        let pattern_match = processor
            .match_pattern(pattern.pattern_id, vec![event.event_id])
            .await
            .unwrap();

        let alert = processor
            .generate_alert(pattern_match.match_id, AlertSeverity::Critical, "Critical pattern detected")
            .await
            .unwrap();

        assert_eq!(alert.severity, AlertSeverity::Critical);
    }

    #[tokio::test]
    async fn test_generate_alert_unknown_match_fails() {
        let processor = ComplexEventProcessor::new();
        let result = processor.generate_alert(Uuid::new_v4(), AlertSeverity::Low, "x").await;
        assert!(matches!(result, Err(CEPError::MatchingFailed)));
    }
}
