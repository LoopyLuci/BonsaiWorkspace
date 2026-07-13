mod auth;
mod gateway;
mod protocol;
mod routing;
mod telemetry;
mod transfer_client;
mod transfer_adapter;
mod webhooks;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "bonsai-api-bridge")]
#[command(about = "Bonsai API Bridge: unified protocol gateway")]
struct Args {
    #[arg(long, default_value = "0.0.0.0")]
    host: String,
    #[arg(long, default_value_t = 11429)]
    port: u16,
    #[arg(long, default_value_t = 11430)]
    grpc_port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("bonsai_api_bridge=info,tower_http=info")),
        )
        .init();

    let args = Args::parse();
    gateway::run(&args.host, args.port, args.grpc_port).await
}
