//! CLI for exercising the network-policies crate.

use network_policies::{Action, IsolationLevel, NetworkPolicy, NetworkSegment, PolicyManager};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = PolicyManager::new();

    let policy = NetworkPolicy {
        policy_id: Uuid::new_v4(),
        name: "allow-frontend-to-api".to_string(),
        source: "frontend".to_string(),
        destination: "api".to_string(),
        port: 8443,
        protocol: "TCP".to_string(),
        action: Action::Allow,
    };
    manager.create_network_policy(&policy).await?;
    let allowed = manager.check_access("frontend", "api", 8443).await?;
    println!("frontend -> api:8443 allowed: {}", allowed);

    let payments = NetworkSegment { segment_id: Uuid::new_v4(), name: "payments".to_string(), cidr: "10.0.1.0/24".to_string(), isolation_level: IsolationLevel::Strict };
    let public = NetworkSegment { segment_id: Uuid::new_v4(), name: "public".to_string(), cidr: "10.0.2.0/24".to_string(), isolation_level: IsolationLevel::Low };
    manager.create_network_segment(&payments).await?;
    manager.create_network_segment(&public).await?;

    let can_talk = manager.segments_can_communicate(payments.segment_id, public.segment_id).await?;
    println!("payments (Strict) <-> public (Low) can communicate: {}", can_talk);

    println!("Total policies tracked: {}", manager.policy_count());
    Ok(())
}
