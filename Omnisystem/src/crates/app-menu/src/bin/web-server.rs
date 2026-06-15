/// Launcher Web Server Binary
///
/// Usage:
///   web-server              - Start server on 127.0.0.1:8080
///   web-server --port 9000  - Start on custom port
///
/// API Endpoints:
///   GET  http://localhost:8080/api/health
///   GET  http://localhost:8080/api/apps
///   GET  http://localhost:8080/api/apps/:id
///   GET  http://localhost:8080/api/search?q=text
///   POST http://localhost:8080/api/launch
///   GET  http://localhost:8080/api/instances
///   POST http://localhost:8080/api/instances/:id/terminate
///   GET  http://localhost:8080/api/status

use app_menu::{client::MockLauncherClient, server::LauncherServer, web::WebConfig};
use anyhow::Result;
use clap::Parser;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "web-server")]
#[command(about = "Launcher Web Server")]
struct Args {
    /// Server port
    #[arg(long, default_value = "8080")]
    port: u16,

    /// Server host/address
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();

    let config = WebConfig {
        port: args.port,
        host: args.host.clone(),
        api_base: "/api".to_string(),
    };

    let addr = format!("{}:{}", args.host, args.port);

    println!("🚀 Starting Launcher Web Server");
    println!("   Address: http://{}", addr);
    println!("   API Base: {}", config.api_base);
    println!("\n📚 API Endpoints:");
    println!("   GET  /api/health                    - Server health");
    println!("   GET  /api/apps                      - List apps");
    println!("   GET  /api/apps/:id                  - Get app details");
    println!("   GET  /api/search?q=query            - Search apps");
    println!("   POST /api/launch                    - Launch app");
    println!("   GET  /api/instances                 - List running");
    println!("   POST /api/instances/:id/terminate   - Terminate app");
    println!("   GET  /api/status                    - System status");
    println!("\n🔗 Open browser at: http://{}\n", addr);

    let client = Arc::new(MockLauncherClient::new());
    let server = LauncherServer::new(client, addr);

    server.start().await?;

    Ok(())
}
