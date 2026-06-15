use crate::{EventPattern, PatternMatch, EventSequence, EventCorrelation, CEPAlert, AlertSeverity, CEPError, CEPResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct ComplexEventProcessor {
    patterns: Arc<DashMap<Uuid, EventPattern>>,
    matches: Arc<DashMap<Uuid, PatternMatch>>,
    sequences: Arc<DashMap<Uuid, EventSequence>>,
    correlations: Arc<DashMap<Uuid, EventCorrelation>>,
    alerts: Arc<DashMap<Uuid, CEPAlert>>,
}

impl ComplexEventProcessor {
    pub fn new() -> Self {
        Self {
            patterns: Arc::new(DashMap::new()),
            matches: Arc::new(DashMap::new()),
            sequences: Arc::new(DashMap::new()),
            correlations: Arc::new(DashMap::new()),
            alerts: Arc::new(DashMap::new()),
        }
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

    pub async fn match_pattern(&self, pattern_id: Uuid, event_ids: Vec<Uuid>) -> CEPResult<PatternMatch> {
        if self.patterns.get(&pattern_id).is_none() {
            return Err(CEPError::PatternNotFound);
        }

        let pattern_match = PatternMatch {
            match_id: Uuid::new_v4(),
            pattern_id,
            matched_events: event_ids,
            confidence: 0.95,
            matched_at: Utc::now(),
        };

        self.matches.insert(pattern_match.match_id, pattern_match.clone());
        Ok(pattern_match)
    }

    pub async fn detect_sequence(&self, event_ids: Vec<Uuid>, sequence_type: &str) -> CEPResult<EventSequence> {
        let sequence = EventSequence {
            sequence_id: Uuid::new_v4(),
            event_ids,
            sequence_type: sequence_type.to_string(),
            duration_ms: 1000,
            detected_at: Utc::now(),
        };

        self.sequences.insert(sequence.sequence_id, sequence.clone());
        Ok(sequence)
    }

    pub async fn correlate_events(&self, primary_event_id: Uuid, related_ids: Vec<Uuid>, correlation_type: &str) -> CEPResult<EventCorrelation> {
        let correlation = EventCorrelation {
            correlation_id: Uuid::new_v4(),
            primary_event_id,
            related_event_ids: related_ids,
            correlation_score: 0.85,
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
    async fn test_match_pattern() {
        let processor = ComplexEventProcessor::new();
        let conditions = vec!["cpu>80".to_string()];
        let pattern = processor.define_pattern("high_cpu", conditions, 10000).await.unwrap();

        let event_ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let pattern_match = processor.match_pattern(pattern.pattern_id, event_ids).await.unwrap();

        assert_eq!(pattern_match.confidence, 0.95);
    }

    #[tokio::test]
    async fn test_detect_sequence() {
        let processor = ComplexEventProcessor::new();
        let event_ids = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];

        let sequence = processor.detect_sequence(event_ids.clone(), "login_then_action").await.unwrap();
        assert_eq!(sequence.sequence_type, "login_then_action");
        assert_eq!(sequence.event_ids.len(), 3);
    }

    #[tokio::test]
    async fn test_generate_alert() {
        let processor = ComplexEventProcessor::new();
        let pattern = processor.define_pattern("alert_pattern", vec![], 1000).await.unwrap();

        let pattern_match = processor
            .match_pattern(pattern.pattern_id, vec![Uuid::new_v4()])
            .await
            .unwrap();

        let alert = processor
            .generate_alert(pattern_match.match_id, AlertSeverity::Critical, "Critical pattern detected")
            .await
            .unwrap();

        assert_eq!(alert.severity, AlertSeverity::Critical);
    }
}
