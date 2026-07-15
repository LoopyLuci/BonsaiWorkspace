//! API Bridge CLI - inspects route translation and load-balancing without
//! starting the full server (see `src/main.rs` for that).

use api_bridge::protocol::rest::to_translated;
use api_bridge::routing::{load_balancer::select_best, route_to_service};

fn main() {
    let routes = [
        "/api/v1/chat/completions",
        "/api/v1/inference",
        "/api/v1/remote/peers",
        "/api/v1/file/sync",
        "/api/v1/blockchain/tx",
        "/unknown/path",
    ];

    for path in routes {
        match to_translated(path, serde_json::json!({}), "cli-trace".to_string()) {
            Some(translated) => {
                let instances = route_to_service(&translated);
                let best = select_best(&instances);
                println!(
                    "{path} -> service={} capability={} backend={:?}",
                    translated.service,
                    translated.required_capability,
                    best.map(|b| b.url)
                );
            }
            None => println!("{path} -> no route"),
        }
    }
}
