//! CLI for exercising the traffic-management crate.

use traffic_management::{RoutingPolicy, RoutingStrategy, TrafficRouter, WeightedDestination};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = TrafficRouter::new();

    router
        .register_routing_policy(&RoutingPolicy {
            policy_id: Uuid::new_v4(),
            service_name: "checkout".to_string(),
            routing_strategy: RoutingStrategy::WeightedDistribution,
            timeout_ms: 3000,
            retries: 2,
        })
        .await?;

    router
        .add_weighted_destination(&WeightedDestination { destination_id: Uuid::new_v4(), service_name: "checkout".to_string(), version: "v1-stable".to_string(), weight: 90 })
        .await?;
    router
        .add_weighted_destination(&WeightedDestination { destination_id: Uuid::new_v4(), service_name: "checkout".to_string(), version: "v2-canary".to_string(), weight: 10 })
        .await?;

    for _ in 0..5 {
        let routed_to = router.route_request("checkout").await?;
        println!("Routed request to: {}", routed_to);
    }

    println!("Total policies registered: {}", router.policy_count());
    Ok(())
}
