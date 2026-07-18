//! CLI for exercising the threat-detection crate.

use threat_detection::{EventType, Severity, ThreatDetector};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let detector = ThreatDetector::new();

    let e1 = detector
        .report_event(EventType::UnauthorizedAccess, "server_1", Severity::High, "Unauthorized login")
        .await?;
    let e2 = detector
        .report_event(EventType::DataExfiltration, "server_1", Severity::Critical, "Large outbound transfer")
        .await?;

    // Simulate a repeated brute-force signature to trip the frequency-based
    // anomaly detector.
    let mut last_anomaly = None;
    for _ in 0..6 {
        last_anomaly = Some(detector.detect_anomaly("failed_login:server_1").await?);
    }
    let anomaly = last_anomaly.unwrap();
    println!("Anomaly score: {:.2} (anomalous: {})", anomaly.anomaly_score, anomaly.is_anomalous);

    let correlation = detector.correlate_events(vec![e1.event_id, e2.event_id]).await?;
    println!("Correlation score: {:.2}, pattern: {}", correlation.correlation_score, correlation.pattern);

    let incident = detector.create_incident(Severity::Critical, vec![e1.event_id, e2.event_id]).await?;
    println!("Created incident {} with {} linked events, threat score {:.2}", incident.incident_id, incident.events.len(), incident.threat_score);

    detector.resolve_incident(incident.incident_id).await?;
    println!("Incident resolved");

    println!("Aggregate threat score: {:.2}", detector.get_threat_score().await?);
    Ok(())
}
