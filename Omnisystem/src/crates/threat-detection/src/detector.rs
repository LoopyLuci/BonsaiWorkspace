use crate::{SecurityEvent, EventType, Severity, AnomalyDetection, ThreatIncident, IncidentStatus, CorrelatedEvents, ThreatError, ThreatResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Occurrences of a signature within this many seconds are considered part
/// of the same burst when computing a frequency-based anomaly score.
const ANOMALY_WINDOW_SECONDS: i64 = 60;
/// A signature occurring at least this many times within the window is
/// flagged as anomalous (e.g. repeated failed logins, port scans).
const ANOMALY_THRESHOLD: usize = 5;

pub struct ThreatDetector {
    events: Arc<DashMap<Uuid, SecurityEvent>>,
    anomalies: Arc<DashMap<Uuid, AnomalyDetection>>,
    incidents: Arc<DashMap<Uuid, ThreatIncident>>,
    correlations: Arc<DashMap<Uuid, CorrelatedEvents>>,
    /// Recent-occurrence timestamps per signature, used to compute a real
    /// frequency-based anomaly score instead of an arbitrary function of
    /// the signature string itself.
    signature_history: Arc<DashMap<String, Vec<DateTime<Utc>>>>,
}

impl ThreatDetector {
    pub fn new() -> Self {
        Self {
            events: Arc::new(DashMap::new()),
            anomalies: Arc::new(DashMap::new()),
            incidents: Arc::new(DashMap::new()),
            correlations: Arc::new(DashMap::new()),
            signature_history: Arc::new(DashMap::new()),
        }
    }

    pub async fn report_event(&self, event_type: EventType, source: &str, severity: Severity, description: &str) -> ThreatResult<SecurityEvent> {
        let event = SecurityEvent {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type,
            source: source.to_string(),
            severity,
            description: description.to_string(),
        };

        self.events.insert(event.event_id, event.clone());
        Ok(event)
    }

    /// Record an occurrence of `signature` (e.g. "source:event-type") and
    /// score it as anomalous if it has recurred `ANOMALY_THRESHOLD`+ times
    /// within `ANOMALY_WINDOW_SECONDS` -- a real (if simple) frequency/rate
    /// based anomaly detector, matching how brute-force/scan detection
    /// actually works, rather than a function of the string's length.
    pub async fn detect_anomaly(&self, signature: &str) -> ThreatResult<AnomalyDetection> {
        let now = Utc::now();
        let mut history = self.signature_history.entry(signature.to_string()).or_default();
        history.push(now);
        history.retain(|ts| (now - *ts).num_seconds() <= ANOMALY_WINDOW_SECONDS);

        let recent_count = history.len();
        let score = (recent_count as f64 / ANOMALY_THRESHOLD as f64).min(1.0);
        let is_anomalous = recent_count >= ANOMALY_THRESHOLD;
        drop(history);

        let anomaly = AnomalyDetection {
            anomaly_id: Uuid::new_v4(),
            event_signature: signature.to_string(),
            anomaly_score: score,
            detected_at: now,
            is_anomalous,
        };

        self.anomalies.insert(anomaly.anomaly_id, anomaly.clone());
        Ok(anomaly)
    }

    /// Create an incident and link the real events that triggered it, so
    /// `ThreatIncident::events` is actually populated instead of always
    /// being an empty vector.
    pub async fn create_incident(&self, severity: Severity, event_ids: Vec<Uuid>) -> ThreatResult<ThreatIncident> {
        let incident = ThreatIncident {
            incident_id: Uuid::new_v4(),
            events: event_ids,
            threat_score: match severity {
                Severity::Critical => 0.95,
                Severity::High => 0.75,
                Severity::Medium => 0.50,
                Severity::Low => 0.25,
            },
            status: IncidentStatus::Detected,
            created_at: Utc::now(),
            resolved_at: None,
        };

        self.incidents.insert(incident.incident_id, incident.clone());
        Ok(incident)
    }

    /// Correlate previously-reported events by real shared attributes: a
    /// higher score the more of the given events share the same source,
    /// and the pattern label reflects whether multiple distinct event
    /// types are involved (characteristic of a multi-stage attack) versus
    /// a single repeated vector, rather than a fixed 0.85/"multi_stage_attack"
    /// regardless of what was passed in.
    pub async fn correlate_events(&self, event_ids: Vec<Uuid>) -> ThreatResult<CorrelatedEvents> {
        let matched: Vec<SecurityEvent> = event_ids
            .iter()
            .filter_map(|id| self.events.get(id).map(|e| e.value().clone()))
            .collect();

        let (correlation_score, pattern) = if matched.is_empty() {
            (0.0, "no_matching_events".to_string())
        } else {
            let mut source_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
            for e in &matched {
                *source_counts.entry(e.source.as_str()).or_insert(0) += 1;
            }
            let max_shared_source = source_counts.values().copied().max().unwrap_or(0);
            let correlation_score = max_shared_source as f64 / matched.len() as f64;

            let distinct_types: std::collections::HashSet<&EventType> = matched.iter().map(|e| &e.event_type).collect();
            let pattern = if distinct_types.len() > 1 {
                "multi_stage_attack".to_string()
            } else {
                "repeated_single_vector".to_string()
            };

            (correlation_score, pattern)
        };

        let correlation = CorrelatedEvents {
            correlation_id: Uuid::new_v4(),
            event_ids,
            correlation_score,
            pattern,
            timestamp: Utc::now(),
        };

        self.correlations.insert(correlation.correlation_id, correlation.clone());
        Ok(correlation)
    }

    pub async fn resolve_incident(&self, incident_id: Uuid) -> ThreatResult<()> {
        if let Some(mut entry) = self.incidents.get_mut(&incident_id) {
            entry.status = IncidentStatus::Resolved;
            entry.resolved_at = Some(Utc::now());
        } else {
            return Err(ThreatError::IncidentNotFound);
        }

        Ok(())
    }

    pub async fn get_threat_score(&self) -> ThreatResult<f64> {
        let mut total_score = 0.0;
        let mut count = 0;

        for entry in self.anomalies.iter() {
            if entry.value().is_anomalous {
                total_score += entry.value().anomaly_score;
                count += 1;
            }
        }

        Ok(if count > 0 { total_score / count as f64 } else { 0.0 })
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

impl Default for ThreatDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_report_event() {
        let detector = ThreatDetector::new();
        let event = detector
            .report_event(EventType::UnauthorizedAccess, "server_1", Severity::High, "Unauthorized login detected")
            .await
            .unwrap();

        assert_eq!(event.severity, Severity::High);
        assert_eq!(detector.event_count(), 1);
    }

    #[tokio::test]
    async fn test_detect_anomaly_flags_bursts_not_string_length() {
        let detector = ThreatDetector::new();

        // A single occurrence of even a long signature should not be
        // anomalous under a real frequency-based detector.
        let long_signature = "a".repeat(200);
        let single = detector.detect_anomaly(&long_signature).await.unwrap();
        assert!(!single.is_anomalous, "one occurrence must not be flagged anomalous regardless of string length");

        // Repeated occurrences of a short signature within the window
        // should trip the threshold.
        let mut last = None;
        for _ in 0..ANOMALY_THRESHOLD {
            last = Some(detector.detect_anomaly("failed_login:server_1").await.unwrap());
        }
        let last = last.unwrap();
        assert!(last.is_anomalous);
        assert!((last.anomaly_score - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_create_incident_links_events() {
        let detector = ThreatDetector::new();
        let event = detector
            .report_event(EventType::MalwareDetected, "host_1", Severity::Critical, "malware")
            .await
            .unwrap();

        let incident = detector.create_incident(Severity::Critical, vec![event.event_id]).await.unwrap();

        assert_eq!(incident.status, IncidentStatus::Detected);
        assert!(incident.threat_score > 0.9);
        assert_eq!(incident.events, vec![event.event_id]);
    }

    #[tokio::test]
    async fn test_correlate_events_uses_real_event_data() {
        let detector = ThreatDetector::new();
        let e1 = detector
            .report_event(EventType::UnauthorizedAccess, "server_1", Severity::High, "login")
            .await
            .unwrap();
        let e2 = detector
            .report_event(EventType::DataExfiltration, "server_1", Severity::Critical, "exfil")
            .await
            .unwrap();

        let correlation = detector.correlate_events(vec![e1.event_id, e2.event_id]).await.unwrap();
        assert_eq!(correlation.event_ids.len(), 2);
        // Same source for both events -> full correlation.
        assert_eq!(correlation.correlation_score, 1.0);
        // Two distinct event types -> multi-stage pattern.
        assert_eq!(correlation.pattern, "multi_stage_attack");
    }

    #[tokio::test]
    async fn test_correlate_events_with_unknown_ids_scores_zero() {
        let detector = ThreatDetector::new();
        let correlation = detector.correlate_events(vec![Uuid::new_v4(), Uuid::new_v4()]).await.unwrap();
        assert_eq!(correlation.correlation_score, 0.0);
        assert_eq!(correlation.pattern, "no_matching_events");
    }

    #[tokio::test]
    async fn test_resolve_incident() {
        let detector = ThreatDetector::new();
        let incident = detector.create_incident(Severity::Low, vec![]).await.unwrap();

        detector.resolve_incident(incident.incident_id).await.unwrap();

        let unknown = detector.resolve_incident(Uuid::new_v4()).await;
        assert!(matches!(unknown, Err(ThreatError::IncidentNotFound)));
    }

    #[tokio::test]
    async fn test_get_threat_score_averages_anomalous_only() {
        let detector = ThreatDetector::new();
        for _ in 0..ANOMALY_THRESHOLD {
            detector.detect_anomaly("port_scan:1.2.3.4").await.unwrap();
        }
        detector.detect_anomaly("one_off_signature").await.unwrap();

        let score = detector.get_threat_score().await.unwrap();
        assert!(score > 0.0);
    }
}
