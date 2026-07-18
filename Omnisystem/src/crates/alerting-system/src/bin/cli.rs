//! CLI: register an alert rule, trip it with a metric breach, open an
//! incident, and acknowledge the alert.

use alerting_system::{AlertRule, AlertSeverity, AlertingSystem};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let system = AlertingSystem::new();

    let rule = AlertRule {
        rule_id: Uuid::new_v4(),
        metric_name: "cpu_usage".to_string(),
        threshold: 80.0,
        comparison_op: ">".to_string(),
        severity: AlertSeverity::Critical,
        enabled: true,
    };
    system.add_rule(&rule).await?;
    println!("registered rule on '{}' (threshold {})", rule.metric_name, rule.threshold);

    let alert = system.check_threshold("cpu_usage", 93.5).await?;
    match alert {
        Some(alert) => {
            println!("alert triggered: value={} severity={:?}", alert.metric_value, alert.severity);

            let incident = system.create_incident(alert.alert_id, alert.severity).await?;
            println!("incident opened: {:?}", incident.status);

            system.acknowledge_alert(alert.alert_id).await?;
            println!("alert acknowledged");
        }
        None => println!("no alert triggered"),
    }

    println!("total alerts recorded: {}", system.alert_count());

    Ok(())
}
