//! api-gateway CLI: registers a couple of routes/rules and looks them up.

use api_gateway::{ApiGateway, Route, Router};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gateway = ApiGateway::new();
    gateway
        .register_route(&Route {
            path: "/api/users".to_string(),
            target_service: "user-service".to_string(),
            methods: vec!["GET".to_string(), "POST".to_string()],
        })
        .await?;
    let route = gateway.get_route("/api/users").await?;
    println!(
        "Gateway has {} route(s); /api/users -> {}",
        gateway.route_count(),
        route.target_service
    );

    let router = Router::new();
    router.add_rule("/api/orders", "order-service").await?;
    let target = router.route("/api/orders").await?;
    println!("Router has {} rule(s); /api/orders -> {}", router.rule_count(), target);

    Ok(())
}
