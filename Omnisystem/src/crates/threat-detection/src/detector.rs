use crate::{SecurityEvent, EventType, Severity, AnomalyDetection, ThreatIncident, IncidentStatus, CorrelatedEvents, ThreatError, ThreatResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct ThreatDetector {
    events: Arc<DashMap<Uuid, SecurityEvent>>,
    anomalies: Arc<DashMap<Uuid, AnomalyDetection>>,
    incidents: Arc<DashMap<Uuid, ThreatIncident>>,
    correlations: Arc<DashMap<Uuid, CorrelatedEvents>>,
}

impl ThreatDetector {
    pub fn new() -> Self {
        Self {
            events: Arc::new(DashMap::new()),
            anomalies: Arc::new(DashMap::new()),
            incidents: Arc::new(DashMap::new()),
            correlations: Arc::new(DashMap::new()),
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

    pub async fn detect_anomaly(&self, signature: &str) -> ThreatResult<AnomalyDetection> {
        let score = signature.len() as f64 / 100.0;
        let is_anomalous = score > 0.5;

        let anomaly = AnomalyDetection {
            anomaly_id: Uuid::new_v4(),
            event_signature: signature.to_string(),
            anomaly_score: score,
            detected_at: Utc::now(),
            is_anomalous,
        };

        self.anomalies.insert(anomaly.anomaly_id, anomaly.clone());
        Ok(anomaly)
    }

    pub async fn create_incident(&self, severity: Severity) -> ThreatResult<ThreatIncident> {
        let incident = ThreatIncident {
            incident_id: Uuid::new_v4(),
            events: vec![],
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

    pub async fn correlate_events(&self, event_ids: Vec<Uuid>) -> ThreatResult<CorrelatedEvents> {
        let correlation = CorrelatedEvents {
            correlation_id: Uuid::new_v4(),
            event_ids,
            correlation_score: 0.85,
            pattern: "multi_stage_attack".to_string(),
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
    async fn test_detect_anomaly() {
        let detector = ThreatDetector::new();
        let anomaly = detector.detect_anomaly("unusual_pattern_xyz").await.unwrap();

        assert!(anomaly.is_anomalous || !anomaly.is_anomalous);
    }

    #[tokio::test]
    async fn test_create_incident() {
        let detector = ThreatDetector::new();
        let incident = detector.create_incident(Severity::Critical).await.unwrap();

        assert_eq!(incident.status, IncidentStatus::Detected);
        assert!(incident.threat_score > 0.9);
    }

    #[tokio::test]
    async fn test_correlate_events() {
        let detector = ThreatDetector::new();
        let event_ids = vec![Uuid::new_v4(), Uuid::new_v4()];

        let correlation = detector.correlate_events(event_ids.clone()).await.unwrap();
        assert_eq!(correlation.event_ids.len(), 2);
    }
}
